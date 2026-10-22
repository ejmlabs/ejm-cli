import { env } from "cloudflare:workers";
import { serializeSignedCookie } from "better-call";
import { applyD1Migrations, evictDurableObject, reset, runDurableObjectAlarm, runInDurableObject } from "cloudflare:test";
import { beforeEach, describe, expect, it, vi } from "vitest";
import worker from "../src/index.js";
import { createAuth, authOptions } from "../src/auth.js";
import { revokeRelayAccount } from "../src/accounts.js";
import { operationalCleanup } from "../src/cleanup.js";
import { credentialHeaders, hashToken, mutationAllowed } from "../src/http.js";
import { AccountQuota, leaseExpiry, windowBoundary } from "../src/quota.js";
import { Room } from "../src/rooms.js";
import type { PrivacyAuthority } from "../src/privacy/index.js";
import { subscriptionEntitlement } from "../src/billing.js";
import { retentionCleanup, retentionPolicy } from "../src/retention.js";
import type { Env, Lease } from "../src/types.js";
import { LIMITS } from "../src/types.js";
import { githubFixtureFetch, githubIdentityRedirectTarget } from "./fixtures/github.js";

const bindings = env as unknown as Env & { TEST_MIGRATIONS: { name: string; queries: string[] }[] };
const origin = "https://relay.example.invalid";
let counter = 0;
function call(path: string, method = "GET", body?: unknown, token?: string, headers?: HeadersInit, override: Env = bindings): Promise<Response> {
  const h = new Headers(headers);
  if (token) h.set("authorization", `Bearer ${token}`);
  if (body !== undefined) h.set("content-type", "application/json");
  return worker.fetch(new Request(`${origin}${path}`, { method, headers: h, ...(body === undefined ? {} : { body: JSON.stringify(body) }) }), override);
}
async function identity(verified = true): Promise<{ accountId: string; token: string; sessionId: string; deviceId: string }> {
  const context = await createAuth(bindings).$context;
  const user = await context.internalAdapter.createUser({ email: `fixture-${++counter}@example.invalid`, name: "Fixture", emailVerified: verified }, { method: "api" });
  const session = await context.internalAdapter.createSession(user.id);
  if (!verified) return { accountId: user.id, token: session.token, sessionId: session.id, deviceId: "" };
  const terms = await call("/v1/account/terms", "POST", { version: bindings.TERMS_VERSION }, session.token);
  expect(terms.status).toBe(200);
  const device = await call("/v1/devices", "POST", { label: "Fixture device", osFamily: "linux", cliMajor: 1 }, session.token);
  expect(device.status).toBe(201);
  const registered = await device.json<{ id: string }>();
  return { accountId: user.id, token: session.token, sessionId: session.id, deviceId: registered.id };
}
async function cookie(owner: Awaited<ReturnType<typeof identity>>): Promise<string> {
  const context = await createAuth(bindings).$context;
  return (await serializeSignedCookie(context.authCookies.sessionToken.name, owner.token, context.secret)).split(";")[0]!;
}
async function ticket(owner: Awaited<ReturnType<typeof identity>>, room: string, role = "host"): Promise<string> {
  const response = await call(`/v1/rooms/${room}/tickets`, "POST", { role, deviceId: owner.deviceId }, owner.token);
  expect(response.status).toBe(200);
  return (await response.json<{ ticket: string }>()).ticket;
}
async function connect(room: string, role: string, token: string): Promise<Response> { return call(`/v1/rooms/${room}/${role}`, "GET", undefined, token, { upgrade: "websocket" }); }
async function message(socket: WebSocket): Promise<ArrayBuffer | string> { return new Promise(resolve => socket.addEventListener("message", event => resolve(event.data as ArrayBuffer | string), { once: true })); }
function afterDatabaseRead(database: D1Database, completed: () => void): D1Database {
  function statement(raw: D1PreparedStatement): D1PreparedStatement {
    return new Proxy(raw, { get(target, key) {
      if (key === "bind") return (...args: unknown[]) => statement(target.bind(...args));
      const value: unknown = Reflect.get(target, key);
      if (key === "first") return async (...args: unknown[]) => {
        const result: unknown = await Reflect.apply(value as (...args: unknown[]) => Promise<unknown>, target, args);
        completed(); return result;
      };
      return typeof value === "function" ? value.bind(target) : value;
    } });
  }
  return new Proxy(database, { get(target, key) {
    if (key === "prepare") return (sql: string) => statement(target.prepare(sql));
    const value: unknown = Reflect.get(target, key); return typeof value === "function" ? value.bind(target) : value;
  } });
}
function afterQuotaFetch(namespace: DurableObjectNamespace, completed: (lease: Lease) => void): DurableObjectNamespace {
  return new Proxy(namespace, { get(target, key) {
    if (key === "get") return (...args: Parameters<typeof target.get>) => {
      const stub = target.get(...args);
      return new Proxy(stub, { get(target, key) {
        if (key === "fetch") return async (...args: Parameters<typeof target.fetch>) => {
          const response = await target.fetch(...args);
          if (response.ok) completed(await response.clone().json<Lease>());
          return response;
        };
        const value: unknown = Reflect.get(target, key); return typeof value === "function" ? value.bind(target) : value;
      } });
    };
    const value: unknown = Reflect.get(target, key); return typeof value === "function" ? value.bind(target) : value;
  } });
}
beforeEach(async () => { vi.restoreAllMocks(); await reset(); await applyD1Migrations(bindings.DB, bindings.TEST_MIGRATIONS); });

describe("account and device boundary", () => {
  it("requires verified identity, terms and explicit admission with sales off", async () => {
    expect((await call("/v1/account")).status).toBe(401);
    const unverified = await identity(false);
    expect((await call("/v1/account", "GET", undefined, unverified.token)).status).toBe(401);
    const user = await identity();
    expect(await (await call("/v1/account/terms", "POST", { version: bindings.TERMS_VERSION }, user.token)).json()).toMatchObject({ commercialChoices: { profile: false, usage: false } });
    expect((await call("/v1/account/terms", "POST", { version: "old" }, user.token)).status).toBe(400);
    expect((await call("/v1/account/terms", "POST", { version: bindings.TERMS_VERSION }, user.token, undefined, { ...bindings, ADMISSION_ENABLED: "false" })).status).toBe(503);
    const record = await bindings.DB.prepare("SELECT email_provenance FROM relay_accounts WHERE id=?").bind(user.accountId).first();
    expect(record).toEqual({ email_provenance: "github_verified_primary" });
    expect((await call("/v1/account/terms", "POST", { version: bindings.TERMS_VERSION }, user.token, undefined, { ...bindings, RETENTION_CONFIG: "{}" })).status).toBe(503);
    const context = await createAuth(bindings).$context;
    await context.internalAdapter.createAccount({ userId: user.accountId, providerId: "github", accountId: "12345", accessToken: "SENTINEL_provider_access", refreshToken: "SENTINEL_provider_refresh", idToken: "SENTINEL_provider_identity" });
    const provider = await bindings.DB.prepare('SELECT "accountId","accessToken","refreshToken","idToken" FROM "account" WHERE "userId"=?').bind(user.accountId).first();
    expect(provider).toEqual({ accountId: "12345", accessToken: null, refreshToken: null, idToken: null });
  });
  it("publishes only validated operator notices with distinct versions and escaped metadata", async () => {
    const notice = 'https://operator.example.invalid/notice?version=" data-injected="true&revision=2&literal=&quot;&numeric=&#34;';
    const policy = { ...JSON.parse(bindings.RETENTION_CONFIG!), version: "policy-<v2>", approvedBy: 'Operator </p><script>alert("fixture")</script>', externalNotice: notice };
    const configured = { ...bindings, TERMS_VERSION: "service-<v3>", RETENTION_CONFIG: JSON.stringify(policy), ADMISSION_ENABLED: "false" };
    async function rendered(override: Env): Promise<{ html: string; text: string; links: (string | null)[]; scripts: (string | null)[]; injected: number }> {
      const response = await call("/terms", "GET", undefined, undefined, undefined, override);
      expect(response.status).toBe(200); expect(response.headers.get("cache-control")).toBe("no-store");
      const links: (string | null)[] = []; const scripts: (string | null)[] = []; let injected = 0; let text = "";
      const parsed = new HTMLRewriter().on("a", { element: element => { links.push(element.getAttribute("href")); } })
        .on("body", { text: chunk => { text += chunk.text; } })
        .on("script", { element: element => { scripts.push(element.getAttribute("src")); } })
        .on("[data-injected]", { element: () => { injected += 1; } }).transform(response);
      const html = await parsed.text(); return { html, text, links, scripts, injected };
    }
    const approved = await rendered(configured);
    // HTMLRewriter returns the attribute source with character references intact.
    const encodedNotice = "https://operator.example.invalid/notice?version=&quot; data-injected=&quot;true&amp;revision=2&amp;literal=&amp;quot;&amp;numeric=&amp;#34;";
    expect(approved.links).toContain(encodedNotice);
    expect(approved.text).toContain("Service terms version "); expect(approved.html).toContain("service-&lt;v3&gt;");
    expect(approved.text).toContain("Operational policy version "); expect(approved.html).toContain("policy-&lt;v2&gt;");
    expect(approved.scripts).toEqual(["/common/theme-boot.js", "/account.js"]); expect(approved.injected).toBe(0);
    expect(approved.html).not.toContain("This preview is not an approved launch notice");
    for (const value of [{}, { ...policy, approved: false }, { ...policy, externalNotice: "javascript:alert(1)" }]) {
      const preview = await rendered({ ...configured, RETENTION_CONFIG: JSON.stringify(value) });
      expect(preview.links).not.toContain(encodedNotice);
      expect(preview.links.some(link => link?.startsWith("javascript:"))).toBe(false);
      expect(preview.html).toContain("This preview is not an approved launch notice");
      expect(preview.html).not.toContain("Operational policy version");
    }
    const owner = await identity();
    expect(await (await call("/v1/account/terms", "POST", { version: configured.TERMS_VERSION }, owner.token, undefined, configured)).json()).toMatchObject({ error: { code: "admission_closed" } });
  });
  it("shows paid actions only when payment capabilities are configured", async () => {
    const owner = await identity();
    const disabled = { ...bindings, STRIPE_SECRET_KEY: "" };
    const free = await call("/account", "GET", undefined, owner.token, undefined, disabled);
    const html = await free.text();
    expect(html).toContain("Free beta"); expect(html).toContain('id="close-account"');
    expect(html).not.toContain("data-plan="); expect(html).not.toContain('id="billing-portal"');
    const paid = await (await call("/account", "GET", undefined, owner.token)).text();
    expect(paid).toContain('data-plan="monthly"'); expect(paid).toContain('id="billing-portal"');
  });
  it("keeps GitHub scope minimal and removes provider profile enrichment", () => {
    const options = authOptions(bindings, bindings.DB);
    expect(options.socialProviders?.github).toMatchObject({ disableDefaultScope: true, scope: ["user:email"] });
    expect(options.advanced?.ipAddress).toMatchObject({ ipAddressHeaders: [] });
    expect(options.logger).toMatchObject({ disabled: true });
  });
  it("enforces an atomic admission cap and lets an existing account accept updated terms", async () => {
    const first = await identity();
    const context = await createAuth(bindings).$context;
    const user = await context.internalAdapter.createUser({ email: "second@example.invalid", name: "Second", emailVerified: true }, { method: "api" });
    const session = await context.internalAdapter.createSession(user.id);
    const capped = { ...bindings, ADMISSION_LIMIT: "1" };
    expect((await call("/v1/account/terms", "POST", { version: bindings.TERMS_VERSION }, session.token, undefined, capped)).status).toBe(503);
    expect((await call("/v1/account/terms", "POST", { version: bindings.TERMS_VERSION }, first.token, undefined, capped)).status).toBe(200);
  });
  it("rejects cross-account device revocation, unsafe metadata and CSRF", async () => {
    const owner = await identity(); const other = await identity();
    expect((await call(`/v1/devices/${owner.deviceId}/revoke`, "POST", {}, other.token)).status).toBe(404);
    expect((await call("/v1/devices", "POST", { label: "\u001b[31msecret" }, owner.token)).status).toBe(400);
    expect((await call("/v1/logout", "POST", {}, owner.token, { origin: "https://attacker.invalid" })).status).toBe(403);
    expect(credentialHeaders(new Headers({ authorization: "Basic fake", cookie: "fallback=true" }))).toBeNull();
    expect(credentialHeaders(new Headers({ authorization: "Bearer safe", cookie: "fallback=true" }))?.has("cookie")).toBe(false);
    expect(mutationAllowed(new Request(origin, { method: "POST", headers: { cookie: "session=true" } }), origin)).toBe(false);
  });
  it("keeps all active devices revocable after newer revoked history", async () => {
    const owner = await identity(); const other = await identity();
    const context = await createAuth(bindings).$context;
    const activeIds = [owner.deviceId];
    for (let index = 1; index < 16; index += 1) {
      const session = await context.internalAdapter.createSession(owner.accountId);
      const response = await call("/v1/devices", "POST", { label: `Active fixture ${index}` }, session.token);
      expect(response.status).toBe(201); activeIds.push((await response.json<{ id: string }>()).id);
    }
    const browser = await context.internalAdapter.createSession(owner.accountId);
    const overLimit = await call("/v1/devices", "POST", { label: "Over limit" }, browser.token);
    expect(overLimit.status).toBe(409); expect(await overLimit.json()).toMatchObject({ error: { code: "device_limit" } });
    const now = Date.now();
    await bindings.DB.prepare("UPDATE relay_devices SET created_at=? WHERE account_id=?").bind(now - 100_000, owner.accountId).run();
    await bindings.DB.prepare("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x<65) INSERT INTO relay_devices(id,account_id,session_id,label,created_at,last_used_at,revoked_at) SELECT printf('fixture_revoked_%016d',x),?,printf('fixture_revoked_session_%016d',x),'Revoked fixture',?,?,? FROM n").bind(owner.accountId, now - 1000, now - 1000, now).run();
    const browserCookie = await cookie({ ...owner, token: browser.token, sessionId: browser.id, deviceId: "" });
    const response = await call("/v1/devices", "GET", undefined, undefined, { cookie: browserCookie });
    expect(response.status).toBe(200);
    const listed = (await response.json<{ devices: { id: string; created_at: number; revoked_at: number | null; session_id?: string }[] }>()).devices;
    expect(listed).toHaveLength(64);
    expect(listed.filter(device => device.revoked_at === null).map(device => device.id).sort()).toEqual([...activeIds].sort());
    expect(listed.slice(0, 16).every(device => device.revoked_at === null)).toBe(true);
    expect(listed.slice(16).every(device => device.revoked_at !== null)).toBe(true);
    expect(listed.every(device => device.id !== other.deviceId && device.session_id === undefined)).toBe(true);
    expect(new Set(listed.map(device => device.id)).size).toBe(64);
    for (const group of [listed.slice(0, 16), listed.slice(16)]) expect(group.map(device => device.created_at)).toEqual(group.map(device => device.created_at).sort((left, right) => right - left));
    expect((await call(`/v1/devices/${owner.deviceId}/revoke`, "POST", {}, undefined, { cookie: browserCookie, origin: "https://attacker.invalid" })).status).toBe(403);
    expect((await call("/v1/account", "GET", undefined, owner.token)).status).toBe(200);
    expect((await call(`/v1/devices/${owner.deviceId}/revoke`, "POST", {}, undefined, { cookie: browserCookie, origin })).status).toBe(200);
    expect((await call("/v1/account", "GET", undefined, owner.token)).status).toBe(401);
    const remaining = await call("/v1/account", "GET", undefined, undefined, { cookie: browserCookie });
    expect(remaining.status).toBe(200); expect(await remaining.json()).toMatchObject({ deviceId: null });
  });
  it("requires a fresh local pairing after account recovery and prevents revoked sessions", async () => {
    const owner = await identity();
    await bindings.DB.prepare(`CREATE TRIGGER fixture_device_session_delete_failure BEFORE DELETE ON "session" BEGIN SELECT RAISE(ABORT,'Offline fixture session deletion failure'); END`).run();
    expect((await call(`/v1/devices/${owner.deviceId}/revoke`, "POST", {}, owner.token)).status).toBe(503);
    expect(await bindings.DB.prepare("SELECT revoked_at FROM relay_devices WHERE id=? AND account_id=?").bind(owner.deviceId, owner.accountId).first()).toEqual({ revoked_at: null });
    expect(await bindings.DB.prepare('SELECT id FROM "session" WHERE id=? AND "userId"=?').bind(owner.sessionId, owner.accountId).first()).toEqual({ id: owner.sessionId });
    expect((await call("/v1/account", "GET", undefined, owner.token)).status).toBe(200);
    await bindings.DB.prepare("DROP TRIGGER fixture_device_session_delete_failure").run();
    expect((await call(`/v1/devices/${owner.deviceId}/revoke`, "POST", {}, owner.token)).status).toBe(200);
    expect((await call("/v1/account", "GET", undefined, owner.token)).status).toBe(401);
    // The service stores no endpoint public/private keys or invite secret.
    const tables = await bindings.DB.prepare("SELECT sql FROM sqlite_master WHERE type='table'").all<{ sql: string }>();
    expect(tables.results.map(row => row.sql).join("\n")).not.toMatch(/private_key|invitation_psk|stream_key|transcript|ciphertext/i);
  });
});

