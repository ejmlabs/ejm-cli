import { env } from "cloudflare:workers";
import { applyD1Migrations, evictDurableObject, reset, runInDurableObject } from "cloudflare:test";
import { beforeEach, expect, it, vi } from "vitest";
import worker from "../src/index.js";
import { operationalCleanup } from "../src/cleanup.js";
import { privacyCleanup, privacyRoute } from "../src/privacy/index.js";
import type { Env } from "../src/types.js";

const bindings = env as unknown as Env & { TEST_MIGRATIONS: { name: string; queries: string[] }[] };
const DAY = 86_400_000;
const HOUR = 3_600_000;
const privacy = () => bindings.PRIVACY_AUTHORITY.get(bindings.PRIVACY_AUTHORITY.idFromName("privacy-v1"));
beforeEach(async () => { vi.restoreAllMocks(); await reset(); await applyD1Migrations(bindings.DB, bindings.TEST_MIGRATIONS); });

interface CleanupFaults { failedInsert?: string; failedAck?: string; lostAck?: string; failedProgress?: string; delayedAck?: { requestId: string; started: () => void; released: Promise<void> } }
function observedEnv(options: CleanupFaults = {}) {
  const counts = { queries: 0, calls: 0 };
  const originals = new WeakMap<D1PreparedStatement, D1PreparedStatement>();
  function statement(raw: D1PreparedStatement, sql: string, values: unknown[] = []): D1PreparedStatement {
    const wrapped = new Proxy(raw, { get(target, key) {
      if (key === "bind") return (...args: unknown[]) => statement(target.bind(...args), sql, args);
      if (["run", "first", "all", "raw"].includes(String(key))) return (...args: unknown[]) => {
        counts.queries += 1;
        if (sql.startsWith("INSERT OR IGNORE INTO privacy_operational_tasks") && values[0] === options.failedInsert) {
          delete options.failedInsert; throw new Error("fixture_enqueue_unavailable");
        }
        if (key === "run" && sql.startsWith("UPDATE privacy_operational_tasks") && options.failedProgress && values.includes(options.failedProgress)) {
          delete options.failedProgress; throw new Error("fixture_progress_unavailable");
        }
        return Reflect.apply(Reflect.get(target, key) as (...args: unknown[]) => unknown, target, args);
      };
      const value: unknown = Reflect.get(target, key); return typeof value === "function" ? value.bind(target) : value;
    } });
    originals.set(wrapped, raw); return wrapped;
  }
  const database = new Proxy(bindings.DB, { get(target, key) {
    if (key === "prepare") return (sql: string) => statement(target.prepare(sql), sql);
    if (key === "batch") return (statements: D1PreparedStatement[]) => { counts.queries += statements.length; return target.batch(statements.map(value => originals.get(value) ?? value)); };
    const value: unknown = Reflect.get(target, key); return typeof value === "function" ? value.bind(target) : value;
  } });
  function namespace(raw: DurableObjectNamespace): DurableObjectNamespace {
    return new Proxy(raw, { get(target, key) {
      if (key === "get") return (id: DurableObjectId) => {
        const stub = target.get(id);
        return new Proxy(stub, { get(object, field) {
          if (field === "fetch") return async (input: RequestInfo, init?: RequestInit) => {
            counts.calls += 1;
            const request = new Request(input, init); const path = new URL(request.url).pathname;
            // No live Stripe I/O. Keep cancellation pending, as in the free deployment.
            if (path === "/billing/cancel") return Response.json({ error: "billing_not_configured" }, { status: 503 });
            if (path === "/internal/operational" && options.failedAck) {
              const body = await request.clone().json<{ requestId: string }>();
              if (body.requestId === options.failedAck) { delete options.failedAck; return Response.json({ error: "fixture_ack_unavailable" }, { status: 503 }); }
            }
            const requestId = path === "/internal/operational" && (options.lostAck || options.delayedAck) ? (await request.clone().json<{ requestId: string }>()).requestId : null;
            const delayed = options.delayedAck;
            const response = await object.fetch(request);
            if (requestId === options.lostAck) { delete options.lostAck; await response.arrayBuffer(); return Response.json({ error: "fixture_reply_lost" }, { status: 503 }); }
            if (delayed && requestId === delayed.requestId) { delete options.delayedAck; delayed.started(); await delayed.released; }
            return response;
          };
          const value: unknown = Reflect.get(object, field); return typeof value === "function" ? value.bind(object) : value;
        } });
      };
      const value: unknown = Reflect.get(target, key); return typeof value === "function" ? value.bind(target) : value;
    } });
  }
  return { counts, bindings: { ...bindings, DB: database, ACCOUNT_QUOTA: namespace(bindings.ACCOUNT_QUOTA), ROOM: namespace(bindings.ROOM), PRIVACY_AUTHORITY: namespace(bindings.PRIVACY_AUTHORITY) } };
}
async function account(id: string, created: number, closed = false): Promise<void> {
  const date = new Date(created).toISOString();
  await bindings.DB.batch([
    bindings.DB.prepare(`INSERT INTO "user"(id,name,email,"emailVerified","createdAt","updatedAt") VALUES(?,'Fixture',?,1,?,?)`).bind(id, `${id}@example.invalid`, date, date),
    bindings.DB.prepare("INSERT INTO relay_accounts(id,status,terms_version,terms_at,email_provenance,created_at,closed_at,cleanup_after,quota_cleanup_at) VALUES(?,?,?,?,'github_verified_primary',?,?,?,?)").bind(id, closed ? "closed" : "active", bindings.TERMS_VERSION, created, created, closed ? created : null, closed ? created : null, closed ? Date.now() : null),
    bindings.DB.prepare("INSERT INTO relay_devices(id,account_id,session_id,label,created_at,last_used_at) VALUES(?,?,?,'Fixture device',?,?)").bind(`device-${id}`, id, `session-${id}`, created, created),
    bindings.DB.prepare("INSERT INTO relay_billing(account_id,customer_id,plan,status,paid_until,cancel_at_period_end,updated_at) VALUES(?,?,'free','canceled',0,1,?)").bind(id, `cus_${id}`, created),
  ]);
}
async function rights(count: number, accountIds: string[], now: number, closeFirst = false): Promise<void> {
  await runInDurableObject(privacy(), async (_instance, state) => {
    for (let i = 0; i < count; i += 1) state.storage.sql.exec("INSERT INTO privacy_rights(id,account_id,status_token_hash,kind,jurisdiction,state,requested_at,acknowledgement_due,response_due,expires_at) VALUES(?,?,?,?,'fixture','acknowledged',?,?,?,?)", `right-${String(i).padStart(3, "0")}`, accountIds[i % accountIds.length]!, "fixture-private-status-hash", closeFirst && i === 0 ? "close" : "export", now - HOUR + i, now + DAY, now + 30 * DAY, now + 730 * DAY);
  });
}
async function queue(count: number, accountIds: string[], now: number, closeFirst = false): Promise<void> {
  await bindings.DB.batch(Array.from({ length: count }, (_, i) => bindings.DB.prepare("INSERT INTO privacy_operational_tasks(request_id,account_id,action,requested_at,due_at,expires_at) VALUES(?,?,?,?,?,?)").bind(`right-${String(i).padStart(3, "0")}`, accountIds[i % accountIds.length]!, closeFirst && i === 0 ? "close" : "export", now - HOUR + i, now + 30 * DAY, now + 30 * DAY)));
}

