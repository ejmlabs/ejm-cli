import type { AccountSession, Env } from "./types.js";
import { boundedBytes, integer, failure, jsonObject, onlyKeys, validId } from "./http.js";
import { retentionPolicy } from "./retention.js";
import { LIMITS } from "./types.js";
import { DEFAULT_RETENTION, parseConfig } from "./privacy/config.js";
import { errorPage, resultPage } from "./privacy/html.js";
import { object, text } from "./privacy/schema.js";
import { withCurrentNav } from "./ui.js";

export async function accountRoute(request: Request, env: Env, auth: AccountSession): Promise<Response | null> {
  const path = new URL(request.url).pathname;
  if (path === "/v1/account" && request.method === "GET") {
    const row = await env.DB.prepare("SELECT status,terms_version,created_at FROM relay_accounts WHERE id=?").bind(auth.accountId).first();
    const plan = await env.DB.prepare("SELECT plan,status,paid_until,cancel_at_period_end FROM relay_billing WHERE account_id=?").bind(auth.accountId).first();
    return Response.json({ id: auth.accountId, email: auth.email, termsVersion: auth.termsVersion, requiredTermsVersion: env.TERMS_VERSION, deviceId: auth.deviceId, account: row, plan: plan ?? { plan: "free", status: "free" } });
  }
  if (path === "/v1/account/terms" && request.method === "POST") {
    const body = await jsonObject(request);
    if (!body || body.version !== env.TERMS_VERSION || !onlyKeys(body, ["version"])) return failure(400, "current_terms_required");
    const now = Date.now();
    const limit = integer(env.ADMISSION_LIMIT, LIMITS.accounts, LIMITS.accounts);
    if (!retentionPolicy(env)) return failure(503, "operational_retention_not_approved");
    if (env.ADMISSION_ENABLED !== "true" || limit === 0) return failure(503, "admission_closed");
    // A single conditional SQLite write is the admission linearization point.
    await env.DB.prepare("INSERT INTO relay_accounts(id,status,terms_version,terms_at,email_provenance,created_at) SELECT ?,'active',?,?,'github_verified_primary',? WHERE (SELECT COUNT(*) FROM relay_accounts WHERE status='active') < ? OR EXISTS(SELECT 1 FROM relay_accounts WHERE id=? AND status='active') ON CONFLICT(id) DO UPDATE SET terms_version=excluded.terms_version,terms_at=excluded.terms_at WHERE relay_accounts.status='active'").bind(auth.accountId, env.TERMS_VERSION, now, now, limit, auth.accountId).run();
    const row = await env.DB.prepare("SELECT status,terms_version FROM relay_accounts WHERE id=?").bind(auth.accountId).first<{ status: string; terms_version: string }>();
    if (!row || row.status !== "active" || row.terms_version !== env.TERMS_VERSION) return failure(503, "relay_capacity_exhausted");
    return Response.json({ accepted: true, version: env.TERMS_VERSION, commercialChoices: { profile: false, usage: false } });
  }
  if (path === "/v1/devices" && request.method === "GET") {
    const result = await env.DB.prepare("SELECT id,label,created_at,last_used_at,revoked_at,os_family,cli_major FROM relay_devices WHERE account_id=? ORDER BY created_at DESC LIMIT 64").bind(auth.accountId).all();
    return Response.json({ devices: result.results });
  }
  if (path === "/v1/devices" && request.method === "POST") {
    if (auth.termsVersion !== env.TERMS_VERSION) return failure(403, "current_terms_required");
    const body = await jsonObject(request);
    if (!body || typeof body.label !== "string" || !/^[\p{L}\p{N} ._-]{1,64}$/u.test(body.label) || !onlyKeys(body, ["label", "osFamily", "cliMajor"])) return failure(400, "invalid_device_metadata");
    if (body.osFamily !== undefined && !["macos", "linux", "windows", "other"].includes(String(body.osFamily))) return failure(400, "invalid_device_metadata");
    if (body.cliMajor !== undefined && (!Number.isSafeInteger(body.cliMajor) || Number(body.cliMajor) < 0 || Number(body.cliMajor) > 1000)) return failure(400, "invalid_device_metadata");
    const existing = await env.DB.prepare("SELECT id,revoked_at FROM relay_devices WHERE account_id=? AND session_id=?").bind(auth.accountId, auth.sessionId).first<{ id: string; revoked_at: number | null }>();
    if (existing) return existing.revoked_at === null ? Response.json({ id: existing.id }) : failure(403, "device_revoked");
    const id = crypto.randomUUID();
    const now = Date.now();
    await env.DB.prepare("INSERT INTO relay_devices(id,account_id,session_id,label,created_at,last_used_at,os_family,cli_major) SELECT ?,?,?,?,?,?,?,? WHERE (SELECT COUNT(*) FROM relay_devices WHERE account_id=? AND revoked_at IS NULL)<16 ON CONFLICT(session_id) DO NOTHING").bind(id, auth.accountId, auth.sessionId, body.label, now, now, body.osFamily ?? null, body.cliMajor ?? null, auth.accountId).run();
    const registered = await env.DB.prepare("SELECT id,revoked_at FROM relay_devices WHERE account_id=? AND session_id=?").bind(auth.accountId, auth.sessionId).first<{ id: string; revoked_at: number | null }>();
    if (!registered) return failure(409, "device_limit");
    if (registered.revoked_at !== null) return failure(403, "device_revoked");
    return Response.json({ id: registered.id }, { status: registered.id === id ? 201 : 200 });
  }
  const revoke = /^\/v1\/devices\/([a-zA-Z0-9_-]{16,128})\/revoke$/.exec(path);
  if (revoke && request.method === "POST") {
    const result = await env.DB.prepare("UPDATE relay_devices SET revoked_at=? WHERE id=? AND account_id=? AND revoked_at IS NULL RETURNING session_id").bind(Date.now(), revoke[1], auth.accountId).first<{ session_id: string }>();
    if (!result) return failure(404, "device_not_found");
    await env.DB.prepare('DELETE FROM "session" WHERE id=? AND "userId"=?').bind(result.session_id, auth.accountId).run();
    return Response.json({ revoked: true, existingLeaseMaximumSeconds: 900 });
  }
  if (path === "/v1/logout" && request.method === "POST") {
    await env.DB.batch([
      env.DB.prepare("UPDATE relay_devices SET revoked_at=? WHERE session_id=? AND account_id=?").bind(Date.now(), auth.sessionId, auth.accountId),
      env.DB.prepare('DELETE FROM "session" WHERE id=? AND "userId"=?').bind(auth.sessionId, auth.accountId),
    ]);
    return Response.json({ revoked: true, existingLeaseMaximumSeconds: 900 });
  }
  if (path === "/v1/account/close" && request.method === "POST") {
    const form = request.headers.get("content-type")?.startsWith("application/x-www-form-urlencoded") ?? false;
    const bytes = form ? await boundedBytes(request) : null;
    const fields = bytes ? new URLSearchParams(new TextDecoder().decode(bytes)) : null;
    const body = form ? fields?.size === 1 ? Object.fromEntries(fields) : null : await jsonObject(request);
    if (!body || body.confirm !== "close" || !onlyKeys(body, ["confirm"])) return failure(400, "confirmation_required");
    const upstream = await env.PRIVACY_AUTHORITY.get(env.PRIVACY_AUTHORITY.idFromName("privacy-v1")).fetch("https://privacy.internal/internal/action", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ action: "rights/request", actor: { accountId: auth.accountId, browserId: null, operator: false, gpc: request.headers.get("sec-gpc") === "1" }, input: { kind: "close", jurisdiction: "unspecified" } }) });
    const result = object(await upstream.json());
    const status = upstream.ok ? await followUpRightsRequest(env, result, auth.accountId) : upstream.status;
    if (status === 200) {
      try { result.billingCancellation = await env.DB.prepare("SELECT state FROM relay_billing_cancellations WHERE account_id=?").bind(auth.accountId).first(); }
      catch { result.billingCancellation = { state: "unknown" }; }
    }
    const errorCode = result.error;
    if (typeof errorCode === "string") result.error = { code: errorCode };
    return form ? withCurrentNav(new Response(result.requestId ? resultPage(result) : errorPage(errorCode), { status, headers: { "content-type": "text/html; charset=utf-8", "cache-control": "no-store" } }), "/privacy") : Response.json(result, { status });
  }
  return null;
}

