import { env } from "cloudflare:workers";
import { applyD1Migrations, evictDurableObject, reset, runInDurableObject } from "cloudflare:test";
import { beforeEach, expect, it, vi } from "vitest";
import worker from "../../src/index.js";
import { createAuth } from "../../src/auth.js";
import { operationalCleanup } from "../../src/cleanup.js";
import type { Env } from "../../src/types.js";

const bindings = env as unknown as Env & { TEST_MIGRATIONS: { name: string; queries: string[] }[] };
const origin = bindings.RELAY_ORIGIN;
const privacy = () => bindings.PRIVACY_AUTHORITY.get(bindings.PRIVACY_AUTHORITY.idFromName("privacy-v1"));
beforeEach(async () => { vi.restoreAllMocks(); await reset(); await applyD1Migrations(bindings.DB, bindings.TEST_MIGRATIONS); });

function call(path: string, token?: string, body?: unknown, config: Env = bindings): Promise<Response> {
  const headers = new Headers({ origin });
  if (token) headers.set("authorization", `Bearer ${token}`);
  if (body !== undefined) headers.set("content-type", "application/json");
  return worker.fetch(new Request(origin + path, { method: body === undefined ? "GET" : "POST", headers, ...(body === undefined ? {} : { body: JSON.stringify(body) }) }), config);
}
async function owner(): Promise<{ id: string; token: string; deviceId: string }> {
  const context = await createAuth(bindings).$context;
  const user = await context.internalAdapter.createUser({ email: "close-fixture@example.invalid", name: "Fixture", emailVerified: true }, { method: "api" });
  const session = await context.internalAdapter.createSession(user.id);
  await context.internalAdapter.createAccount({ userId: user.id, providerId: "github", accountId: "fixture-provider-identity" });
  expect((await call("/v1/account/terms", session.token, { version: bindings.TERMS_VERSION })).status).toBe(200);
  const device = await call("/v1/devices", session.token, { label: "Fixture device", osFamily: "linux", cliMajor: 1 });
  expect(device.status).toBe(201);
  return { id: user.id, token: session.token, deviceId: (await device.json<{ id: string }>()).id };
}
async function tracked(requestId: string, statusKey: string): Promise<Record<string, unknown>> {
  const response = await call("/api/privacy/rights/status", undefined, { requestId, statusKey });
  expect(response.status).toBe(200); return response.json();
}

it("tracks dashboard closure and prior recipients before removing session authority", async () => {
  const account = await owner();
  await call("/api/privacy", account.token);
  await runInDurableObject(privacy(), async (_instance, state) => {
    state.storage.sql.exec("INSERT INTO privacy_ledger(id,manifest_id,buyer_id,purpose,category,fields,notice_version,subjects,digest,released_at,expires_at) VALUES('disclosed','manifest','approved-buyer','Fixture purpose','profile','[\"role\"]','fixture',?,'fixture-digest',?,?)", JSON.stringify([account.id]), Date.now(), Date.now() + 86_400_000);
  });
  const response = await call("/v1/account/close", account.token, { confirm: "close" });
  expect(response.status).toBe(200);
  const record = await response.json<{ requestId: string; statusKey: string; closed: boolean; relayClosureAcknowledged: boolean }>();
  expect(record.requestId).toEqual(expect.any(String)); expect(record.statusKey).toMatch(/^[a-f0-9]{64}$/u);
  expect(record).toMatchObject({ closed: true, relayClosureAcknowledged: true });
  expect(await bindings.DB.prepare("SELECT account_id,action,state FROM privacy_operational_tasks WHERE request_id=?").bind(record.requestId).first()).toEqual({ account_id: account.id, action: "close", state: "pending" });
  expect(await tracked(record.requestId, record.statusKey)).toMatchObject({ id: record.requestId, kind: "close", operational_state: "pending", recipients: [{ disclosureId: "disclosed", buyerId: "approved-buyer", state: "pending" }] });
  expect((await call("/v1/account", account.token)).status).toBe(401);
  const stored = await runInDurableObject(privacy(), async (_instance, state) => state.storage.sql.exec("SELECT status_token_hash FROM privacy_rights WHERE id=?", record.requestId).toArray());
  expect(JSON.stringify(stored)).not.toContain(record.statusKey);
});

