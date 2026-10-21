import type { Env } from "./types.js";
import { retentionCleanup } from "./retention.js";
import { diagnostic } from "./diagnostics.js";
import { revokeRelayAccount } from "./accounts.js";
export async function operationalCleanup(env: Env): Promise<void> {
  const now = Date.now();
  await env.DB.batch([
    env.DB.prepare("DELETE FROM connection_tickets WHERE hash IN (SELECT hash FROM connection_tickets WHERE expires_at<=? ORDER BY expires_at LIMIT 500)").bind(now),
    env.DB.prepare("DELETE FROM relay_billing_events WHERE id IN (SELECT id FROM relay_billing_events WHERE expires_at<=? AND status='processed' ORDER BY expires_at LIMIT 200)").bind(now),
    env.DB.prepare('DELETE FROM "session" WHERE id IN (SELECT id FROM "session" WHERE "expiresAt"<? ORDER BY "expiresAt" LIMIT 200)').bind(new Date(now).toISOString()),
    env.DB.prepare('DELETE FROM "verification" WHERE id IN (SELECT id FROM "verification" WHERE "expiresAt"<? ORDER BY "expiresAt" LIMIT 200)').bind(new Date(now).toISOString()),
    env.DB.prepare('DELETE FROM "deviceCode" WHERE id IN (SELECT id FROM "deviceCode" WHERE "expiresAt"<? ORDER BY "expiresAt" LIMIT 200)').bind(new Date(now).toISOString()),
    env.DB.prepare('DELETE FROM "rateLimit" WHERE id IN (SELECT id FROM "rateLimit" WHERE "lastRequest"<? ORDER BY "lastRequest" LIMIT 200)').bind(now - 7 * 86_400_000),
  ]);
  const accounts = await env.DB.prepare("SELECT id FROM relay_accounts WHERE quota_cleanup_at IS NULL OR quota_cleanup_at<=? ORDER BY quota_cleanup_at,created_at LIMIT 2").bind(now - 86400000).all<{ id: string }>();
  for (const row of accounts.results) {
    let cleaned = false;
    try {
      const response = await env.ACCOUNT_QUOTA.get(env.ACCOUNT_QUOTA.idFromName(row.id)).fetch("https://internal/cleanup", { method: "POST" });
      await response.text(); cleaned = response.ok;
    } catch { /* Retry this account after an hour while later accounts can progress. */ }
    await env.DB.prepare("UPDATE relay_accounts SET quota_cleanup_at=? WHERE id=?").bind(cleaned ? now : now - 23 * 3_600_000, row.id).run();
  }
  // Claim one oldest attempt without changing its pending state before provider I/O.
  const cancellations = await env.DB.prepare("UPDATE relay_billing_cancellations SET updated_at=? WHERE account_id IN(SELECT account_id FROM relay_billing_cancellations WHERE state='pending' ORDER BY updated_at,created_at,account_id LIMIT 1) RETURNING account_id").bind(now).all<{ account_id: string }>();
  for (const cancellation of cancellations.results) {
    try {
      const response = await env.ACCOUNT_QUOTA.get(env.ACCOUNT_QUOTA.idFromName(cancellation.account_id)).fetch("https://internal/billing/cancel", { method: "POST", body: JSON.stringify({ accountId: cancellation.account_id }) });
      await response.text();
    } catch { /* The durable pending marker survives a failed provider request. */ }
  }
  // Pending result contains only retry metadata until the authority acknowledges the result.
  // Claim and rotate in one statement so a failed old request cannot block the queue.
  const tasks = await env.DB.prepare("UPDATE privacy_operational_tasks SET result=json_object('retryAfter',?) WHERE request_id IN(SELECT request_id FROM privacy_operational_tasks WHERE state='pending' AND (result IS NULL OR json_extract(result,'$.retryAfter')<=?) ORDER BY COALESCE(json_extract(result,'$.retryAfter'),requested_at),requested_at,request_id LIMIT 1) RETURNING request_id,account_id,action").bind(now + 3_600_000, now).all<{ request_id: string; account_id: string; action: string }>();
  for (const task of tasks.results) {
    try { await completeOperationalTask(env, task); }
    catch { /* Keep pending and retry after the durable delay; continue other phases. */ }
  }
  await retentionCleanup(env, now);
  diagnostic("cleanup_complete", 200, tasks.results.length);
}

