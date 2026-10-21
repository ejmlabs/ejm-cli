import { DurableObject } from "cloudflare:workers";
import { parseCookies, serializeCookie } from "better-call";
import type { AccountSession, Env } from "../types.js";
import { withCurrentNav } from "../ui.js";
import { followUpRightsRequest, queueOperationalTask } from "../accounts.js";
import { parseConfig } from "./config.js";
import { accountPage, optOutPage, errorPage, resultPage, rightsStatusPage, signInRequiredPage } from "./html.js";
import { exactKeys, object, PrivacyError, PROFILE_FIELDS, text } from "./schema.js";
import { PrivacyService, type Actor, type OperationalTaskCursor } from "./service.js";
import type { Binding, SqlStore } from "./store.js";

class CloudflareStore implements SqlStore {
  private depth = 0;
  constructor(private readonly storage: DurableObjectStorage) {}
  exec<T extends Record<string, unknown>>(query: string, ...bindings: Binding[]): T[] { return this.storage.sql.exec(query, ...bindings).toArray() as T[]; }
  transaction<T>(callback: () => T): T {
    if (this.depth > 0) return callback();
    return this.storage.transactionSync(() => { this.depth += 1; try { return callback(); } finally { this.depth -= 1; } });
  }
}

export class PrivacyAuthority extends DurableObject<Env> {
  private readonly service: PrivacyService;
  constructor(state: DurableObjectState, env: Env) { super(state, env); this.service = new PrivacyService(new CloudflareStore(state.storage), parseConfig(env.COMMERCIAL_CONFIG)); }
  override async fetch(request: Request): Promise<Response> {
    try {
      const path = new URL(request.url).pathname;
      if (request.method !== "POST") return json({ error: "method_not_allowed" }, 405);
      const message = object(await boundedJson(request));
      if (path === "/internal/close") return json(this.service.close(text(message.accountId)));
      if (path === "/internal/usage") return json(this.service.recordUsage(message));
      if (path === "/internal/tasks") {
        const tasks = this.service.operationalTasks(await this.ctx.storage.get<OperationalTaskCursor>("operational-task-cursor"));
        const last = tasks.at(-1);
        if (last) await this.ctx.storage.put("operational-task-cursor", { requestedAt: last.requestedAt, requestId: last.requestId });
        else await this.ctx.storage.delete("operational-task-cursor");
        return json(tasks);
      }
      if (path === "/internal/operational") return json(this.service.operationalUpdate(message));
      if (path === "/internal/cleanup") return json(this.service.cleanup());
      if (path !== "/internal/action") return json({ error: "not_found" }, 404);
      const actor = message.actor as Actor;
      if (!actor || typeof actor.operator !== "boolean" || typeof actor.gpc !== "boolean") throw new PrivacyError("invalid_internal_actor");
      const action = text(message.action);
      if (actor.gpc || (message.applyBrowserOptout === true && actor.accountId !== null)) this.service.optOut(actor);
      const result = await dispatch(this.service, action, actor, message.input ?? {});
      return json(result);
    } catch (error) { return failure(error); }
  }
}

async function dispatch(service: PrivacyService, action: string, actor: Actor, input: unknown): Promise<unknown> {
  switch (action) {
    case "status": return service.status(actor);
    case "adult": return service.adult(actor, input);
    case "consent": return service.acceptConsent(actor, input);
    case "profile": return service.profile(actor, input);
    case "withdraw": return service.withdraw(actor, input);
    case "optout": return service.optOut(actor);
    case "rejoin": return service.rejoin(actor, input);
    case "access": return service.access(actor);
    case "evidence": return service.evidencePage(actor, input);
    case "rights": return service.rightsList(actor);
    case "rights/request": return service.rightsRequest(actor, input);
    case "rights/status": return service.rightsStatus(input);
    case "operator/prepare": return service.prepare(actor, input);
    case "operator/release": return service.release(actor, input);
    case "operator/cohort": return service.cohort(actor, input);
    case "operator/rights": return service.rightsUpdate(actor, input);
    default: throw new PrivacyError("not_found", 404);
  }
}

const UUID = /^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/u;
function cookieName(env: Env): string { return env.PRIVACY_COOKIE_SECURE === "false" ? "ejm_privacy_local" : "__Host-ejm_privacy"; }
function browserId(request: Request, env: Env): string | null {
  const value = parseCookies(request.headers.get("cookie") ?? "").get(cookieName(env));
  return value && UUID.test(value) ? value : null;
}
function cookie(env: Env, id: string, clear = false): string {
  return serializeCookie(cookieName(env), clear ? "" : id, { path: "/", httpOnly: true, sameSite: "lax", maxAge: clear ? 0 : 400 * 86_400, secure: env.PRIVACY_COOKIE_SECURE !== "false" });
}
function actorFor(request: Request, env: Env, account: AccountSession | null, id: string | null): Actor {
  return { accountId: account?.accountId ?? null, browserId: id, gpc: request.headers.get("sec-gpc") === "1", operator: account !== null && (env.PRIVACY_OPERATOR_ACCOUNT_IDS ?? "").split(",").map((entry) => entry.trim()).filter(Boolean).includes(account.accountId) };
}
async function call(env: Env, action: string, actor: Actor, input: unknown = {}, applyBrowserOptout = false): Promise<Response> {
  const stub = env.PRIVACY_AUTHORITY.get(env.PRIVACY_AUTHORITY.idFromName("privacy-v1"));
  return stub.fetch("https://privacy.internal/internal/action", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ action, actor, input, applyBrowserOptout }) });
}