it("delivers all device-history parts across lost replies, failed progress writes and eviction", async () => {
  let now = Date.now(); const id = "history-export-fixture"; const created = now - DAY;
  await account(id, created);
  const history = Array.from({ length: 128 }, (_, i) => `history-${String(i).padStart(3, "0")}`);
  await bindings.DB.batch(history.map((device, i) => bindings.DB.prepare("INSERT INTO relay_devices(id,account_id,session_id,label,created_at,last_used_at,revoked_at,os_family,cli_major) VALUES(?,?,?,?,?,?,?,'linux',1)").bind(device, id, `session-${device}`, `Device ${i} ${"界".repeat(40)}`, created, created, i < 7 ? null : created + HOUR)));
  await account("other-export-fixture", created);
  const headers = { origin: bindings.RELAY_ORIGIN, "content-type": "application/json" };
  const request = (path: string, body: unknown) => new Request(bindings.RELAY_ORIGIN + path, { method: "POST", headers, body: JSON.stringify(body) });
  const started = await privacyRoute(request("/api/privacy/rights/request", { kind: "export", jurisdiction: "fixture" }), bindings, { accountId: id, sessionId: "fixture-session", deviceId: null, email: `${id}@example.invalid`, termsVersion: bindings.TERMS_VERSION });
  expect(started?.status).toBe(200);
  const { requestId, statusKey } = await started!.json<{ requestId: string; statusKey: string }>(); const receipt = { requestId, statusKey };
  type ExportStatus = { operational_state: string; part: number; nextPart: number | null; operational_result: { devices: { id: string; label: string }[]; deviceExport: { partCount: number; deviceCount: number; complete: boolean } } };
  async function tracked(part?: number): Promise<ExportStatus> {
    const response = await privacyRoute(request("/api/privacy/rights/status", { ...receipt, ...(part === undefined ? {} : { part }) }), bindings, null);
    expect(response?.status).toBe(200); expect(response?.headers.get("cache-control")).toBe("no-store");
    return response!.json<ExportStatus>();
  }
  const taskState = () => bindings.DB.prepare("SELECT state FROM privacy_operational_tasks WHERE request_id=?").bind(receipt.requestId).first();
  const faults: CleanupFaults = {}; const observed = observedEnv(faults);
  const clock = vi.spyOn(Date, "now").mockImplementation(() => now);
  async function nextAttempt(): Promise<void> { now += HOUR + 1; await operationalCleanup(observed.bindings); }

  await operationalCleanup(observed.bindings);
  // This public state assertion fails on the truncated old implementation before any part API or new table is used.
  expect(await taskState()).toEqual({ state: "pending" });
  expect((await tracked()).operational_state).toBe("pending");
  const first = await tracked(0); expect(first.operational_result.devices).toHaveLength(64); expect(first.nextPart).toBe(1);
  await bindings.DB.prepare("INSERT INTO relay_devices(id,account_id,session_id,label,created_at,last_used_at) VALUES('after-watermark',?,'after-watermark-session','Later device',?,?)").bind(id, now, now).run();
  faults.lostAck = receipt.requestId;
  await nextAttempt();
  expect(faults.lostAck).toBeUndefined(); expect(await taskState()).toEqual({ state: "pending" });
  await evictDurableObject(privacy());
  const persisted = await tracked(1); expect(persisted.operational_result.devices).toHaveLength(64); expect(persisted.operational_state).toBe("pending");
  await bindings.DB.prepare("UPDATE relay_devices SET label='Changed after durable part' WHERE account_id=? AND id=?").bind(id, persisted.operational_result.devices[0]!.id).run();
  let release!: () => void; let markHeld!: () => void;
  const held = new Promise<void>(resolve => { markHeld = resolve; }); const released = new Promise<void>(resolve => { release = resolve; });
  faults.delayedAck = { requestId: receipt.requestId, started: markHeld, released };
  const lateAttempt = nextAttempt(); await held;
  try { await nextAttempt(); } finally { release(); await lateAttempt; }
  expect((await tracked(1)).operational_result.devices).toEqual(persisted.operational_result.devices);
  expect(await taskState()).toEqual({ state: "pending" });

  faults.failedProgress = receipt.requestId;
  await nextAttempt();
  expect(faults.failedProgress).toBeUndefined(); expect(await taskState()).toEqual({ state: "pending" });
  expect((await tracked(2)).operational_state).toBe("completed");
  await evictDurableObject(privacy());
  await nextAttempt();
  expect(await taskState()).toEqual({ state: "completed" });
  const parts = await Promise.all([0, 1, 2].map(tracked));
  expect(parts.map(part => part.part)).toEqual([0, 1, 2]); expect(parts.map(part => part.nextPart)).toEqual([1, 2, null]);
  const records = parts.flatMap(part => part.operational_result.devices);
  expect(records.map(device => device.id)).toEqual([`device-${id}`, ...history].sort());
  expect(new Set(records.map(device => device.id)).size).toBe(129);
  expect(parts[2]!.operational_result.deviceExport).toMatchObject({ partCount: 3, deviceCount: 129, complete: true });
  expect(parts.every(part => part.operational_state === "completed")).toBe(true);
  const sizes = parts.map(part => new TextEncoder().encode(JSON.stringify(part.operational_result)).byteLength);
  expect(Math.max(...sizes)).toBeLessThan(48 * 1024);
  const storedSizes = await runInDurableObject(privacy(), async (_instance, state) => state.storage.sql.exec<{ bytes: number }>("SELECT length(CAST(payload AS BLOB)) AS bytes FROM privacy_operational_parts WHERE request_id=? ORDER BY part", receipt.requestId).toArray());
  expect(storedSizes).toHaveLength(3); expect(Math.max(...storedSizes.map(part => part.bytes))).toBeLessThan(48 * 1024);
  console.info({ exportDeviceRows: records.length, exportParts: parts.length, largestPublicPartBytes: Math.max(...sizes), largestStoredPartBytes: Math.max(...storedSizes.map(part => part.bytes)) });
  const denied = await privacyRoute(request("/api/privacy/rights/status", { ...receipt, statusKey: "0".repeat(64), part: 1 }), bindings, null);
  expect(denied?.status).toBe(404); expect(await denied?.json()).toEqual({ error: "request_not_found" });
  const unavailable = await privacyRoute(request("/api/privacy/rights/status", { ...receipt, part: 3 }), bindings, null);
  expect(unavailable?.status).toBe(404); await unavailable?.arrayBuffer();
  clock.mockRestore();
});