describe("OAuth admission boundary", () => {
  function mockGithub(subject = "54321", scenario = "success") {
    return vi.spyOn(globalThis, "fetch").mockImplementation(githubFixtureFetch(subject, scenario));
  }
  async function start(config: Env): Promise<{ state: string; cookies: string }> {
    const response = await call("/api/auth/sign-in/social", "POST", { provider: "github", callbackURL: `${origin}/account` }, undefined, { origin }, config);
    expect(response.status).toBe(200);
    const authorize = new URL((await response.json<{ url: string }>()).url);
    expect(authorize.origin).toBe("https://github.com");
    expect(authorize.pathname).toBe("/login/oauth/authorize");
    expect(authorize.searchParams.get("state")).toBeTruthy();
    const cookies = response.headers.getSetCookie().map(value => value.split(";")[0]!).join("; ");
    expect(cookies).not.toBe("");
    return { state: authorize.searchParams.get("state")!, cookies };
  }
  async function complete(config: Env, pending: Awaited<ReturnType<typeof start>>): Promise<Response> {
    const query = new URLSearchParams({ code: "fixture-provider-code", state: pending.state });
    const response = await call(`/api/auth/callback/github?${query}`, "GET", undefined, undefined, { cookie: pending.cookies }, config);
    await response.text();
    return response;
  }
  async function follow(response: Response, config: Env): Promise<Response> {
    let current = response;
    for (let hop = 0; current.status === 302 && hop < 3; hop += 1) {
      const target = new URL(current.headers.get("location")!, origin);
      expect(target.origin).toBe(origin);
      current = await call(target.pathname + target.search, "GET", undefined, undefined, undefined, config);
    }
    return current;
  }
  it.each(["success", "profile-unavailable", "profile-read-failure"])("renders the complete closed-signup callback chain without a download: %s", async scenario => {
    mockGithub("54321", scenario); const config = { ...bindings, ADMISSION_ENABLED: "false" };
    const landing = await follow(await complete(config, await start(config)), config);
    expect(landing.status).toBe(200);
    expect(landing.headers.get("content-type")).toMatch(/^text\/html(?:;|$)/);
    expect(landing.headers.get("content-disposition")).toBeNull();
    const html = await landing.text(); expect(html).toContain("<h1");
    expect(await counts()).toEqual({ users: 0, accounts: 0, sessions: 0, admitted: 0 });
  });
  it("renders callback storage failure without creating another session", async () => {
    mockGithub(); await complete(bindings, await start(bindings));
    const before = await counts();
    await bindings.DB.prepare(`CREATE TRIGGER fixture_callback_storage_failure BEFORE UPDATE ON "account" BEGIN SELECT RAISE(ABORT,'Offline fixture storage failure'); END`).run();
    const config = { ...bindings, ADMISSION_ENABLED: "false" };
    const response = await complete(config, await start(config));
    expect(response.status).toBe(500);
    expect(response.headers.get("content-type")).toMatch(/^text\/html(?:;|$)/);
    expect(response.headers.get("content-disposition")).toBeNull();
    expect(await counts()).toEqual(before);
  });
  function counts(): Promise<{ users: number; accounts: number; sessions: number; admitted: number } | null> {
    return bindings.DB.prepare('SELECT (SELECT COUNT(*) FROM "user") AS users,(SELECT COUNT(*) FROM "account") AS accounts,(SELECT COUNT(*) FROM "session") AS sessions,(SELECT COUNT(*) FROM relay_accounts) AS admitted').first();
  }
  it.each([
    { label: "disabled admission", override: { ADMISSION_ENABLED: "false" } },
    { label: "unapproved retention", override: { RETENTION_CONFIG: "{}" } },
    { label: "zero account capacity", override: { ADMISSION_LIMIT: "0" } },
  ])("rejects new identity persistence with $label", async ({ override }) => {
    const provider = mockGithub(); const config = { ...bindings, ...override };
    const response = await complete(config, await start(config));
    expect(response.status).toBe(302);
    expect(await counts()).toEqual({ users: 0, accounts: 0, sessions: 0, admitted: 0 });
    expect(new URL(response.headers.get("location")!).searchParams.get("error")).toBe("signup_disabled");
    expect(provider).toHaveBeenCalledTimes(3);
  });
  it.each(["profile-redirect", "emails-redirect"])("rejects redirected GitHub identity responses without forwarding credentials: %s", async scenario => {
    const provider = mockGithub("54321", scenario); const config = { ...bindings, ADMISSION_ENABLED: "false" };
    const response = await complete(config, await start(config));
    expect(response.status).toBe(302);
    expect(new URL(response.headers.get("location")!).searchParams.get("error")).toBe("unable_to_get_user_info");
    const requests = provider.mock.calls.map(([input, init]) => new Request(input, init));
    const identityRequests = requests.filter(request => new URL(request.url).origin === "https://api.github.com");
    expect(identityRequests.map(request => request.url)).toEqual(["https://api.github.com/user", "https://api.github.com/user/emails"]);
    for (const request of identityRequests) {
      expect(request.redirect).toBe("manual");
      expect(request.headers.get("user-agent")).toBe("ejm-cli-relay");
    }
    expect(requests.some(request => request.url === githubIdentityRedirectTarget)).toBe(false);
    expect(await counts()).toEqual({ users: 0, accounts: 0, sessions: 0, admitted: 0 });
  });
  it("uses current callback policy when admission closes after OAuth starts", async () => {
    const provider = mockGithub(); const pending = await start(bindings);
    const response = await complete({ ...bindings, ADMISSION_ENABLED: "false" }, pending);
    expect(response.status).toBe(302);
    expect(await counts()).toEqual({ users: 0, accounts: 0, sessions: 0, admitted: 0 });
    expect(new URL(response.headers.get("location")!).searchParams.get("error")).toBe("signup_disabled");
    expect(provider).toHaveBeenCalledTimes(3);
  });
  it("registers verified identity only with approved open admission and still requires terms", async () => {
    const provider = mockGithub();
    const response = await complete(bindings, await start(bindings));
    expect(response.status).toBe(302); expect(response.headers.get("location")).toBe(`${origin}/account`);
    expect(provider).toHaveBeenCalledTimes(3);
    expect(await counts()).toEqual({ users: 1, accounts: 1, sessions: 1, admitted: 0 });
    const account = await bindings.DB.prepare('SELECT "accessToken","refreshToken","idToken" FROM "account"').first();
    expect(account).toEqual({ accessToken: null, refreshToken: null, idToken: null });
    const browserCookie = response.headers.getSetCookie().map(value => value.split(";")[0]!).join("; ");
    const current = await call("/v1/account", "GET", undefined, undefined, { cookie: browserCookie });
    expect(current.status).toBe(200);
    expect(await current.json()).toMatchObject({ email: "oauth-54321@example.invalid", account: null });
    const admitted = await call("/v1/account/terms", "POST", { version: bindings.TERMS_VERSION }, undefined, { cookie: browserCookie, origin });
    expect(admitted.status).toBe(200); await admitted.text();
    expect(await counts()).toEqual({ users: 1, accounts: 1, sessions: 1, admitted: 1 });
  });
  it.each([
    { path: "/v1/account/close", body: { confirm: "close" } },
    { path: "/api/privacy/rights/request", body: { kind: "close", jurisdiction: "fixture" } },
  ])("keeps pre-terms closure effective after another GitHub login through $path", async ({ path, body }) => {
    const provider = mockGithub();
    const first = await complete(bindings, await start(bindings));
    const firstCookie = first.headers.getSetCookie().map(value => value.split(";")[0]!).join("; ");
    expect(await counts()).toEqual({ users: 1, accounts: 1, sessions: 1, admitted: 0 });
    const closed = await call(path, "POST", body, undefined, { cookie: firstCookie, origin });
    expect(closed.status).toBe(200);
    const receipt = await closed.json<{ requestId: string; statusKey: string; closed: boolean; relayClosureAcknowledged: boolean }>();
    expect(receipt).toMatchObject({ closed: true, relayClosureAcknowledged: true });
    expect(receipt.statusKey).toMatch(/^[a-f0-9]{64}$/u);
    const fresh = await complete(bindings, await start(bindings));
    expect(fresh.status).toBe(302); expect(provider).toHaveBeenCalledTimes(6);
    const freshCookie = fresh.headers.getSetCookie().map(value => value.split(";")[0]!).join("; ");
    const current = await call("/v1/account", "GET", undefined, undefined, { cookie: freshCookie });
    const terms = await call("/v1/account/terms", "POST", { version: bindings.TERMS_VERSION }, undefined, { cookie: freshCookie, origin });
    expect([current.status, terms.status]).toEqual([401, 401]);
    expect((await call("/v1/account", "GET", undefined, undefined, { cookie: firstCookie })).status).toBe(401);
    const marker = await bindings.DB.prepare("SELECT id,status,terms_version,terms_at,closed_at,cleanup_after FROM relay_accounts").first<{ id: string }>();
    expect(marker).toMatchObject({ status: "closed", terms_version: null, terms_at: null, closed_at: expect.any(Number), cleanup_after: expect.any(Number) });
    expect(await bindings.DB.prepare("SELECT account_id,action,state FROM privacy_operational_tasks WHERE request_id=?").bind(receipt.requestId).first()).toEqual({ account_id: marker!.id, action: "close", state: "pending" });
    const tracked = await call("/api/privacy/rights/status", "POST", { requestId: receipt.requestId, statusKey: receipt.statusKey }, undefined, { origin });
    expect(tracked.status).toBe(200); expect(await tracked.json()).toMatchObject({ kind: "close", operational_state: "pending" });
  });
  it("preserves admitted closure dates on retry and finishes queued work after identity purge", async () => {
    const owner = await identity();
    const record = () => bindings.DB.prepare("SELECT terms_version,terms_at,created_at,closed_at,cleanup_after FROM relay_accounts WHERE id=?").bind(owner.accountId).first<{ terms_version: string; terms_at: number; created_at: number; closed_at: number | null; cleanup_after: number | null }>();
    const accepted = await record();
    expect(accepted).toMatchObject({ terms_version: bindings.TERMS_VERSION, terms_at: expect.any(Number), created_at: expect.any(Number) });
    const closed = await call("/v1/account/close", "POST", { confirm: "close" }, owner.token);
    expect(closed.status).toBe(200); const receipt = await closed.json<{ requestId: string }>();
    const original = await record();
    expect(original).toMatchObject({ ...accepted, closed_at: expect.any(Number), cleanup_after: expect.any(Number) });
    const closedAt = original!.closed_at!;
    const clock = vi.spyOn(Date, "now").mockReturnValue(closedAt + 3_600_000);
    try {
      await revokeRelayAccount(bindings, owner.accountId, true);
      expect(await record()).toEqual(original);
      const policy = retentionPolicy(bindings)!;
      const purgeAt = closedAt + Math.max(policy.accountClosedDays, policy.providerIdentityDays, policy.paymentDays) * 86_400_000 + 1;
      clock.mockReturnValue(purgeAt); await retentionCleanup(bindings, purgeAt);
      expect(await record()).toBeNull();
      expect(await bindings.DB.prepare('SELECT id FROM "user" WHERE id=?').bind(owner.accountId).first()).toBeNull();
      await evictDurableObject(bindings.ACCOUNT_QUOTA.get(bindings.ACCOUNT_QUOTA.idFromName(owner.accountId)));
      await operationalCleanup(bindings);
      expect(await bindings.DB.prepare("SELECT state FROM privacy_operational_tasks WHERE request_id=?").bind(receipt.requestId).first()).toEqual({ state: "exception" });
      expect(await record()).toBeNull();
      expect(await bindings.DB.prepare('SELECT id FROM "user" WHERE id=?').bind(owner.accountId).first()).toBeNull();
    } finally { clock.mockRestore(); }
  });
  it("keeps linked existing login, logout and privacy available while admission is closed", async () => {
    const provider = mockGithub();
    const first = await complete(bindings, await start(bindings));
    expect(first.headers.get("location")).toBe(`${origin}/account`);
    const firstCookie = first.headers.getSetCookie().map(value => value.split(";")[0]!).join("; ");
    const admitted = await call("/v1/account/terms", "POST", { version: bindings.TERMS_VERSION }, undefined, { cookie: firstCookie, origin });
    expect(admitted.status).toBe(200); await admitted.text();
    const config = { ...bindings, ADMISSION_ENABLED: "false", RETENTION_CONFIG: "{}" };
    const response = await complete(config, await start(config));
    expect(response.status).toBe(302); expect(response.headers.get("location")).toBe(`${origin}/account`);
    expect(provider).toHaveBeenCalledTimes(6);
    expect(await counts()).toEqual({ users: 1, accounts: 1, sessions: 2, admitted: 1 });
    const browserCookie = response.headers.getSetCookie().map(value => value.split(";")[0]!).join("; ");
    const privacy = await call("/api/privacy", "GET", undefined, undefined, { cookie: browserCookie }, config);
    expect(privacy.status).toBe(200); expect(await privacy.json()).toMatchObject({ optedOut: false });
    const terms = await call("/v1/account/terms", "POST", { version: bindings.TERMS_VERSION }, undefined, { cookie: browserCookie, origin }, config);
    expect(terms.status).toBe(503); await terms.text();
    const signedOut = await call("/api/auth/sign-out", "POST", {}, undefined, { cookie: browserCookie, origin }, config);
    expect(signedOut.status).toBe(200); await signedOut.text();
    const after = await call("/api/privacy", "GET", undefined, undefined, { cookie: browserCookie }, config);
    expect(after.status).toBe(401); await after.text();
    expect(await counts()).toEqual({ users: 1, accounts: 1, sessions: 1, admitted: 1 });
  });
  it("lets a fresh GitHub session stop its account's share without transport enrollment", async () => {
    mockGithub();
    const first = await complete(bindings, await start(bindings));
    const firstCookie = first.headers.getSetCookie().map(value => value.split(";")[0]!).join("; ");
    expect((await call("/v1/account/terms", "POST", { version: bindings.TERMS_VERSION }, undefined, { cookie: firstCookie, origin })).status).toBe(200);
    const device = await call("/v1/devices", "POST", { label: "Fixture host" }, undefined, { cookie: firstCookie, origin });
    const deviceId = (await device.json<{ id: string }>()).id;
    const room = "opaque_fresh_browser_stop_01";
    const issued = await call(`/v1/rooms/${room}/tickets`, "POST", { role: "host", deviceId }, undefined, { cookie: firstCookie, origin });
    expect(issued.status).toBe(200); await issued.text();
    const config = { ...bindings, ADMISSION_ENABLED: "false" };
    const fresh = await complete(config, await start(config));
    const browserCookie = fresh.headers.getSetCookie().map(value => value.split(";")[0]!).join("; ");
    expect(await (await call("/v1/account", "GET", undefined, undefined, { cookie: browserCookie }, config)).json()).toMatchObject({ deviceId: null });
    expect(await (await call("/v1/rooms", "GET", undefined, undefined, { cookie: browserCookie }, config)).json()).toMatchObject({ rooms: [{ id: room }] });
    for (const supplied of [undefined, "https://attacker.invalid", "null"]) {
      const headers = new Headers({ cookie: browserCookie }); if (supplied) headers.set("origin", supplied);
      expect((await call(`/v1/rooms/${room}/stop`, "POST", {}, undefined, headers, config)).status).toBe(403);
    }
    expect((await call(`/v1/rooms/${room}/stop`, "POST", {}, undefined, { cookie: browserCookie, origin }, config)).status).toBe(200);
    const current = await bindings.DB.prepare('SELECT id FROM "user"').first<{ id: string }>();
    const context = await createAuth(bindings).$context;
    const session = await context.internalAdapter.createSession(current!.id);
    expect((await call(`/v1/rooms/${room}/stop`, "POST", {}, session.token, undefined, config)).status).toBe(200);
    expect((await call(`/v1/rooms/${room}/tickets`, "POST", { role: "host", deviceId }, session.token, undefined, config)).status).toBe(403);
    expect(await bindings.DB.prepare("SELECT COUNT(*) AS n FROM relay_devices").first()).toEqual({ n: 1 });
  });
});