/** Call on all browser requests, then append returned headers to the service response. */
export async function privacySignal(request: Request, env: Env, account: AccountSession | null): Promise<Headers> {
  const headers = new Headers(); const previous = browserId(request, env);
  if (request.headers.get("sec-gpc") !== "1" && previous === null) return headers;
  const id = previous ?? crypto.randomUUID(); const response = await call(env, "optout", actorFor(request, env, account, id));
  await response.arrayBuffer();
  if (!response.ok) throw new PrivacyError("optout_temporarily_unavailable", 503);
  headers.append("set-cookie", cookie(env, id)); return headers;
}

export async function privacyRoute(request: Request, env: Env, account: AccountSession | null): Promise<Response | null> {
  const path = new URL(request.url).pathname;
  if (path !== "/privacy" && path !== "/privacy/rights" && path !== "/do-not-sell-or-share" && path !== "/api/privacy" && !path.startsWith("/api/privacy/")) return null;
  try {
    const previous = browserId(request, env); const gpc = request.headers.get("sec-gpc") === "1";
    const id = previous ?? (gpc || path === "/do-not-sell-or-share" && request.method === "POST" ? crypto.randomUUID() : null);
    const actor = actorFor(request, env, account, id); const headers = privateHeaders();
    if (path === "/privacy/rights" && request.method === "GET") { headers.set("content-type", "text/html; charset=utf-8"); return withCurrentNav(new Response(rightsStatusPage(), { headers }), "/privacy"); }
    if (gpc || previous !== null && account !== null) { const signal = await call(env, "optout", actor); if (!signal.ok) return signal; await signal.arrayBuffer(); if (id) headers.append("set-cookie", cookie(env, id)); }
    if (path === "/do-not-sell-or-share") {
      if (request.method === "POST") { const response = await call(env, "optout", actor); if (!response.ok) return response; await response.arrayBuffer(); if (id) headers.append("set-cookie", cookie(env, id)); }
      else if (request.method !== "GET") return json({ error: "method_not_allowed" }, 405);
      headers.set("content-type", "text/html; charset=utf-8"); return withCurrentNav(new Response(optOutPage(gpc || previous !== null || request.method === "POST"), { headers }), "/privacy");
    }
    const statusRead = path === "/api/privacy/rights/status";
    if (account === null && !statusRead) {
      if (path === "/privacy" && request.method === "GET" && request.headers.get("accept")?.includes("text/html")) {
        headers.set("content-type", "text/html; charset=utf-8");
        return withCurrentNav(new Response(signInRequiredPage(), { status: 401, headers }), "/privacy");
      }
      throw new PrivacyError("authentication_required", 401);
    }
    const action = path === "/privacy" || path === "/api/privacy" ? "status" : path.slice("/api/privacy/".length);
    const readOnly = ["status", "access", "evidence", "rights"].includes(action);
    if (request.method !== (readOnly ? "GET" : "POST")) return json({ error: "method_not_allowed" }, 405);
    if (!readOnly && request.headers.get("origin") !== new URL(request.url).origin) throw new PrivacyError("same_origin_required", 403);
    const form = request.headers.get("content-type")?.startsWith("application/x-www-form-urlencoded") ?? false;
    const input = readOnly ? action === "evidence" ? Object.fromEntries(new URL(request.url).searchParams) : {} : form ? await formInput(request, action) : await boundedJson(request);
    const upstream = await call(env, action, actor, input);
    const result = await upstream.json() as Record<string, unknown>;
    let status = upstream.status;
    if (upstream.ok && action === "rights/request" && result.operationalTask) {
      status = await followUpRightsRequest(env, result, account!.accountId);
    }
    if (upstream.ok && action === "rejoin" && result.rejoined === true) headers.append("set-cookie", cookie(env, id ?? "", true));
    if (path === "/privacy") { headers.set("content-type", "text/html; charset=utf-8"); return withCurrentNav(new Response(upstream.ok ? accountPage(result, gpc) : errorPage(result.error), { status: upstream.status, headers }), "/privacy"); }
    if (form) { headers.set("content-type", "text/html; charset=utf-8"); return withCurrentNav(new Response(upstream.ok ? resultPage(result) : errorPage(result.error), { status, headers }), "/privacy"); }
    headers.set("content-type", "application/json");
    if (action === "access") headers.set("content-disposition", 'attachment; filename="ejm-privacy-data.json"');
    return new Response(JSON.stringify(result), { status, headers });
  } catch (error) {
    if (request.headers.get("content-type")?.startsWith("application/x-www-form-urlencoded")) {
      const headers = privateHeaders(); headers.set("content-type", "text/html; charset=utf-8");
      return withCurrentNav(new Response(errorPage(error instanceof PrivacyError ? error.code : "privacy_operation_failed"), { status: error instanceof PrivacyError ? error.status : 500, headers }), "/privacy");
    }
    return failure(error);
  }
}