export async function queueOperationalTask(env: Env, task: Record<string, unknown>): Promise<void> {
  const days = (parseConfig(env.COMMERCIAL_CONFIG)?.retention ?? DEFAULT_RETENTION).operationalTaskDays;
  await env.DB.prepare("INSERT OR IGNORE INTO privacy_operational_tasks(request_id,account_id,action,state,requested_at,due_at,expires_at) VALUES(?,?,?,'pending',?,?,?)").bind(text(task.requestId), text(task.accountId), text(task.action), task.requestedAt, task.dueAt, typeof task.expiresAt === "number" ? task.expiresAt : Number(task.requestedAt) + days * 86_400_000).run();
}

export async function followUpRightsRequest(env: Env, result: Record<string, unknown>, accountId: string): Promise<number> {
  // The authority owns the pending task before relay credentials are revoked.
  try { await queueOperationalTask(env, object(result.operationalTask)); }
  catch { result.operationalQueuePending = true; }
  if (result.kind === "close") {
    try { await revokeRelayAccount(env, accountId, true); result.closed = true; result.relayClosureAcknowledged = true; }
    catch { result.error = "relay_revocation_pending"; result.relayClosureAcknowledged = false; return 503; }
  }
  return 200;
}

export async function revokeRelayAccount(env: Env, accountId: string, privacyAlreadyClosed = false): Promise<void> {
  if (!privacyAlreadyClosed) {
    const privacy = env.PRIVACY_AUTHORITY.get(env.PRIVACY_AUTHORITY.idFromName("privacy-v1"));
    const closed = await privacy.fetch("https://internal/internal/close", { method: "POST", body: JSON.stringify({ accountId }) });
    await closed.text();
    if (!closed.ok) throw new Error("privacy_authority_unavailable");
  }
  const now = Date.now();
  await env.DB.batch([
    env.DB.prepare("UPDATE relay_accounts SET status='closed',closed_at=COALESCE(closed_at,?),cleanup_after=COALESCE(cleanup_after,?) WHERE id=?").bind(now, now, accountId),
    env.DB.prepare("UPDATE relay_devices SET revoked_at=? WHERE account_id=?").bind(now, accountId),
    env.DB.prepare('DELETE FROM "session" WHERE "userId"=?').bind(accountId),
  ]);
  await env.DB.prepare("INSERT INTO relay_billing_cancellations(account_id,state,created_at,updated_at) SELECT account_id,'pending',?,? FROM relay_billing WHERE account_id=? ON CONFLICT(account_id) DO UPDATE SET state='pending',updated_at=excluded.updated_at").bind(now, now, accountId).run();
  const response = await env.ACCOUNT_QUOTA.get(env.ACCOUNT_QUOTA.idFromName(accountId)).fetch("https://internal/revoke", { method: "POST", body: JSON.stringify({ accountId }) });
  if (!response.ok) { await response.text(); throw new Error("quota_authority_unavailable"); }
  const current = await response.json<{ room?: string; expiresAt?: number }>();
  if (current.room && validId(current.room)) {
    const room = await env.ROOM.get(env.ROOM.idFromName(current.room)).fetch("https://internal/close", { method: "POST", body: JSON.stringify({ accountId, expiresAt: current.expiresAt }) });
    await room.text();
    if (!room.ok) throw new Error("room_revocation_unavailable");
  }
  const cancel = await env.ACCOUNT_QUOTA.get(env.ACCOUNT_QUOTA.idFromName(accountId)).fetch("https://internal/billing/cancel", { method: "POST", body: JSON.stringify({ accountId }) });
  await cancel.text();
}