describe("maintained device grant", () => {
  it("enforces client, access label, polling slowdown and expiration", async () => {
    expect((await call("/api/auth/device/code", "POST", { client_id: "attacker", scope: "relay" })).status).toBe(400);
    expect((await call("/api/auth/device/code", "POST", { client_id: "ejm-cli-relay", scope: "admin" })).status).toBe(400);
    const response = await call("/api/auth/device/code", "POST", { client_id: "ejm-cli-relay", scope: "relay" });
    expect(response.status).toBe(200);
    const grant = await response.json<{ device_code: string; verification_uri: string; interval: number; expires_in: number }>();
    expect(grant).toMatchObject({ verification_uri: `${origin}/device`, interval: 5, expires_in: 600 });
    const body = { client_id: "ejm-cli-relay", grant_type: "urn:ietf:params:oauth:grant-type:device_code", device_code: grant.device_code };
    expect(await (await call("/api/auth/device/token", "POST", body)).json()).toMatchObject({ error: "authorization_pending" });
    expect(await (await call("/api/auth/device/token", "POST", body)).json()).toMatchObject({ error: "slow_down" });
    await bindings.DB.prepare('UPDATE "deviceCode" SET "expiresAt"=?,"lastPolledAt"=NULL').bind(new Date(0).toISOString()).run();
    expect(await (await call("/api/auth/device/token", "POST", body)).json()).toMatchObject({ error: "expired_token" });
  });
  it("uses a shared maintained auth bucket without retaining raw IPs or user agents", async () => {
    for (let index = 0; index < 10; index++) {
      const response = await call("/api/auth/device/code", "POST", { client_id: "ejm-cli-relay", scope: "relay" }, undefined, { "cf-connecting-ip": "203.0.113.99", "x-forwarded-for": "203.0.113.98", "user-agent": "SENTINEL_raw_user_agent" });
      expect(response.status).toBe(200); await response.text();
    }
    expect((await call("/api/auth/device/code", "POST", { client_id: "ejm-cli-relay", scope: "relay" })).status).toBe(429);
    const rows = await bindings.DB.prepare('SELECT key FROM "rateLimit"').all<{ key: string }>();
    expect(rows.results.length).toBeGreaterThan(0);
    expect(JSON.stringify(rows.results)).not.toMatch(/203\.0\.113|SENTINEL/);
    const owner = await identity();
    const session = await bindings.DB.prepare('SELECT "ipAddress","userAgent" FROM "session" WHERE id=?').bind(owner.sessionId).first();
    expect(session).toEqual({ ipAddress: null, userAgent: null });
  });
  it("binds approval to one browser session and atomically decides once", async () => {
    const owner = await identity(); const other = await identity();
    const issued = await call("/api/auth/device/code", "POST", { client_id: "ejm-cli-relay", scope: "relay" });
    const grant = await issued.json<{ user_code: string; device_code: string }>();
    await call(`/api/auth/device?user_code=${encodeURIComponent(grant.user_code)}`, "GET", undefined, owner.token);
    expect((await call("/api/auth/device/approve", "POST", { userCode: grant.user_code }, other.token, { origin })).status).toBeGreaterThanOrEqual(400);
    const responses = await Promise.all(["approve", "deny"].map(action => call(`/api/auth/device/${action}`, "POST", { userCode: grant.user_code }, owner.token, { origin })));
    expect(responses.filter(response => response.status === 200)).toHaveLength(1);
    expect((await call("/api/auth/device/approve", "POST", { userCode: grant.user_code }, owner.token)).status).toBe(403);
  });
  it("accepts RFC8628 form encoding without exposing the code in a URL", async () => {
    const response = await worker.fetch(new Request(`${origin}/api/auth/device/code`, { method: "POST", headers: { "content-type": "application/x-www-form-urlencoded" }, body: "client_id=ejm-cli-relay&scope=relay" }), bindings);
    expect(response.status).toBe(200);
    expect((await response.json<{ verification_uri: string }>()).verification_uri).toBe(`${origin}/device`);
  });
});