async function formInput(request: Request, action: string): Promise<unknown> {
  const form = new URLSearchParams(await boundedText(request));
  const entries = Object.fromEntries(form);
  if (new Set(form.keys()).size !== [...form.keys()].length) throw new PrivacyError("duplicate_form_fields");
  if (action === "profile") { exactKeys(entries, ["provenance", ...PROFILE_FIELDS]); const fields = Object.fromEntries(Object.entries(entries).filter(([key, value]) => key !== "provenance" && value !== "")); return { provenance: entries.provenance, fields }; }
  if (action === "consent") return { ...entries, accepted: entries.accepted === "true", fields: JSON.parse(entries.fields ?? "null") as unknown };
  if (action === "adult") return { ...entries, adult: entries.adult === "true" };
  if (action === "rejoin") return entries.confirmed === undefined ? entries : { ...entries, confirmed: entries.confirmed === "true" };
  if (action === "rights/status") {
    exactKeys(entries, ["requestId", "statusKey", "part"]);
    const { part, ...identity } = entries;
    if (part === undefined || part === "") return identity;
    if (!/^\d+$/u.test(part) || !Number.isSafeInteger(Number(part))) throw new PrivacyError("invalid_part");
    return { ...identity, part: Number(part) };
  }
  return entries;
}
async function boundedText(request: Request): Promise<string> {
  if (!request.body) return "";
  const reader = request.body.getReader(); const chunks: Uint8Array[] = []; let size = 0;
  while (true) { const value = await reader.read(); if (value.done) break; size += value.value.byteLength; if (size > 65_536) { await reader.cancel(); throw new PrivacyError("body_too_large", 413); } chunks.push(value.value); }
  const bytes = new Uint8Array(size); let offset = 0; for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.byteLength; } return new TextDecoder("utf-8", { fatal: true, ignoreBOM: false }).decode(bytes);
}
async function boundedJson(request: Request): Promise<unknown> { try { return JSON.parse(await boundedText(request)) as unknown; } catch (error) { if (error instanceof PrivacyError) throw error; throw new PrivacyError("invalid_json"); } }
function privateHeaders(): Headers { return new Headers({ "cache-control": "no-store", "x-content-type-options": "nosniff", "referrer-policy": "same-origin", "content-security-policy": "default-src 'self'; script-src 'self'; style-src 'self'; font-src 'self'; object-src 'none'; form-action 'self'; frame-ancestors 'none'; base-uri 'none'" }); }
function json(value: unknown, status = 200): Response { const headers = privateHeaders(); headers.set("content-type", "application/json"); return new Response(JSON.stringify(value), { status, headers }); }
function failure(error: unknown): Response { return json({ error: error instanceof PrivacyError ? error.code : "privacy_operation_failed" }, error instanceof PrivacyError ? error.status : 500); }

export async function privacyCleanup(env: Env): Promise<void> {
  const stub = env.PRIVACY_AUTHORITY.get(env.PRIVACY_AUTHORITY.idFromName("privacy-v1"));
  const response = await stub.fetch("https://privacy.internal/internal/cleanup", { method: "POST", headers: { "content-type": "application/json" }, body: "{}" });
  await response.arrayBuffer();
  if (!response.ok) throw new PrivacyError("privacy_cleanup_failed", 503);
  const tasksResponse = await stub.fetch("https://privacy.internal/internal/tasks", { method: "POST", headers: { "content-type": "application/json" }, body: "{}" });
  if (!tasksResponse.ok) { await tasksResponse.arrayBuffer(); throw new PrivacyError("privacy_tasks_unavailable", 503); }
  for (const task of await tasksResponse.json() as Record<string, unknown>[]) {
    try { await queueOperationalTask(env, task); }
    catch { /* The authority keeps this request pending and revisits it after the cursor wraps. */ }
  }
  await env.DB.prepare("DELETE FROM privacy_operational_tasks WHERE request_id IN (SELECT request_id FROM privacy_operational_tasks WHERE expires_at<=? AND state IN ('completed','exception') ORDER BY expires_at LIMIT 128)").bind(Date.now()).run();
}
