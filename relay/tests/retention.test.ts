import { env } from "cloudflare:workers";
import { applyD1Migrations, reset } from "cloudflare:test";
import { beforeEach, expect, it } from "vitest";
import { createAuth } from "../src/auth.js";
import { retentionCleanup, retentionPolicy } from "../src/retention.js";
import type { Env } from "../src/types.js";
const bindings = env as unknown as Env & { TEST_MIGRATIONS: { name: string; queries: string[] }[] };
beforeEach(async () => { await reset(); await applyD1Migrations(bindings.DB, bindings.TEST_MIGRATIONS); });
it("requires operational approval independently of commercial approval", () => {
  expect(retentionPolicy({ ...bindings, RETENTION_CONFIG: "{}" })).toBeNull();
  expect(retentionPolicy({ ...bindings, COMMERCIAL_CONFIG: "{}" })?.approved).toBe(true);
});
it("purges expired closed and unadmitted identities while respecting legal holds and pending cancellations", async () => {
  const context = await createAuth(bindings).$context; const now = Date.now(); const old = now - 400 * 86400000;
  const ids: string[] = [];
  for (const label of ["expired", "held", "pending", "unadmitted", "unadmitted-held"]) {
    const user = await context.internalAdapter.createUser({ email: `${label}@example.invalid`, name: "Fixture", emailVerified: true }, { method: "api" }); ids.push(user.id);
    await bindings.DB.prepare('UPDATE "user" SET "createdAt"=? WHERE id=?').bind(new Date(old).toISOString(), user.id).run();
    if (label === "unadmitted" || label === "unadmitted-held") {
      await context.internalAdapter.createAccount({ userId: user.id, providerId: "github", accountId: `fixture-${label}` });
      await context.internalAdapter.createSession(user.id);
      continue;
    }
    await bindings.DB.prepare("INSERT INTO relay_accounts(id,status,terms_version,terms_at,email_provenance,created_at,closed_at,cleanup_after) VALUES(?,'closed','fixture',?,'github_verified_primary',?,?,?)").bind(user.id, old, old, old, old).run();
    await bindings.DB.prepare("INSERT INTO relay_billing(account_id,customer_id,plan,status,paid_until,cancel_at_period_end,updated_at) VALUES(?,?,'free','canceled',0,1,?)").bind(user.id, `cus_${label}`, old).run();
  }
  await bindings.DB.prepare("INSERT INTO operational_legal_holds(id,account_id,category,reason,evidence,expires_at) VALUES('hold',?,'payment','Fixture retained claim','Fixture reviewed evidence',?)").bind(ids[1], now + 86400000).run();
  await bindings.DB.prepare("INSERT INTO operational_legal_holds(id,account_id,category,reason,evidence,expires_at) VALUES('unadmitted-hold',?,'provider_identity','Fixture retained identity','Fixture reviewed evidence',?)").bind(ids[4], now + 86400000).run();
  expect(await bindings.DB.prepare('SELECT count(*) AS count FROM "account" WHERE "userId" IN (?,?)').bind(ids[3], ids[4]).first()).toEqual({ count: 2 });
  expect(await bindings.DB.prepare('SELECT count(*) AS count FROM "session" WHERE "userId" IN (?,?)').bind(ids[3], ids[4]).first()).toEqual({ count: 2 });
  await bindings.DB.prepare("INSERT INTO relay_billing_cancellations(account_id,state,created_at,updated_at) VALUES(?,'pending',?,?)").bind(ids[2], old, old).run();
  // Each scheduled pass handles one closed account. Both eligible subjects
  // must receive a pass regardless of their randomly assigned ID order.
  for (let i = 0; i < 3; i += 1) await retentionCleanup(bindings, now);
  await retentionCleanup(bindings, now);
  expect(await bindings.DB.prepare('SELECT id FROM "user" WHERE id IN (?,?)').bind(ids[0], ids[3]).all()).toMatchObject({ results: [] });
  expect(await bindings.DB.prepare('SELECT email FROM "user" WHERE id=?').bind(ids[1]).first()).toEqual({ email: "held@example.invalid" });
  expect(await bindings.DB.prepare('SELECT email FROM "user" WHERE id=?').bind(ids[2]).first()).toEqual({ email: `${ids[2]}@deleted.invalid` });
  expect(await bindings.DB.prepare("SELECT customer_id FROM relay_billing WHERE account_id=?").bind(ids[2]).first()).toEqual({ customer_id: "cus_pending" });
  expect(await bindings.DB.prepare('SELECT "userId" FROM "account" WHERE "userId"=?').bind(ids[3]).first()).toBeNull();
  expect(await bindings.DB.prepare('SELECT "userId" FROM "session" WHERE "userId"=?').bind(ids[3]).first()).toBeNull();
  expect(await bindings.DB.prepare('SELECT email FROM "user" WHERE id=?').bind(ids[4]).first()).toEqual({ email: "unadmitted-held@example.invalid" });
  expect(await bindings.DB.prepare('SELECT "userId" FROM "account" WHERE "userId"=?').bind(ids[4]).first()).toEqual({ userId: ids[4] });
  expect(await bindings.DB.prepare('SELECT "userId" FROM "session" WHERE "userId"=?').bind(ids[4]).first()).toEqual({ userId: ids[4] });
  await bindings.DB.prepare("DELETE FROM operational_legal_holds WHERE id='hold'").run();
  await bindings.DB.prepare("UPDATE relay_billing_cancellations SET state='completed' WHERE account_id=?").bind(ids[2]).run();
  for (let i = 0; i < 2; i += 1) await retentionCleanup(bindings, now + 86400001);
  await retentionCleanup(bindings, now + 86400001);
  expect(await bindings.DB.prepare('SELECT id FROM "user"').all()).toMatchObject({ results: [] });
  expect(await bindings.DB.prepare('SELECT "userId" FROM "account" WHERE "userId" IN (?,?)').bind(ids[3], ids[4]).all()).toMatchObject({ results: [] });
  expect(await bindings.DB.prepare('SELECT "userId" FROM "session" WHERE "userId" IN (?,?)').bind(ids[3], ids[4]).all()).toMatchObject({ results: [] });
  expect(await bindings.DB.prepare("SELECT id FROM operational_legal_holds WHERE id='unadmitted-hold'").first()).toBeNull();
});