describe("durable admission and hibernating forwarding", () => {
  function frameOrClose(socket: WebSocket): Promise<number[] | null> {
    return new Promise(resolve => {
      socket.addEventListener("message", event => resolve(Array.from(new Uint8Array(event.data as ArrayBuffer))), { once: true });
      socket.addEventListener("close", () => resolve(null), { once: true });
    });
  }
  it.each([false, true])("preserves the account owner after Room eviction with disconnected transports=%s", async disconnected => {
    const owner = await identity(); const other = await identity(); const room = "opaque_durable_owner_0001";
    await ticket(owner, room);
    const stub = bindings.ROOM.get(bindings.ROOM.idFromName(room));
    if (disconnected) {
      const host = await connect(room, "host", await ticket(owner, room));
      const controller = await connect(room, "controller", await ticket(owner, room, "controller"));
      host.webSocket!.accept(); controller.webSocket!.accept();
      const closed = [frameOrClose(host.webSocket!), frameOrClose(controller.webSocket!)];
      host.webSocket!.close(); controller.webSocket!.close();
      expect(await Promise.all(closed)).toEqual([null, null]);
      expect(await runInDurableObject(stub, async (_instance, state) => state.getWebSockets().length)).toBe(0);
    }
    await runInDurableObject(stub, async () => undefined);
    await evictDurableObject(stub);
    const collision = await call(`/v1/rooms/${room}/tickets`, "POST", { role: "host", deviceId: other.deviceId }, other.token);
    expect(collision.status).toBe(403); await collision.text();
    expect((await call(`/v1/rooms/${room}/stop`, "POST", {}, other.token)).status).toBe(403);
    expect(await bindings.DB.prepare("SELECT COUNT(*) AS n FROM connection_tickets WHERE account_id=? AND room=?").bind(other.accountId, room).first()).toEqual({ n: 0 });
    expect(await (await call("/v1/rooms", "GET", undefined, other.token)).json()).toEqual({ rooms: [] });
    const host = await connect(room, "host", await ticket(owner, room));
    const controller = await connect(room, "controller", await ticket(owner, room, "controller"));
    expect(host.status).toBe(101); expect(controller.status).toBe(101);
    host.webSocket!.accept(); controller.webSocket!.accept(); controller.webSocket!.binaryType = "arraybuffer";
    const receiving = frameOrClose(controller.webSocket!); host.webSocket!.send(new Uint8Array([79, 83]));
    expect(await receiving).toEqual([79, 83]); host.webSocket!.close(); controller.webSocket!.close();
    await ticket(other, "opaque_collision_recovery_01");
  });
  it("retains the original owner deadline after failed ticket publication and cleans it only at expiry", async () => {
    const owner = await identity(); const room = "opaque_owner_publish_0001";
    await bindings.DB.prepare("CREATE TRIGGER fixture_ticket_publish_failure BEFORE INSERT ON connection_tickets BEGIN SELECT RAISE(ABORT,'Offline fixture publication failure'); END").run();
    expect((await call(`/v1/rooms/${room}/tickets`, "POST", { role: "host", deviceId: owner.deviceId }, owner.token)).status).toBe(503);
    const stub = bindings.ROOM.get(bindings.ROOM.idFromName(room));
    const original = await runInDurableObject(stub, async (_instance, state) => state.storage.sql.exec<{ account_id: string; expires_at: number }>("SELECT account_id,expires_at FROM owner").one());
    expect(original.account_id).toBe(owner.accountId);
    await bindings.DB.prepare("DROP TRIGGER fixture_ticket_publish_failure").run();
    await evictDurableObject(stub); await ticket(owner, room);
    const extension = await stub.fetch("https://internal/claim", { method: "POST", body: JSON.stringify({ accountId: owner.accountId, expiresAt: original.expires_at + 1, created: true }) });
    expect(extension.status).toBe(409); await extension.text();
    expect(await runDurableObjectAlarm(stub)).toBe(true);
    expect(await runInDurableObject(stub, async (_instance, state) => ({ owner: state.storage.sql.exec("SELECT account_id,expires_at FROM owner").one(), alarm: await state.storage.getAlarm() }))).toEqual({ owner: original, alarm: original.expires_at });
    vi.spyOn(Date, "now").mockReturnValue(original.expires_at);
    try {
      expect(await runDurableObjectAlarm(stub)).toBe(true);
      expect(await runInDurableObject(stub, async (_instance, state) => ({ rows: state.storage.sql.exec("SELECT * FROM owner").toArray(), alarm: await state.storage.getAlarm() }))).toEqual({ rows: [], alarm: null });
    } finally { vi.restoreAllMocks(); }
  });
  it("retires ownerless legacy transports and requires a fresh owned share ID", async () => {
    const owner = await identity(); const room = "opaque_unowned_legacy_001";
    const host = await connect(room, "host", await ticket(owner, room));
    const controller = await connect(room, "controller", await ticket(owner, room, "controller"));
    host.webSocket!.accept(); controller.webSocket!.accept();
    const pending = await ticket(owner, room);
    const closed = [frameOrClose(host.webSocket!), frameOrClose(controller.webSocket!)];
    const stub = bindings.ROOM.get(bindings.ROOM.idFromName(room));
    await runInDurableObject(stub, async (_instance, state) => { state.storage.sql.exec("DELETE FROM owner"); });
    await evictDurableObject(stub);
    const old = await connect(room, "host", pending); expect(old.status).toBe(403); await old.text();
    expect(await Promise.all(closed)).toEqual([null, null]);
    const retry = await call(`/v1/rooms/${room}/tickets`, "POST", { role: "host", deviceId: owner.deviceId }, owner.token);
    expect(retry.status).toBe(409); expect(await retry.json()).toMatchObject({ error: { code: "fresh_share_required" } });
    expect((await call(`/v1/rooms/${room}/stop`, "POST", {}, owner.token)).status).toBe(200);
    const fresh = await connect("opaque_fresh_legacy_0001", "host", await ticket(owner, "opaque_fresh_legacy_0001"));
    expect(fresh.status).toBe(101); fresh.webSocket!.accept(); fresh.webSocket!.close();
  });
  it("retries a failed nested close acknowledgement without retiring a newer share", async () => {
    const owner = await identity(); const room = "opaque_close_ack_old_0001"; await ticket(owner, room);
    const quota = bindings.ACCOUNT_QUOTA.get(bindings.ACCOUNT_QUOTA.idFromName(owner.accountId)); let attempts = 0;
    await runInDurableObject(quota, async instance => {
      const original = (instance as AccountQuota).fetch.bind(instance);
      vi.spyOn(instance as AccountQuota, "fetch").mockImplementation(request => {
        if (new URL(request.url).pathname === "/ack-close" && ++attempts === 1) return Promise.resolve(Response.json({ error: "fixture_ack_unavailable" }, { status: 503 }));
        return original(request);
      });
    });
    expect((await call(`/v1/rooms/${room}/stop`, "POST", {}, owner.token)).status).toBe(503);
    const next = "opaque_close_ack_next_001"; await ticket(owner, next);
    expect((await call(`/v1/rooms/${room}/stop`, "POST", {}, owner.token)).status).toBe(200);
    expect(attempts).toBe(2);
    expect(await (await call("/v1/rooms", "GET", undefined, owner.token)).json()).toMatchObject({ rooms: [{ id: next }] });
    expect(await runInDurableObject(quota, async (_instance, state) => state.storage.sql.exec("SELECT * FROM pending_close").toArray())).toEqual([]);
    const connected = await connect(next, "host", await ticket(owner, next));
    expect(connected.status).toBe(101); connected.webSocket!.accept(); connected.webSocket!.close();
  });
  it("expires an old pending close before the same routing ID belongs to a later account", async () => {
    const owner = await identity(); const other = await identity(); const room = "opaque_expired_close_reuse_01";
    await ticket(owner, room);
    const quota = bindings.ACCOUNT_QUOTA.get(bindings.ACCOUNT_QUOTA.idFromName(owner.accountId));
    const stop = await quota.fetch("https://internal/stop", { method: "POST", body: JSON.stringify({ accountId: owner.accountId, room }) });
    expect(stop.status).toBe(200); const stopped = await stop.json<{ expiresAt: number }>();
    expect(await runInDurableObject(quota, async (_instance, state) => state.storage.sql.exec("SELECT room,expires_at FROM pending_close").toArray())).toEqual([{ room, expires_at: stopped.expiresAt }]);
    vi.spyOn(Date, "now").mockReturnValue(stopped.expiresAt);
    try {
      await ticket(other, room); let closes = 0;
      const namespace = bindings.ROOM;
      const config = { ...bindings, ROOM: { idFromName: (name: string) => namespace.idFromName(name), get: (id: DurableObjectId) => ({ fetch: async (input: RequestInfo, init?: RequestInit) => { closes += 1; return namespace.get(id).fetch(input, init); } }) } as unknown as DurableObjectNamespace };
      await revokeRelayAccount(config, owner.accountId, true);
      expect(closes).toBe(0);
      expect(await runInDurableObject(quota, async (_instance, state) => state.storage.sql.exec("SELECT * FROM pending_close").toArray())).toEqual([]);
      const stub = bindings.ROOM.get(bindings.ROOM.idFromName(room));
      expect(await runInDurableObject(stub, async (_instance, state) => state.storage.sql.exec<{ account_id: string; expires_at: number }>("SELECT account_id,expires_at FROM owner").one())).toEqual({ account_id: other.accountId, expires_at: stopped.expiresAt + LIMITS.shareMs });
      const connected = await connect(room, "host", await ticket(other, room));
      expect(connected.status).toBe(101); connected.webSocket!.accept(); connected.webSocket!.close();
    } finally { vi.restoreAllMocks(); }
  });
  it("keeps dashboard-stopped epochs retired across owned retries and eviction", async () => {
    const owner = await identity(); const other = await identity();
    const rooms = ["opaque_retired_room_0001", "opaque_retired_room_0002", "opaque_retired_room_0003"];
    const quota = bindings.ACCOUNT_QUOTA.get(bindings.ACCOUNT_QUOTA.idFromName(owner.accountId));
    const host = await connect(rooms[0]!, "host", await ticket(owner, rooms[0]!));
    const controller = await connect(rooms[0]!, "controller", await ticket(owner, rooms[0]!, "controller"));
    host.webSocket!.accept(); controller.webSocket!.accept();
    const closed = [host, controller].map(response => new Promise<CloseEvent>(resolve => response.webSocket!.addEventListener("close", resolve, { once: true })));
    const pending = await ticket(owner, rooms[0]!);
    const stop = async (room: string, token = owner.token) => { const response = await call(`/v1/rooms/${room}/stop`, "POST", {}, token); await response.text(); return response.status; };
    expect(await stop(rooms[0]!)).toBe(200);
    expect((await Promise.all(closed)).every(event => event.code === 1000)).toBe(true);
    const rejected = await connect(rooms[0]!, "host", pending);
    await rejected.text(); expect([401, 403]).toContain(rejected.status);
    const reconnect = await call(`/v1/rooms/${rooms[0]}/tickets`, "POST", { role: "host", deviceId: owner.deviceId }, owner.token);
    await reconnect.text(); expect(reconnect.status).toBe(403);
    const next = await ticket(owner, rooms[1]!);
    expect(await stop(rooms[0]!, other.token)).toBe(403);
    expect(await stop(rooms[0]!)).toBe(200);
    expect(await (await call("/v1/rooms", "GET", undefined, owner.token)).json()).toMatchObject({ rooms: [{ id: rooms[1] }] });
    const nextHost = await connect(rooms[1]!, "host", next); expect(nextHost.status).toBe(101); nextHost.webSocket!.accept();
    expect(await stop(rooms[1]!)).toBe(200);
    await evictDurableObject(quota);
    for (const room of rooms.slice(0, 2)) for (const role of ["host", "controller"]) {
      const response = await call(`/v1/rooms/${room}/tickets`, "POST", { role, deviceId: owner.deviceId }, owner.token);
      await response.text(); expect(response.status).toBe(403);
    }
    const fresh = await connect(rooms[2]!, "host", await ticket(owner, rooms[2]!));
    expect(fresh.status).toBe(101); fresh.webSocket!.accept(); fresh.webSocket!.close();
  });
  it("rejects a redeemed connection delayed past dashboard stop and Room eviction", async () => {
    const owner = await identity(); const room = "opaque_delayed_room_0001";
    const quota = bindings.ACCOUNT_QUOTA.get(bindings.ACCOUNT_QUOTA.idFromName(owner.accountId));
    const issued = await ticket(owner, room);
    const redeemed = await quota.fetch("https://internal/redeem", { method: "POST", body: JSON.stringify({ accountId: owner.accountId, room, role: "host", hash: await hashToken(issued) }) });
    expect(redeemed.status).toBe(200); const lease = await redeemed.json<Lease>();
    const stopped = await call(`/v1/rooms/${room}/stop`, "POST", {}, owner.token); await stopped.text(); expect(stopped.status).toBe(200);
    const stub = bindings.ROOM.get(bindings.ROOM.idFromName(room));
    let expiry = 0;
    for (const evict of [false, true]) {
      if (evict) {
        expiry = await runInDurableObject(stub, async (_instance, state) => {
          const retirement = state.storage.sql.exec<{ account_id: string; expires_at: number }>("SELECT account_id,expires_at FROM retirement").one();
          expect(retirement.account_id).toBe(owner.accountId); expect(await state.storage.getAlarm()).toBe(retirement.expires_at);
          return retirement.expires_at;
        });
        expect(await runDurableObjectAlarm(stub)).toBe(true);
        expect(await runInDurableObject(stub, async (_instance, state) => state.storage.getAlarm())).toBe(expiry);
        await evictDurableObject(stub);
      }
      const delayed = await stub.fetch("https://internal/connect", { headers: { upgrade: "websocket", "x-internal-lease": JSON.stringify(lease) } });
      if (delayed.webSocket) { delayed.webSocket.accept(); delayed.webSocket.close(); } else await delayed.text();
      expect(delayed.status).toBe(403);
    }
    await evictDurableObject(quota);
    const renewal = await quota.fetch("https://internal/reserve", { method: "POST", body: JSON.stringify(lease) });
    await renewal.text(); expect(renewal.status).toBe(403);
    const clock = vi.spyOn(Date, "now").mockReturnValue(expiry);
    try {
      expect(await runDurableObjectAlarm(stub)).toBe(true);
      expect(await runInDurableObject(stub, async (_instance, state) => ({ rows: state.storage.sql.exec("SELECT * FROM retirement").toArray(), alarm: await state.storage.getAlarm() }))).toEqual({ rows: [], alarm: null });
      await evictDurableObject(stub);
      expect(await runInDurableObject(stub, async (_instance, state) => state.storage.sql.exec("SELECT * FROM retirement").toArray())).toEqual([]);
    } finally { clock.mockRestore(); }
  });
  it.each(["free", "paid"])("charges every retired-epoch retry against the same daily %s control bound", async plan => {
    const owner = await identity(); const room = "opaque_budget_room_0001";
    if (plan === "paid") await bindings.DB.prepare("INSERT INTO relay_billing(account_id,customer_id,plan,status,paid_until,cancel_at_period_end,updated_at) VALUES(?,?,'paid','active',?,0,?)").bind(owner.accountId, `customer_${owner.accountId}`, Date.now() + LIMITS.shareMs, Date.now()).run();
    await ticket(owner, room);
    const stopped = await call(`/v1/rooms/${room}/stop`, "POST", {}, owner.token); await stopped.text(); expect(stopped.status).toBe(200);
    const quota = bindings.ACCOUNT_QUOTA.get(bindings.ACCOUNT_QUOTA.idFromName(owner.accountId));
    await runInDurableObject(quota, async (_instance, state) => { state.storage.sql.exec("UPDATE windows SET controls=297"); });
    const scope = { accountId: owner.accountId, deviceId: owner.deviceId, sessionId: owner.sessionId, room, role: "host", hash: "a".repeat(64), expiresAt: Date.now() + LIMITS.ticketMs };
    for (const path of ["issue", "redeem", "reserve"]) {
      const response = await quota.fetch(`https://internal/${path}`, { method: "POST", body: JSON.stringify(scope) });
      await response.text(); expect(response.status).toBe(403);
    }
    const exhausted = await quota.fetch("https://internal/issue", { method: "POST", body: JSON.stringify(scope) });
    await exhausted.text(); expect(exhausted.status).toBe(429);
    expect(await runInDurableObject(quota, async (_instance, state) => state.storage.sql.exec<{ controls: number }>("SELECT controls FROM windows").one().controls)).toBe(300);
  });
  it("bounds retired metadata without evicting live fences and reclaims expired entries", async () => {
    const owner = await identity(); const room = "opaque_capacity_room_0001"; await ticket(owner, room);
    const quota = bindings.ACCOUNT_QUOTA.get(bindings.ACCOUNT_QUOTA.idFromName(owner.accountId));
    const stop = async () => { const response = await call(`/v1/rooms/${room}/stop`, "POST", {}, owner.token); await response.text(); return response.status; };
    expect(await stop()).toBe(200);
    const originalExpiry = await runInDurableObject(quota, async (_instance, state) => {
      const expiry = state.storage.sql.exec<{ expires_at: number }>("SELECT expires_at FROM retired WHERE room=?", room).one().expires_at;
      state.storage.sql.exec("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x<599) INSERT INTO retired(room,expires_at) SELECT 'retired_fixture_epoch_'||x,? FROM n", expiry);
      return expiry;
    });
    const full = await call("/v1/rooms/opaque_capacity_room_0002/tickets", "POST", { role: "host", deviceId: owner.deviceId }, owner.token);
    await full.text(); expect(full.status).toBe(429); expect(await stop()).toBe(200);
    await runInDurableObject(quota, async (_instance, state) => {
      expect(state.storage.sql.exec<{ expires_at: number }>("SELECT expires_at FROM retired WHERE room=?", room).one().expires_at).toBe(originalExpiry);
      state.storage.sql.exec("UPDATE retired SET expires_at=? WHERE room<>?", Date.now() - 1, room);
    });
    const cleanup = await quota.fetch("https://internal/cleanup", { method: "POST" }); await cleanup.text(); expect(cleanup.status).toBe(200);
    expect(await runInDurableObject(quota, async (_instance, state) => state.storage.sql.exec<{ n: number }>("SELECT COUNT(*) AS n FROM retired").one().n)).toBe(1);
    await ticket(owner, "opaque_capacity_room_0002");
  });
  it("forwards binary sentinel only to the peer and preserves tickets across eviction", async () => {
    const owner = await identity(); const room = "opaque_fixture_room_0001";
    const hostTicket = await ticket(owner, room); const controllerTicket = await ticket(owner, room, "controller");
    const quota = bindings.ACCOUNT_QUOTA.get(bindings.ACCOUNT_QUOTA.idFromName(owner.accountId));
    await evictDurableObject(quota);
    const host = await connect(room, "host", hostTicket); expect(host.status).toBe(101);
    await bindings.DB.prepare("UPDATE relay_devices SET cli_major=0 WHERE id=?").bind(owner.deviceId).run();
    const controller = await connect(room, "controller", controllerTicket); expect(controller.status).toBe(101);
    const hostSocket = host.webSocket!; const controllerSocket = controller.webSocket!; hostSocket.accept(); controllerSocket.accept(); hostSocket.binaryType = "arraybuffer"; controllerSocket.binaryType = "arraybuffer";
    const sentinel = new TextEncoder().encode("SENTINEL_ciphertext_fixture_never_persist_7d0aa");
    const receiving = message(controllerSocket); hostSocket.send(sentinel);
    expect(new Uint8Array(await receiving as ArrayBuffer)).toEqual(sentinel);
    expect((await connect(room, "host", hostTicket)).status).toBe(401);
    const stub = bindings.ROOM.get(bindings.ROOM.idFromName(room));
    await evictDurableObject(stub);
    const again = message(hostSocket); controllerSocket.send(new Uint8Array([1, 2, 3]));
    expect(new Uint8Array(await again as ArrayBuffer)).toEqual(new Uint8Array([1, 2, 3]));
    const attachments = await runInDurableObject(stub, async (_instance, state) => state.getWebSockets().map(socket => socket.deserializeAttachment()));
    expect(JSON.stringify(attachments)).not.toContain("SENTINEL");
    expect(attachments.map((value: { frames: number }) => value.frames)).toEqual([511, 511]);
    const persisted = await runInDurableObject(stub, async (_instance, state) => Array.from(await state.storage.list()));
    expect(JSON.stringify(persisted)).not.toContain("SENTINEL");
    const usage: Record<string, unknown>[] = [];
    const authority = bindings.PRIVACY_AUTHORITY.get(bindings.PRIVACY_AUTHORITY.idFromName("privacy-v1"));
    await runInDurableObject(authority, async instance => {
      const service = instance as PrivacyAuthority; const original = service.fetch.bind(service);
      vi.spyOn(service, "fetch").mockImplementation(async request => {
        if (new URL(request.url).pathname === "/internal/usage") usage.push(await request.clone().json<Record<string, unknown>>());
        return original(request);
      });
    });
    await runInDurableObject(stub, async (instance, state) => (instance as Room).webSocketClose(state.getWebSockets("host")[0]!));
    hostSocket.close(); controllerSocket.close();
    expect(usage).toHaveLength(2);
    expect(usage).toMatchObject([{ accountId: owner.accountId, frames: 1, osFamily: "linux", cliMajor: 1 }, { accountId: owner.accountId, frames: 1, osFamily: "linux", cliMajor: 0 }]);
    expect(JSON.stringify(usage)).not.toContain("SENTINEL");
  });
  it.each(["webSocketClose", "webSocketError"] as const)("isolates delayed old %s callbacks from a replacement pair after hibernation", async callback => {
    const owner = await identity(); const room = `opaque_generation_${callback}`;
    const oldHost = await connect(room, "host", await ticket(owner, room));
    const oldController = await connect(room, "controller", await ticket(owner, room, "controller"));
    oldHost.webSocket!.accept(); oldController.webSocket!.accept();
    const stub = bindings.ROOM.get(bindings.ROOM.idFromName(room));
    await evictDurableObject(stub);
    let departed: WebSocket | undefined;
    await runInDurableObject(stub, async (instance, state) => {
      departed = state.getWebSockets("host")[0]!;
      for (const socket of state.getWebSockets()) socket.serializeAttachment({ ...socket.deserializeAttachment(), expiresAt: Date.now() - 1 });
      // Let admission finish before explicitly delivering either old callback.
      vi.spyOn(instance as Room, "webSocketClose").mockResolvedValue(undefined);
      vi.spyOn(instance as Room, "webSocketError").mockResolvedValue(undefined);
    });
    const host = await connect(room, "host", await ticket(owner, room));
    const controller = await connect(room, "controller", await ticket(owner, room, "controller"));
    expect(host.status).toBe(101); expect(controller.status).toBe(101);
    const hostSocket = host.webSocket!; const controllerSocket = controller.webSocket!;
    hostSocket.accept(); controllerSocket.accept(); hostSocket.binaryType = "arraybuffer"; controllerSocket.binaryType = "arraybuffer";
    try {
      const receiving = frameOrClose(controllerSocket);
      await runInDurableObject(stub, async instance => {
        const old = departed!; departed = undefined; vi.restoreAllMocks(); await (instance as Room)[callback](old);
      });
      if (hostSocket.readyState === WebSocket.OPEN) hostSocket.send(new Uint8Array([31, 37, 41]));
      expect(await receiving).toEqual([31, 37, 41]);
      const reply = frameOrClose(hostSocket); controllerSocket.send(new Uint8Array([43, 47]));
      expect(await reply).toEqual([43, 47]);
    } finally { hostSocket.close(); controllerSocket.close(); oldHost.webSocket!.close(); oldController.webSocket!.close(); }
  });
  it("retires a lone paired survivor before admitting a fresh handshake after hibernation", async () => {
    const owner = await identity(); const room = "opaque_survivor_room_0001";
    const oldHost = await connect(room, "host", await ticket(owner, room));
    const oldController = await connect(room, "controller", await ticket(owner, room, "controller"));
    oldHost.webSocket!.accept(); oldController.webSocket!.accept(); oldController.webSocket!.binaryType = "arraybuffer";
    const stub = bindings.ROOM.get(bindings.ROOM.idFromName(room));
    await evictDurableObject(stub);
    let callbacks = 0;
    await runInDurableObject(stub, async instance => {
      vi.spyOn(instance as Room, "webSocketClose").mockImplementation(async socket => { socket.close(1000, "fixture_disconnect"); callbacks += 1; });
    });
    const disconnected = frameOrClose(oldHost.webSocket!);
    oldHost.webSocket!.close(1000, "fixture_disconnect");
    expect(await disconnected).toBeNull();
    expect(await runInDurableObject(stub, async (_instance, state) => { vi.restoreAllMocks(); expect(callbacks).toBe(1); return state.getWebSockets("host").length; })).toBe(0);
    const oldOutcome = frameOrClose(oldController.webSocket!);
    const host = await connect(room, "host", await ticket(owner, room));
    expect(host.status).toBe(101); host.webSocket!.accept();
    let controller: Response | undefined;
    try {
      controller = await connect(room, "controller", await ticket(owner, room, "controller"));
      expect(controller.status).toBe(101); controller.webSocket!.accept(); controller.webSocket!.binaryType = "arraybuffer";
      const receiving = frameOrClose(controller.webSocket!); host.webSocket!.send(new Uint8Array([53, 59, 61]));
      expect(await oldOutcome).toBeNull(); expect(await receiving).toEqual([53, 59, 61]);
    } finally { host.webSocket!.close(); controller?.webSocket?.close(); if (controller && !controller.webSocket) await controller.text(); oldController.webSocket!.close(); }
  });
  it("requires fresh transports when hibernation restores legacy attachments", async () => {
    const owner = await identity(); const room = "opaque_legacy_pair_00001";
    const oldHost = await connect(room, "host", await ticket(owner, room));
    const oldController = await connect(room, "controller", await ticket(owner, room, "controller"));
    oldHost.webSocket!.accept(); oldController.webSocket!.accept();
    const stub = bindings.ROOM.get(bindings.ROOM.idFromName(room));
    await runInDurableObject(stub, async (_instance, state) => {
      for (const socket of state.getWebSockets()) {
        const attachment = socket.deserializeAttachment() as Record<string, unknown>;
        delete attachment.generation; delete attachment.paired; socket.serializeAttachment(attachment);
      }
    });
    await evictDurableObject(stub);
    const closed = [frameOrClose(oldHost.webSocket!), frameOrClose(oldController.webSocket!)];
    const host = await connect(room, "host", await ticket(owner, room));
    let controller: Response | undefined;
    try {
      expect(host.status).toBe(101); host.webSocket!.accept();
      controller = await connect(room, "controller", await ticket(owner, room, "controller"));
      expect(controller.status).toBe(101); controller.webSocket!.accept(); controller.webSocket!.binaryType = "arraybuffer";
      const receiving = frameOrClose(controller.webSocket!); host.webSocket!.send(new Uint8Array([67, 71]));
      expect(await Promise.all(closed)).toEqual([null, null]); expect(await receiving).toEqual([67, 71]);
    } finally { host.webSocket?.close(); if (!host.webSocket) await host.text(); controller?.webSocket?.close(); if (controller && !controller.webSocket) await controller.text(); oldHost.webSocket!.close(); oldController.webSocket!.close(); }
  });
  it.each([
    { boundary: "expired peer", reason: "peer_lease_expired" },
    { boundary: "expired sender", reason: "lease_expired" },
    { boundary: "valid renewed sender", reason: "" },
  ])("checks $boundary authority after quota renewal", async ({ reason }) => {
    const owner = await identity(); const room = "opaque_forward_expiry_001";
    const host = await connect(room, "host", await ticket(owner, room));
    const controller = await connect(room, "controller", await ticket(owner, room, "controller"));
    expect(host.status).toBe(101); expect(controller.status).toBe(101);
    const hostSocket = host.webSocket!; const controllerSocket = controller.webSocket!;
    hostSocket.accept(); controllerSocket.accept(); controllerSocket.binaryType = "arraybuffer";
    const sentinel = new Uint8Array([19, 23, 29]);
    const outcome = new Promise<{ type: "message"; data: number[] } | { type: "close" }>(resolve => {
      controllerSocket.addEventListener("message", event => resolve({ type: "message", data: Array.from(new Uint8Array(event.data as ArrayBuffer)) }), { once: true });
      controllerSocket.addEventListener("close", () => resolve({ type: "close" }), { once: true });
    });
    const senderClosed = new Promise<CloseEvent>(resolve => hostSocket.addEventListener("close", resolve, { once: true }));
    const wall = Date.now(); const started = Math.min(wall, windowBoundary(wall) - LIMITS.leaseMs - 10_000);
    const clock = vi.spyOn(Date, "now").mockReturnValue(started);
    let renewed: Lease | undefined;
    try {
      const stub = bindings.ROOM.get(bindings.ROOM.idFromName(room));
      const final = await runInDurableObject(stub, async (_instance, state) => {
        const sender = state.getWebSockets("host")[0]!; const peer = state.getWebSockets("controller")[0]!;
        sender.serializeAttachment({ ...sender.deserializeAttachment(), expiresAt: started + 500, frames: 0, measuredFrames: 0, measuredBytes: 0 });
        peer.serializeAttachment({ ...peer.deserializeAttachment(), expiresAt: started + (reason === "peer_lease_expired" ? 1000 : LIMITS.leaseMs + 1000) });
        const delayed = new Room(state, { ...bindings, ACCOUNT_QUOTA: afterQuotaFetch(bindings.ACCOUNT_QUOTA, lease => {
          renewed = lease; clock.mockReturnValue(reason === "lease_expired" ? lease.expiresAt : started + 1000);
        }) });
        await delayed.webSocketMessage(sender, sentinel.buffer);
        const final = sender.deserializeAttachment() as Lease & { measuredFrames: number };
        peer.close(1000, "fixture_cleanup");
        return final;
      });
      expect(renewed?.expiresAt).toBe(started + LIMITS.leaseMs);
      expect(await outcome).toEqual(reason ? { type: "close" } : { type: "message", data: Array.from(sentinel) });
      if (reason) {
        expect((await senderClosed).reason).toBe(reason);
        expect(final.frames).toBe(0); expect(final.measuredFrames).toBe(0);
      } else {
        expect(final.expiresAt).toBe(renewed!.expiresAt); expect(final.frames).toBe(LIMITS.blockFrames - 1); expect(final.measuredFrames).toBe(1);
      }
    } finally { clock.mockRestore(); hostSocket.close(); controllerSocket.close(); }
  });
  it.each([
    { remaining: 1, forwards: false },
    { remaining: 2, forwards: true },
  ])("bounds forwarding by a real final $remaining-byte quota reservation", async ({ remaining, forwards }) => {
    const owner = await identity(); const room = "opaque_partial_bytes_0001";
    const host = await connect(room, "host", await ticket(owner, room));
    const controller = await connect(room, "controller", await ticket(owner, room, "controller"));
    expect(host.status).toBe(101); expect(controller.status).toBe(101);
    const hostSocket = host.webSocket!; const controllerSocket = controller.webSocket!;
    hostSocket.accept(); controllerSocket.accept(); controllerSocket.binaryType = "arraybuffer";
    const spentFrames = 63 * LIMITS.blockFrames; const spentBytes = 63 * LIMITS.blockBytes;
    await bindings.DB.prepare("INSERT INTO relay_billing(account_id,customer_id,plan,status,paid_until,cancel_at_period_end,updated_at) VALUES(?,?,'paid','active',?,0,?)").bind(owner.accountId, `customer_${owner.accountId}`, Date.now() + LIMITS.shareMs, Date.now()).run();
    const quota = bindings.ACCOUNT_QUOTA.get(bindings.ACCOUNT_QUOTA.idFromName(owner.accountId));
    await runInDurableObject(quota, async (instance, state) => {
      const limited = new AccountQuota(state, { ...bindings, PAID_DAILY_BYTES: String(spentBytes + remaining) });
      vi.spyOn(instance as AccountQuota, "fetch").mockImplementation(request => limited.fetch(request));
      state.storage.sql.exec("UPDATE windows SET frames=?,bytes=?", spentFrames, spentBytes);
    });
    const sentinel = new Uint8Array([89, 97]);
    const outcome = frameOrClose(controllerSocket);
    const senderClosed = new Promise<CloseEvent>(resolve => hostSocket.addEventListener("close", resolve, { once: true }));
    let renewed: Lease | undefined;
    try {
      const stub = bindings.ROOM.get(bindings.ROOM.idFromName(room));
      const final = await runInDurableObject(stub, async (_instance, state) => {
        const sender = state.getWebSockets("host")[0]!; const peer = state.getWebSockets("controller")[0]!;
        sender.serializeAttachment({ ...sender.deserializeAttachment(), frames: 0, bytes: 0, measuredFrames: 0, measuredBytes: 0 });
        const bounded = new Room(state, { ...bindings, ACCOUNT_QUOTA: afterQuotaFetch(bindings.ACCOUNT_QUOTA, lease => { renewed = lease; }) });
        await bounded.webSocketMessage(sender, sentinel.buffer);
        const final = sender.deserializeAttachment() as Lease & { measuredFrames: number };
        peer.close(1000, "fixture_cleanup"); return final;
      });
      expect(renewed).toMatchObject({ frames: LIMITS.blockFrames, bytes: remaining });
      const reserved = await runInDurableObject(quota, async (_instance, state) => state.storage.sql.exec<{ frames: number; bytes: number }>("SELECT frames,bytes FROM windows").one());
      expect(reserved).toEqual({ frames: spentFrames + LIMITS.blockFrames, bytes: spentBytes + remaining });
      expect({ outcome: await outcome, bytes: final.bytes, measuredFrames: final.measuredFrames }).toEqual({ outcome: forwards ? Array.from(sentinel) : null, bytes: 0, measuredFrames: forwards ? 1 : 0 });
      if (!forwards) expect(await senderClosed).toMatchObject({ code: 1008, reason: "daily_quota_exhausted" });
      else expect(final.frames).toBe(LIMITS.blockFrames - 1);
    } finally { vi.restoreAllMocks(); hostSocket.close(); controllerSocket.close(); }
  });
  it("rejects role substitution, cross-account room admission and live peer replacement", async () => {
    const owner = await identity(); const other = await identity(); const room = "opaque_fixture_room_0002";
    const hostTicket = await ticket(owner, room);
    expect((await connect(room, "controller", hostTicket)).status).toBe(401);
    expect((await call(`/v1/rooms/${room}/tickets`, "POST", { role: "controller", deviceId: other.deviceId }, other.token)).status).toBe(409);
    const host = await connect(room, "host", hostTicket); host.webSocket?.accept();
    const replacement = await ticket(owner, room);
    expect((await connect(room, "host", replacement)).status).toBe(409);
    host.webSocket?.close();
  });
  it("keeps one room and account-wide quota reservations across races and eviction", async () => {
    const owner = await identity(); const room = "opaque_fixture_room_0003";
    await ticket(owner, room);
    const races = await Promise.all(["opaque_other_room_00001", "opaque_other_room_00002"].map(id => call(`/v1/rooms/${id}/tickets`, "POST", { role: "host", deviceId: owner.deviceId }, owner.token)));
    expect(races.every(result => result.status === 409)).toBe(true);
    await Promise.all(races.map(response => response.text()));
    const quota = bindings.ACCOUNT_QUOTA.get(bindings.ACCOUNT_QUOTA.idFromName(owner.accountId));
    const scope = { accountId: owner.accountId, deviceId: owner.deviceId, sessionId: owner.sessionId, room, role: "host" };
    let frames = 0; let bytes = 0; let rejected = false;
    for (let index = 0; index < 110; index++) {
      if (index === 12) await evictDurableObject(quota);
      const response = await quota.fetch("https://internal/reserve", { method: "POST", body: JSON.stringify(scope) });
      if (response.status === 429) { rejected = true; break; }
      expect(response.status).toBe(200);
      const lease = await response.json<Lease>(); frames += lease.frames; bytes += lease.bytes;
    }
    expect(rejected).toBe(true); expect(frames).toBeLessThanOrEqual(LIMITS.dailyFrames); expect(bytes).toBe(LIMITS.dailyBytes);
    expect((await quota.fetch("https://internal/reserve", { method: "POST", body: JSON.stringify(scope) })).status).toBe(429);
  });
  it("rejects a ticket that expires during its authority lookup without consuming quota", async () => {
    const owner = await identity(); const room = "opaque_ticket_expiry_0001";
    const hash = await hashToken(await ticket(owner, room));
    const record = await bindings.DB.prepare("SELECT expires_at FROM connection_tickets WHERE hash=?").bind(hash).first<{ expires_at: number }>();
    const expiry = record!.expires_at;
    const clock = vi.spyOn(Date, "now").mockReturnValue(expiry - 1);
    try {
      const quota = bindings.ACCOUNT_QUOTA.get(bindings.ACCOUNT_QUOTA.idFromName(owner.accountId));
      const response = await runInDurableObject(quota, async (_instance, state) => {
        const delayed = new AccountQuota(state, { ...bindings, DB: afterDatabaseRead(bindings.DB, () => { clock.mockReturnValue(expiry); }) });
        return delayed.fetch(new Request("https://internal/redeem", { method: "POST", body: JSON.stringify({ accountId: owner.accountId, room, role: "host", hash }) }));
      });
      expect(response.status).toBe(401); expect(await response.json()).toMatchObject({ error: { code: "invalid_ticket" } });
      expect(await runInDurableObject(quota, async (_instance, state) => ({
        used: state.storage.sql.exec<{ used: number }>("SELECT used FROM tickets WHERE hash=?", hash).one().used,
        reserved: state.storage.sql.exec<{ frames: number; bytes: number }>("SELECT COALESCE(SUM(frames),0) AS frames,COALESCE(SUM(bytes),0) AS bytes FROM windows").one(),
      }))).toEqual({ used: 0, reserved: { frames: 0, bytes: 0 } });
    } finally { clock.mockRestore(); }
  });
  it("denies a lease renewal when its session expires during the authority lookup", async () => {
    const owner = await identity(); const room = "opaque_session_expiry_001"; await ticket(owner, room);
    const expiry = Date.now() + 1000;
    await bindings.DB.prepare('UPDATE "session" SET "expiresAt"=? WHERE id=?').bind(new Date(expiry).toISOString(), owner.sessionId).run();
    const clock = vi.spyOn(Date, "now").mockReturnValue(expiry - 1);
    try {
      const quota = bindings.ACCOUNT_QUOTA.get(bindings.ACCOUNT_QUOTA.idFromName(owner.accountId));
      const response = await runInDurableObject(quota, async (_instance, state) => {
        const delayed = new AccountQuota(state, { ...bindings, DB: afterDatabaseRead(bindings.DB, () => { clock.mockReturnValue(expiry); }) });
        return delayed.fetch(new Request("https://internal/reserve", { method: "POST", body: JSON.stringify({ accountId: owner.accountId, deviceId: owner.deviceId, sessionId: owner.sessionId, room, role: "host" }) }));
      });
      expect(response.status).toBe(403); expect(await response.json()).toMatchObject({ error: { code: "account_or_device_inactive" } });
      expect(await runInDurableObject(quota, async (_instance, state) => state.storage.sql.exec<{ frames: number; bytes: number }>("SELECT COALESCE(SUM(frames),0) AS frames,COALESCE(SUM(bytes),0) AS bytes FROM windows").one())).toEqual({ frames: 0, bytes: 0 });
    } finally { clock.mockRestore(); }
  });
  it.each([{ elapsed: 1000, expected: 200 }, { elapsed: LIMITS.leaseMs, expected: 403 }])("keeps the authority lookup lease deadline after $elapsed ms", async ({ elapsed, expected }) => {
    const owner = await identity(); const room = "opaque_lookup_deadline_01"; await ticket(owner, room);
    const wall = Date.now(); const started = Math.min(wall, windowBoundary(wall) - LIMITS.leaseMs - 10_000);
    const clock = vi.spyOn(Date, "now").mockReturnValue(started);
    try {
      const quota = bindings.ACCOUNT_QUOTA.get(bindings.ACCOUNT_QUOTA.idFromName(owner.accountId));
      const response = await runInDurableObject(quota, async (_instance, state) => {
        const delayed = new AccountQuota(state, { ...bindings, DB: afterDatabaseRead(bindings.DB, () => { clock.mockReturnValue(started + elapsed); }) });
        return delayed.fetch(new Request("https://internal/reserve", { method: "POST", body: JSON.stringify({ accountId: owner.accountId, deviceId: owner.deviceId, sessionId: owner.sessionId, room, role: "host" }) }));
      });
      expect(response.status).toBe(expected);
      if (expected === 200) expect((await response.json<Lease>()).expiresAt).toBe(started + LIMITS.leaseMs);
      else {
        await response.text();
        expect(await runInDurableObject(quota, async (_instance, state) => state.storage.sql.exec<{ frames: number; bytes: number }>("SELECT COALESCE(SUM(frames),0) AS frames,COALESCE(SUM(bytes),0) AS bytes FROM windows").one())).toEqual({ frames: 0, bytes: 0 });
      }
    } finally { clock.mockRestore(); }
  });
  it("bounds leases at midnight and denies renewed authority after device revocation", async () => {
    const beforeMidnight = Date.UTC(2026, 9, 18, 23, 59, 59);
    expect(leaseExpiry(beforeMidnight)).toBe(Date.UTC(2026, 9, 19));
    expect(leaseExpiry(beforeMidnight) - beforeMidnight).toBe(1000);
    expect(windowBoundary(Date.UTC(2026, 9, 19))).toBe(Date.UTC(2026, 9, 20));
    const owner = await identity(); const room = "opaque_fixture_room_0004"; await ticket(owner, room);
    await call(`/v1/devices/${owner.deviceId}/revoke`, "POST", {}, owner.token);
    const quota = bindings.ACCOUNT_QUOTA.get(bindings.ACCOUNT_QUOTA.idFromName(owner.accountId));
    expect((await quota.fetch("https://internal/reserve", { method: "POST", body: JSON.stringify({ accountId: owner.accountId, deviceId: owner.deviceId, sessionId: owner.sessionId, room, role: "host" }) })).status).toBe(403);
  });
  it("measures a bounded 90-frame workload without per-frame quota writes or payload storage", async () => {
    const owner = await identity(); const room = "opaque_load_room_000001";
    const host = await connect(room, "host", await ticket(owner, room));
    const controller = await connect(room, "controller", await ticket(owner, room, "controller"));
    host.webSocket!.accept(); controller.webSocket!.accept(); host.webSocket!.binaryType = "arraybuffer"; controller.webSocket!.binaryType = "arraybuffer";
    const quota = bindings.ACCOUNT_QUOTA.get(bindings.ACCOUNT_QUOTA.idFromName(owner.accountId));
    const ledger = () => runInDurableObject(quota, async (_instance, state) => ({ windows: state.storage.sql.exec("SELECT frames,bytes,controls FROM windows").toArray(), storageBytes: state.storage.sql.databaseSize }));
    const before = await ledger(); const durations: number[] = [];
    const frame = new Uint8Array(4096); frame.set(new TextEncoder().encode("SENTINEL_load_ciphertext_no_storage"));
    const started = performance.now();
    for (let index = 0; index < 90; index++) {
      const sender = index % 2 === 0 ? host.webSocket! : controller.webSocket!;
      const peer = index % 2 === 0 ? controller.webSocket! : host.webSocket!;
      const receiving = message(peer); const one = performance.now(); sender.send(frame);
      expect((await receiving as ArrayBuffer).byteLength).toBe(frame.byteLength); durations.push(performance.now() - one);
    }
    const elapsedMs = performance.now() - started; durations.sort((a, b) => a - b);
    expect(await ledger()).toEqual(before);
    expect(before.windows).toEqual([{ frames: 1024, bytes: 2097152, controls: 4 }]);
    const storage = await runInDurableObject(bindings.ROOM.get(bindings.ROOM.idFromName(room)), async (_instance, state) => ({ persisted: Array.from(await state.storage.list()), sqlBytes: state.storage.sql.databaseSize }));
    expect(storage.persisted).toEqual([]);
    console.info(JSON.stringify({ fixture: "relay_90_frames", frames: 90, bytes: 90 * frame.byteLength, elapsedMs, p50Ms: durations[45], p95Ms: durations[85], reserved: before.windows[0], quotaStorageBytes: before.storageBytes, roomSqlBytes: storage.sqlBytes }));
    host.webSocket!.close(); controller.webSocket!.close();
  });
  it("checks restored lease expiry on every frame and applies the combined burst limit", async () => {
    const owner = await identity(); const room = "opaque_limits_room_0001";
    const host = await connect(room, "host", await ticket(owner, room));
    const controller = await connect(room, "controller", await ticket(owner, room, "controller"));
    host.webSocket!.accept(); controller.webSocket!.accept();
    await runInDurableObject(bindings.ROOM.get(bindings.ROOM.idFromName(room)), async (_instance, state) => {
      const socket = state.getWebSockets("host")[0]!; const attachment = socket.deserializeAttachment() as Lease; attachment.expiresAt = Date.now() - 1; socket.serializeAttachment(attachment);
    });
    const closed = new Promise<CloseEvent>(resolve => host.webSocket!.addEventListener("close", resolve, { once: true }));
    host.webSocket!.send(new Uint8Array([1])); expect((await closed).reason).toBe("lease_expired");
    controller.webSocket!.close();
    const burstRoom = "opaque_limits_room_0002";
    expect((await call(`/v1/rooms/${room}/stop`, "POST", {}, owner.token)).status).toBe(200);
    const burstHost = await connect(burstRoom, "host", await ticket(owner, burstRoom)); const burstController = await connect(burstRoom, "controller", await ticket(owner, burstRoom, "controller"));
    burstHost.webSocket!.accept(); burstController.webSocket!.accept();
    await runInDurableObject(bindings.ROOM.get(bindings.ROOM.idFromName(burstRoom)), async (_instance, state) => {
      for (const socket of state.getWebSockets()) { const attachment = socket.deserializeAttachment() as Record<string, unknown>; attachment.burstSecond = Math.floor(Date.now() / 1000); attachment.burstCount = 100; socket.serializeAttachment(attachment); }
    });
    const limited = new Promise<CloseEvent>(resolve => burstHost.webSocket!.addEventListener("close", resolve, { once: true })); burstHost.webSocket!.send(new Uint8Array([1]));
    expect((await limited).reason).toBe("burst_limit"); burstController.webSocket!.close();
  });
  it("closes oversize and text frames without a transcript queue", async () => {
    const owner = await identity(); const room = "opaque_fixture_room_0005";
    const host = await connect(room, "host", await ticket(owner, room)); const controller = await connect(room, "controller", await ticket(owner, room, "controller"));
    host.webSocket!.accept(); controller.webSocket!.accept();
    const closed = new Promise<CloseEvent>(resolve => host.webSocket!.addEventListener("close", resolve, { once: true }));
    host.webSocket!.send(new Uint8Array(LIMITS.frameBytes + 1));
    expect((await closed).code).toBe(1009); controller.webSocket!.close();
  });
});