async function completeOperationalTask(env: Env, task: { request_id: string; account_id: string; action: string }): Promise<void> {
  let state = "completed";
  let result: Record<string, unknown>;
  if (["access", "export"].includes(task.action)) {
    const account = await env.DB.prepare("SELECT id,status,terms_version,terms_at,email_provenance,created_at,closed_at FROM relay_accounts WHERE id=?").bind(task.account_id).first();
    const devices = await env.DB.prepare("SELECT id,label,created_at,revoked_at,os_family,cli_major FROM relay_devices WHERE account_id=? LIMIT 64").bind(task.account_id).all();
    const billing = await env.DB.prepare("SELECT customer_id,subscription_id,plan,status,paid_until,cancel_at_period_end,updated_at FROM relay_billing WHERE account_id=?").bind(task.account_id).first();
    const identity = await env.DB.prepare('SELECT email,"emailVerified","createdAt" FROM "user" WHERE id=?').bind(task.account_id).first();
    const providers = await env.DB.prepare('SELECT "providerId","accountId","createdAt" FROM "account" WHERE "userId"=? LIMIT 16').bind(task.account_id).all();
    const quotaResponse = await env.ACCOUNT_QUOTA.get(env.ACCOUNT_QUOTA.idFromName(task.account_id)).fetch("https://internal/access", { method: "POST", body: JSON.stringify({ accountId: task.account_id }) });
    const reservations: unknown = await quotaResponse.json();
    if (!quotaResponse.ok) throw new Error("operational_access_unavailable");
    result = { account, identity, providerIdentities: providers.results, devices: devices.results, billing, reservations, retention: "Payment references, consent and disclosure evidence follow configured legal retention policies." };
  } else if (["delete", "close"].includes(task.action)) {
    await revokeRelayAccount(env, task.account_id, task.action === "close");
    const now = Date.now();
    const changes = await env.DB.batch([
      env.DB.prepare("DELETE FROM connection_tickets WHERE account_id=?").bind(task.account_id),
      env.DB.prepare('DELETE FROM "account" WHERE "userId"=? AND NOT EXISTS(SELECT 1 FROM operational_legal_holds WHERE account_id=? AND expires_at>?)').bind(task.account_id, task.account_id, now),
      env.DB.prepare("UPDATE relay_devices SET label='Deleted device',os_family=NULL,cli_major=NULL WHERE account_id=? AND NOT EXISTS(SELECT 1 FROM operational_legal_holds WHERE account_id=? AND expires_at>?)").bind(task.account_id, task.account_id, now),
      env.DB.prepare('UPDATE "user" SET name=\'Deleted account\',email=?,image=NULL,"emailVerified"=0 WHERE id=? AND NOT EXISTS(SELECT 1 FROM operational_legal_holds WHERE account_id=? AND expires_at>?)').bind(`${task.account_id}@deleted.invalid`, task.account_id, task.account_id, now),
      env.DB.prepare("SELECT MAX(expires_at) AS expiresAt FROM operational_legal_holds WHERE account_id=? AND expires_at>?").bind(task.account_id, now),
    ]);
    const hold = changes[4]!.results[0] as { expiresAt: number | null };
    state = "exception";
    result = { deleted: hold.expiresAt === null ? ["Provider credentials", "Session credentials", "Device labels", "Operational email and profile"] : ["Session credentials", "Connection tickets"], retained: ["Minimal closed-account key prevents reopening transfers", "Required payment, consent and disclosure references follow configured category policies"], ...(hold.expiresAt === null ? {} : { legalHold: { expiresAt: hold.expiresAt, retained: ["Provider identity and credentials", "Operational email and profile", "Device labels and metadata"] } }), recipients: "Recipient follow-up is tracked separately; previously released copies cannot be recalled." };
  } else {
    state = "exception"; result = { message: "Provider-owned verified identity corrections require a new verified sign-in or an operator-reviewed correction. No unverified email change is applied." };
  }
  const response = await env.PRIVACY_AUTHORITY.get(env.PRIVACY_AUTHORITY.idFromName("privacy-v1")).fetch("https://internal/internal/operational", { method: "POST", body: JSON.stringify({ requestId: task.request_id, state, result }) });
  await response.text();
  // A failed cross-store acknowledgement keeps the durable task pending for retry.
  if (response.ok) await env.DB.prepare("UPDATE privacy_operational_tasks SET state=?,result=? WHERE request_id=? AND state='pending'").bind(state, JSON.stringify(result), task.request_id).run();
}