it("lets later expired accounts pass a failed purge and retries the preserved account", async () => {
  const context = await createAuth(bindings).$context; const now = Date.now(); const old = now - 400 * 86400000;
  const ids: string[] = [];
  for (let i = 0; i < 3; i += 1) {
    const user = await context.internalAdapter.createUser({ email: `purge-${i}@example.invalid`, name: "Fixture", emailVerified: true }, { method: "api" }); ids.push(user.id);
    await bindings.DB.prepare("INSERT INTO relay_accounts(id,status,terms_version,terms_at,email_provenance,created_at,closed_at,cleanup_after) VALUES(?,'closed','fixture',?,'github_verified_primary',?,?,?)").bind(user.id, old + i, old + i, old + i, old + i).run();
  }
  const calls: string[] = []; let unavailable = true;
  const namespace = bindings.ACCOUNT_QUOTA;
  const override = { ...bindings, ACCOUNT_QUOTA: {
    idFromName: (id: string) => namespace.idFromName(id),
    get: (id: DurableObjectId) => ({ fetch: async (input: RequestInfo, init?: RequestInit) => {
      const request = new Request(input, init); const body = await request.clone().json<{ accountId: string }>(); calls.push(body.accountId);
      if (body.accountId === ids[0] && unavailable) throw new Error("fixture_quota_unavailable");
      return namespace.get(id).fetch(request);
    } }),
  } as unknown as DurableObjectNamespace };
  for (let i = 0; i < 3; i += 1) await retentionCleanup(override, now);
  expect(calls).toEqual(ids);
  expect(await bindings.DB.prepare("SELECT id,cleanup_after FROM relay_accounts").all()).toMatchObject({ results: [{ id: ids[0], cleanup_after: now + 3600000 }] });
  unavailable = false; await retentionCleanup(override, now + 3600001);
  expect(calls).toEqual([...ids, ids[0]]);
  expect(await bindings.DB.prepare('SELECT id FROM "user"').all()).toMatchObject({ results: [] });
});