it("returns a private receipt when revocation fails and finishes the durable task on retry", async () => {
  const account = await owner(); const namespace = bindings.ACCOUNT_QUOTA; let fail = true;
  const config = { ...bindings, ACCOUNT_QUOTA: { idFromName: (name: string) => namespace.idFromName(name), get: (id: DurableObjectId) => ({ fetch: async (input: RequestInfo, init?: RequestInit) => {
    const request = new Request(input, init);
    if (new URL(request.url).pathname === "/revoke" && fail) { fail = false; return Response.json({ error: "fixture_unavailable" }, { status: 503 }); }
    return namespace.get(id).fetch(request);
  } }) } as unknown as DurableObjectNamespace };
  const response = await call("/v1/account/close", account.token, { confirm: "close" }, config);
  expect(response.status).toBe(503);
  const receipt = await response.json<{ requestId: string; statusKey: string }>();
  expect(receipt).toMatchObject({ error: { code: "relay_revocation_pending" }, relayClosureAcknowledged: false });
  expect(receipt.statusKey).toMatch(/^[a-f0-9]{64}$/u);
  expect(await tracked(receipt.requestId, receipt.statusKey)).toMatchObject({ operational_state: "pending" });
  expect((await call("/v1/account", account.token)).status).toBe(401);
  await operationalCleanup(config);
  expect(await tracked(receipt.requestId, receipt.statusKey)).toMatchObject({ operational_state: "exception" });
  expect(await bindings.DB.prepare("SELECT state FROM privacy_operational_tasks WHERE request_id=?").bind(receipt.requestId).first()).toEqual({ state: "exception" });
});

it("retains a Room closure target after quota revocation and retries before acknowledging rights completion", async () => {
  const account = await owner(); const room = "opaque_rights_revoke_retry_01";
  async function connect(role: string): Promise<WebSocket> {
    const issued = await call(`/v1/rooms/${room}/tickets`, account.token, { role, deviceId: account.deviceId });
    expect(issued.status).toBe(200); const ticket = await issued.json<{ ticket: string }>();
    const response = await worker.fetch(new Request(`${origin}/v1/rooms/${room}/${role}`, { headers: { authorization: `Bearer ${ticket.ticket}`, upgrade: "websocket" } }), bindings);
    expect(response.status).toBe(101); response.webSocket!.accept(); return response.webSocket!;
  }
  const host = await connect("host"); const controller = await connect("controller");
  const closed = [host, controller].map(socket => new Promise<CloseEvent>(resolve => socket.addEventListener("close", resolve, { once: true })));
  const namespace = bindings.ROOM; let attempts = 0;
  const config = { ...bindings, ROOM: { idFromName: (name: string) => namespace.idFromName(name), get: (id: DurableObjectId) => ({ fetch: async (input: RequestInfo, init?: RequestInit) => {
    const request = new Request(input, init);
    if (new URL(request.url).pathname === "/close" && ++attempts <= 2) return Response.json({ error: "fixture_room_unavailable" }, { status: 503 });
    return namespace.get(id).fetch(request);
  } }) } as unknown as DurableObjectNamespace };
  const quota = bindings.ACCOUNT_QUOTA.get(bindings.ACCOUNT_QUOTA.idFromName(account.id));
  try {
    const response = await call("/v1/account/close", account.token, { confirm: "close" }, config);
    expect(response.status).toBe(503);
    const receipt = await response.json<{ requestId: string; statusKey: string }>();
    expect(receipt).toMatchObject({ error: { code: "relay_revocation_pending" }, relayClosureAcknowledged: false });
    expect((await call("/v1/account", account.token)).status).toBe(401);
    await evictDurableObject(quota);
    await operationalCleanup(config);
    expect(attempts).toBe(2);
    expect(await tracked(receipt.requestId, receipt.statusKey)).toMatchObject({ operational_state: "pending" });
    const retry = await bindings.DB.prepare("SELECT json_extract(result,'$.retryAfter') AS retry_at FROM privacy_operational_tasks WHERE request_id=?").bind(receipt.requestId).first<{ retry_at: number }>();
    vi.spyOn(Date, "now").mockReturnValue(retry!.retry_at + 1);
    await operationalCleanup(config);
    expect(attempts).toBe(3);
    expect((await Promise.all(closed)).every(event => event.code === 1000)).toBe(true);
    expect(await tracked(receipt.requestId, receipt.statusKey)).toMatchObject({ operational_state: "exception" });
    expect(await runInDurableObject(quota, async (_instance, state) => ({ current: state.storage.sql.exec("SELECT * FROM room").toArray(), pending: state.storage.sql.exec("SELECT * FROM pending_close").toArray() }))).toEqual({ current: [], pending: [] });
  } finally { vi.restoreAllMocks(); host.close(); controller.close(); }
});