it.each([true, false])("bounds the populated scheduled path and leaves excess work durable (closure=%s)", async closeFirst => {
  const now = Date.now();
  const ids = Array.from({ length: 30 }, (_, i) => `account-${String(i).padStart(3, "0")}-fixture`);
  for (const [i, id] of ids.entries()) await account(id, now - DAY + i);
  await account("expired-retention-fixture", now - 400 * DAY, true);
  await account("unadmitted-fixture", now - 400 * DAY);
  await bindings.DB.prepare("DELETE FROM relay_devices WHERE account_id='unadmitted-fixture'").run();
  await bindings.DB.prepare("DELETE FROM relay_billing WHERE account_id='unadmitted-fixture'").run();
  await bindings.DB.prepare("DELETE FROM relay_accounts WHERE id='unadmitted-fixture'").run();
  await rights(128, ids, now, closeFirst); await queue(20, ids, now, closeFirst);
  await bindings.DB.batch(ids.slice(0, 20).map((id, i) => bindings.DB.prepare("INSERT INTO relay_billing_cancellations(account_id,state,created_at,updated_at) VALUES(?,'pending',?,?)").bind(id, now - DAY + i, now - DAY + i)));
  const room = "room-for-cleanup-fixture";
  await runInDurableObject(bindings.ACCOUNT_QUOTA.get(bindings.ACCOUNT_QUOTA.idFromName(ids[0]!)), async (_instance, state) => { state.storage.sql.exec("INSERT INTO room(singleton,room,expires_at) VALUES(1,?,?)", room, now + HOUR); });
  const expired = new Date(now - DAY).toISOString();
  await bindings.DB.batch([
    bindings.DB.prepare("INSERT INTO connection_tickets(hash,account_id,room,role,expires_at) VALUES('expired-ticket',?,'expired-room','host',?)").bind(ids[0], now - 1),
    bindings.DB.prepare("INSERT INTO relay_billing_events(id,account_id,status,created_at,expires_at) VALUES('expired-event',?,'processed',?,?)").bind(ids[0], now - DAY, now - 1),
    bindings.DB.prepare(`INSERT INTO "session"(id,"expiresAt",token,"createdAt","updatedAt","userId") VALUES('expired-session',?,'expired-token',?,?,?)`).bind(expired, expired, expired, ids[0]),
    bindings.DB.prepare(`INSERT INTO "verification"(id,identifier,value,"expiresAt","createdAt","updatedAt") VALUES('expired-verification','fixture','expired-value',?,?,?)`).bind(expired, expired, expired),
    bindings.DB.prepare(`INSERT INTO "deviceCode"(id,"deviceCode","userCode","expiresAt",status) VALUES('expired-device-code','expired-device-code','expired-user-code',?,'pending')`).bind(expired),
    bindings.DB.prepare(`INSERT INTO "rateLimit"(id,key,count,"lastRequest") VALUES('expired-rate','fixture',1,?)`).bind(now - 8 * DAY),
    bindings.DB.prepare("INSERT INTO operational_legal_holds(id,account_id,category,reason,evidence,expires_at) VALUES('expired-hold',?,'payment','Fixture reason','Fixture evidence',?)").bind(ids[0], now - 1),
    bindings.DB.prepare("INSERT INTO privacy_operational_tasks(request_id,account_id,action,state,requested_at,due_at,expires_at) VALUES('expired-right',?,'export','completed',?,?,?)").bind(ids[0], now - 60 * DAY, now - 30 * DAY, now - 1),
  ]);
  const observed = observedEnv();
  await worker.scheduled({} as ScheduledController, observed.bindings);
  // Count every executed batch member and internal fetch, including both cron phases.
  expect(observed.counts.queries).toBeLessThanOrEqual(38);
  expect(observed.counts.calls).toBeLessThanOrEqual(11);
  expect(observed.counts.queries + observed.counts.calls).toBeLessThan(50);
  console.info({ scheduledD1Queries: observed.counts.queries, scheduledInternalCalls: observed.counts.calls });
  expect(await bindings.DB.prepare("SELECT state FROM privacy_operational_tasks WHERE request_id='right-000'").first()).toEqual({ state: closeFirst ? "exception" : "completed" });
  expect(await bindings.DB.prepare("SELECT COUNT(*) AS n FROM privacy_operational_tasks WHERE state='pending'").first()).toEqual({ n: 19 });
  expect(await bindings.DB.prepare("SELECT COUNT(*) AS n FROM relay_billing_cancellations WHERE state='pending'").first()).toEqual({ n: 20 });
  expect(await bindings.DB.prepare("SELECT COUNT(*) AS n FROM relay_accounts WHERE quota_cleanup_at IS NOT NULL").first()).toEqual({ n: 2 });
  expect(await bindings.DB.prepare('SELECT id FROM "user" WHERE id IN(\'expired-retention-fixture\',\'unadmitted-fixture\')').all()).toMatchObject({ results: [] });
  const pending = await runInDurableObject(privacy(), async (_instance, state) => state.storage.sql.exec("SELECT COUNT(*) AS n FROM privacy_rights WHERE operational_state='pending'").one());
  expect(pending.n).toBe(127);
});