describe("protected browser control", () => {
  it("serves only fixed common assets publicly and never leaks controller files through aliases", async () => {
    const assets = vi.fn(async (request: Request) => new Response(new URL(request.url).pathname));
    const configured = { ...bindings, ASSETS: { fetch: assets } as unknown as Fetcher };
    for (const path of ["/common/relay.css", "/common/theme-boot.js", "/common/relay-logo-ink.svg", "/common/relay-logo-paper.svg", "/common/relay-mark-ink.svg", "/common/relay-mark-paper.svg", "/common/relay-favicon.svg", "/common/manrope-latin.woff2", "/common/ibm-plex-mono-500-latin.woff2", "/account.css"]) {
      const response = await call(path, "GET", undefined, undefined, undefined, configured);
      expect(response.status).toBe(200); expect(await response.text()).toBe(path === "/account.css" ? "/common/relay.css" : path);
      expect(response.headers.get("content-security-policy")).toContain("style-src 'self'");
    }
    assets.mockClear();
    for (const path of ["/common/app.js", "/common/../assets/app.js", "/common/ejm_relay_web_bg.wasm", "/common/cli-mark-ink.svg", "/common/relay/review.svg", "/common/relay/editable/logo-ink.svg", "/index.html", "/assets/protocol.js", "/control", "/control/assets/protocol.js"]) {
      const response = await call(path, "GET", undefined, undefined, undefined, configured);
      expect([401, 404]).toContain(response.status); await response.text();
    }
    expect(assets).not.toHaveBeenCalled();
    const mutation = await call("/common/relay.css", "POST", {}, undefined, undefined, configured);
    expect(mutation.status).toBe(405); await mutation.text(); expect(assets).not.toHaveBeenCalled();
  });
  it("authenticates every controller asset and keeps room metadata account scoped", async () => {
    const owner = await identity(); const other = await identity(); const browserCookie = await cookie(owner);
    const assets = vi.fn(async (request: Request) => new Response(new URL(request.url).pathname, { headers: { "content-type": "text/html" } }));
    const configured = { ...bindings, ASSETS: { fetch: assets } as unknown as Fetcher };
    for (const headers of [undefined, { accept: "application/json" }]) {
      const denied = await call("/control", "GET", undefined, undefined, headers, configured);
      expect(denied.status).toBe(401); expect(await denied.json()).toEqual({ error: { code: "sign_in_required" } });
    }
    const recovery = await call("/control", "GET", undefined, undefined, { accept: "text/html,application/xhtml+xml" }, configured);
    expect(recovery.status).toBe(401); expect(recovery.headers.get("content-type")).toContain("text/html");
    expect(recovery.headers.get("cache-control")).toBe("no-store");
    expect(recovery.headers.get("x-content-type-options")).toBe("nosniff");
    expect(recovery.headers.get("referrer-policy")).toBe("same-origin");
    expect(recovery.headers.get("content-security-policy")).toContain("style-src 'self'");
    expect(recovery.headers.get("content-security-policy")).not.toContain("unsafe-inline");
    const links: (string | null)[] = [], scripts: (string | null)[] = [];
    const navigation = new HTMLRewriter().on("main a", { element: element => { links.push(element.getAttribute("href")); } }).on("script", { element: element => { scripts.push(element.getAttribute("src")); } }).transform(recovery);
    await navigation.arrayBuffer();
    expect(links).toEqual(["/account"]); expect(scripts).toEqual(["/common/theme-boot.js"]);
    for (const headers of [undefined, { accept: "text/html" }]) {
      const denied = await call("/control/assets/app.js", "GET", undefined, undefined, headers, configured);
      expect(denied.status).toBe(401); expect(await denied.json()).toEqual({ error: { code: "sign_in_required" } });
    }
    expect(assets).not.toHaveBeenCalled();
    const index = await call("/control", "GET", undefined, undefined, { cookie: browserCookie }, configured);
    expect(index.status).toBe(200); expect(await index.text()).toBe("/index.html");
    expect(index.headers.get("cache-control")).toBe("no-store");
    expect(index.headers.get("content-security-policy")).toContain("'wasm-unsafe-eval'");
    expect(index.headers.get("content-security-policy")).toContain("wss://relay.example.invalid");
    const asset = await call("/control/assets/app.js", "GET", undefined, undefined, { cookie: browserCookie }, configured);
    expect(await asset.text()).toBe("/assets/app.js");
    const room = "opaque_browser_room_0001"; await ticket(owner, room);
    expect(await (await call("/v1/rooms", "GET", undefined, owner.token)).json()).toMatchObject({ rooms: [{ id: room }] });
    expect(await (await call("/v1/rooms", "GET", undefined, other.token)).json()).toEqual({ rooms: [] });
    expect((await call(`/v1/rooms/${room}/stop`, "POST", {}, undefined, { cookie: browserCookie, origin: "https://attacker.invalid" })).status).toBe(403);
    expect((await call(`/v1/rooms/${room}/stop`, "POST", {}, other.token)).status).toBe(403);
  });
  it("uses one-use header protocols with exact origin and the requesting browser session", async () => {
    const owner = await identity(); const other = await identity(); const browserCookie = await cookie(owner); const otherCookie = await cookie(other);
    const room = "opaque_browser_room_0002";
    const host = await connect(room, "host", await ticket(owner, room)); host.webSocket!.accept();
    const secret = await ticket(owner, room, "controller");
    const browser = (headers: HeadersInit, suffix = "") => call(`/v1/rooms/${room}/controller${suffix}`, "GET", undefined, undefined, { upgrade: "websocket", "sec-websocket-protocol": `ejm-relay-v1,ticket.${secret}`, ...headers });
    expect((await browser({ cookie: browserCookie })).status).toBe(403);
    expect((await browser({ cookie: browserCookie, origin: "https://attacker.invalid" })).status).toBe(403);
    expect((await browser({ cookie: otherCookie, origin })).status).toBe(403);
    expect((await browser({ cookie: browserCookie, origin }, `?ticket=${secret}`)).status).toBe(400);
    const context = await createAuth(bindings).$context;
    const recovered = await context.internalAdapter.createSession(owner.accountId);
    const recoveryCookie = (await serializeSignedCookie(context.authCookies.sessionToken.name, recovered.token, context.secret)).split(";")[0]!;
    expect((await browser({ cookie: recoveryCookie, origin })).status).toBe(401);
    const registered = await call("/v1/devices", "POST", { label: "Recovered browser" }, recovered.token); expect(registered.status).toBe(201);
    expect((await browser({ cookie: recoveryCookie, origin })).status).toBe(403);
    const controller = await browser({ cookie: browserCookie, origin, "sec-gpc": "1" }); expect(controller.status).toBe(101);
    expect(controller.headers.get("set-cookie")).toContain("HttpOnly");
    expect(await (await call("/api/privacy", "GET", undefined, owner.token)).json()).toMatchObject({ optedOut: true });
    expect(controller.headers.get("sec-websocket-protocol")).toBe("ejm-relay-v1");
    expect([...controller.headers].join(" ")).not.toContain(secret);
    controller.webSocket!.accept(); controller.webSocket!.binaryType = "arraybuffer";
    const receiving = message(controller.webSocket!); host.webSocket!.send(new Uint8Array([4, 8, 12]));
    expect(new Uint8Array(await receiving as ArrayBuffer)).toEqual(new Uint8Array([4, 8, 12]));
    expect((await browser({ cookie: browserCookie, origin })).status).toBe(401);
    host.webSocket!.close(); controller.webSocket!.close();
  });
  it("closes the opposite peer before any fresh transport can reuse an old cipher", async () => {
    const owner = await identity(); const room = "opaque_browser_room_0003";
    const host = await connect(room, "host", await ticket(owner, room));
    const controller = await connect(room, "controller", await ticket(owner, room, "controller"));
    host.webSocket!.accept(); controller.webSocket!.accept();
    const received = message(controller.webSocket!); host.webSocket!.send(new Uint8Array([1])); await received;
    const closed = new Promise<CloseEvent>(resolve => controller.webSocket!.addEventListener("close", resolve, { once: true }));
    host.webSocket!.close(1000, "fixture_disconnect");
    expect((await closed).reason).toBe("connection_closed");
  });
});