it("keeps account access and a readable form error when tracked closure cannot be admitted", async () => {
  const account = await owner(); await call("/api/privacy", account.token);
  await runInDurableObject(privacy(), async (_instance, state) => {
    for (let i = 0; i < 32; i += 1) state.storage.sql.exec("INSERT INTO privacy_rights(id,account_id,status_token_hash,kind,jurisdiction,state,requested_at,acknowledgement_due,response_due,expires_at) VALUES(?,?,?,'access','fixture','acknowledged',?,?,?,?)", `pending-${i}`, account.id, "fixture-private-status-hash", Date.now(), Date.now(), Date.now(), Date.now() + 86_400_000);
  });
  const response = await worker.fetch(new Request(origin + "/v1/account/close", { method: "POST", headers: { origin, authorization: `Bearer ${account.token}`, "content-type": "application/x-www-form-urlencoded" }, body: "confirm=close" }), bindings);
  expect(response.status).toBe(429); expect(response.headers.get("content-type")).toContain("text/html");
  const html = await response.text(); expect(html).toContain("rights_request_limit"); expect(html).not.toContain("[object Object]");
  expect((await call("/v1/account", account.token)).status).toBe(200);
  expect(await bindings.DB.prepare("SELECT status FROM relay_accounts WHERE id=?").bind(account.id).first()).toEqual({ status: "active" });
});

it.each([true, false])("respects live operational holds during rights deletion and revokes access (live=%s)", async held => {
  const account = await owner(); const now = Date.now();
  const response = await call("/api/privacy/rights/request", account.token, { kind: "delete", jurisdiction: "fixture" });
  expect(response.status).toBe(200); const receipt = await response.json<{ requestId: string; statusKey: string }>();
  const expiresAt = now + (held ? 86_400_000 : -1);
  await bindings.DB.prepare("INSERT INTO operational_legal_holds(id,account_id,category,reason,evidence,expires_at) VALUES('retained-identity',?,'payment','Fixture legal claim','Fixture reviewed evidence',?)").bind(account.id, expiresAt).run();
  await operationalCleanup(bindings);
  expect((await call("/v1/account", account.token)).status).toBe(401);
  expect(await bindings.DB.prepare("SELECT status FROM relay_accounts WHERE id=?").bind(account.id).first()).toEqual({ status: "closed" });
  const device = await bindings.DB.prepare("SELECT label,os_family,cli_major,revoked_at FROM relay_devices WHERE id=?").bind(account.deviceId).first();
  expect(device).toMatchObject(held ? { label: "Fixture device", os_family: "linux", cli_major: 1, revoked_at: expect.any(Number) } : { label: "Deleted device", os_family: null, cli_major: null, revoked_at: expect.any(Number) });
  const identity = await bindings.DB.prepare('SELECT email,name FROM "user" WHERE id=?').bind(account.id).first();
  expect(identity).toEqual(held ? { email: "close-fixture@example.invalid", name: "Fixture" } : { email: `${account.id}@deleted.invalid`, name: "Deleted account" });
  expect(await bindings.DB.prepare('SELECT "accountId" FROM "account" WHERE "userId"=?').bind(account.id).first()).toEqual(held ? { accountId: "fixture-provider-identity" } : null);
  const record = await tracked(receipt.requestId, receipt.statusKey);
  expect(record.operational_state).toBe("exception");
  const result = record.operational_result as Record<string, unknown>;
  if (held) {
    expect(result.legalHold).toEqual({ expiresAt, retained: ["Provider identity and credentials", "Operational email and profile", "Device labels and metadata"] });
    expect(result.deleted).toEqual(["Session credentials", "Connection tickets"]);
  } else expect(result.legalHold).toBeUndefined();
});