it("drains beyond failed enqueues and acknowledgements across authority eviction, then retries", async () => {
  const now = Date.now(); const id = "rights-drain-account-fixture";
  await account(id, now - DAY); await rights(6, [id], now);
  const observed = observedEnv({ failedInsert: "right-000", failedAck: "right-000" });
  await privacyCleanup(observed.bindings);
  expect(await bindings.DB.prepare("SELECT COUNT(*) AS n FROM privacy_operational_tasks").first()).toEqual({ n: 1 });
  await evictDurableObject(privacy());
  await privacyCleanup(observed.bindings); await privacyCleanup(observed.bindings);
  expect(await bindings.DB.prepare("SELECT COUNT(*) AS n FROM privacy_operational_tasks").first()).toEqual({ n: 5 });
  await privacyCleanup(observed.bindings);
  expect(await bindings.DB.prepare("SELECT COUNT(*) AS n FROM privacy_operational_tasks").first()).toEqual({ n: 6 });
  await operationalCleanup(observed.bindings); // Failed acknowledgement stays pending.
  expect(await bindings.DB.prepare("SELECT state FROM privacy_operational_tasks WHERE request_id='right-000'").first()).toEqual({ state: "pending" });
  for (let i = 0; i < 5; i += 1) await operationalCleanup(observed.bindings);
  expect(await bindings.DB.prepare("SELECT COUNT(*) AS n FROM privacy_operational_tasks WHERE state='completed'").first()).toEqual({ n: 5 });
  await evictDurableObject(privacy());
  const retry = await bindings.DB.prepare("SELECT json_extract(result,'$.retryAfter') AS retry_at FROM privacy_operational_tasks WHERE request_id='right-000'").first<{ retry_at: number }>();
  vi.spyOn(Date, "now").mockReturnValue(retry!.retry_at + 1);
  await operationalCleanup(observed.bindings);
  vi.restoreAllMocks();
  expect(await bindings.DB.prepare("SELECT COUNT(*) AS n FROM privacy_operational_tasks WHERE state='pending'").first()).toEqual({ n: 0 });
  const completed = await runInDurableObject(privacy(), async (_instance, state) => state.storage.sql.exec("SELECT COUNT(*) AS n FROM privacy_rights WHERE operational_state='completed'").one());
  expect(completed.n).toBe(6);
});