describe("payment authority", () => {
  it("requires configured allowed plans and ignores checkout redirects", async () => {
    const owner = await identity();
    expect((await call("/v1/billing/checkout", "POST", { plan: "monthly" }, owner.token, undefined, { ...bindings, STRIPE_SECRET_KEY: "" })).status).toBe(503);
    expect((await call("/account?checkout=returned", "GET", undefined, owner.token)).status).toBe(200);
    expect(await bindings.DB.prepare("SELECT * FROM relay_billing").all()).toMatchObject({ results: [] });
    expect((await call("/v1/billing/webhook", "POST", {}, undefined, { "stripe-signature": "forged" }, { ...bindings, STRIPE_SECRET_KEY: "" })).status).toBe(503);
  });
  it("derives absolute expiry from current subscriptions and rejects payment failure/unknown prices", () => {
    const now = Date.now(); const until = Math.floor(now / 1000) + 3600;
    const subscription = { status: "active", cancel_at_period_end: true, latest_invoice: "in_fixture", items: { data: [{ price: { id: "price_allowed" }, current_period_end: until }] } };
    expect(subscriptionEntitlement(subscription, ["price_allowed"], now)).toMatchObject({ active: true, until: until * 1000, cancelAtPeriodEnd: true });
    for (const status of ["past_due", "unpaid", "canceled", "incomplete", "trialing"]) expect(subscriptionEntitlement({ ...subscription, status }, ["price_allowed"], now).active).toBe(false);
    expect(subscriptionEntitlement(subscription, ["price_attacker"], now).active).toBe(false);
    expect(subscriptionEntitlement(subscription, ["price_allowed"], until * 1000).active).toBe(false);
  });
});
