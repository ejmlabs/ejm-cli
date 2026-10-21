import type { Env } from "./types.js";
export interface OperationalRetention {
  approved: true; version: string; approvedBy: string;
  accountActivePurpose: string; unadmittedDays: number;
  accountClosedDays: number; providerIdentityDays: number; paymentDays: number;
  externalNotice: string; legalHoldPolicy: string;
}
export function retentionPolicy<T extends Pick<Env, "RETENTION_CONFIG">>(env: T): OperationalRetention | null {
  if (!env.RETENTION_CONFIG || env.RETENTION_CONFIG.length > 8192) return null;
  try {
    const data = JSON.parse(env.RETENTION_CONFIG) as Partial<OperationalRetention>;
    if (data.approved !== true || typeof data.version !== "string" || data.version.length < 1 || data.version.length > 128 || typeof data.approvedBy !== "string" || data.approvedBy.length < 1 || data.approvedBy.length > 128) return null;
    if (typeof data.accountActivePurpose !== "string" || data.accountActivePurpose.length < 1 || data.accountActivePurpose.length > 2048) return null;
    if (![data.unadmittedDays, data.accountClosedDays, data.providerIdentityDays, data.paymentDays].every(value => typeof value === "number" && Number.isSafeInteger(value) && value >= 1 && value <= 3650)) return null;
    if (typeof data.externalNotice !== "string" || data.externalNotice.length > 2048 || new URL(data.externalNotice).protocol !== "https:" || typeof data.legalHoldPolicy !== "string" || data.legalHoldPolicy.length < 1 || data.legalHoldPolicy.length > 2048) return null;
    return data as OperationalRetention;
  } catch { return null; }
}
export async function retentionCleanup(env: Env, now: number): Promise<void> {
  const policy = retentionPolicy(env);
  if (!policy) return;
  const day = 86_400_000;
  await env.DB.prepare('DELETE FROM "user" WHERE id IN(SELECT u.id FROM "user" u WHERE u."createdAt"<? AND NOT EXISTS(SELECT 1 FROM relay_accounts a WHERE a.id=u.id) ORDER BY u."createdAt" LIMIT 100)').bind(new Date(now - policy.unadmittedDays * day).toISOString()).run();
  const subjects = await env.DB.prepare("SELECT id,closed_at FROM relay_accounts a WHERE status='closed' AND cleanup_after<=? AND NOT EXISTS(SELECT 1 FROM operational_legal_holds h WHERE h.account_id=a.id AND h.expires_at>?) ORDER BY cleanup_after,closed_at,id LIMIT 1").bind(now, now).all<{ id: string; closed_at: number }>();
  for (const subject of subjects.results) {
    const pendingCancellation = await env.DB.prepare("SELECT account_id FROM relay_billing_cancellations WHERE account_id=? AND state='pending'").bind(subject.id).first();
    if (subject.closed_at <= now - policy.providerIdentityDays * day) await env.DB.batch([
      env.DB.prepare('DELETE FROM "account" WHERE "userId"=?').bind(subject.id),
      env.DB.prepare('DELETE FROM "session" WHERE "userId"=?').bind(subject.id),
      env.DB.prepare('UPDATE "user" SET name=\'Deleted account\',email=?,image=NULL,"emailVerified"=0 WHERE id=?').bind(`${subject.id}@deleted.invalid`, subject.id),
    ]);
    if (subject.closed_at <= now - policy.accountClosedDays * day) await env.DB.prepare("DELETE FROM relay_devices WHERE account_id=?").bind(subject.id).run();
    if (!pendingCancellation && subject.closed_at <= now - policy.paymentDays * day) await env.DB.batch([
      env.DB.prepare("DELETE FROM relay_billing_events WHERE account_id=?").bind(subject.id),
      env.DB.prepare("DELETE FROM relay_payment_holds WHERE account_id=?").bind(subject.id),
      env.DB.prepare("DELETE FROM relay_billing WHERE account_id=?").bind(subject.id),
      env.DB.prepare("DELETE FROM relay_billing_cancellations WHERE account_id=?").bind(subject.id),
    ]);
    const next = [policy.accountClosedDays, policy.providerIdentityDays, policy.paymentDays].map(days => subject.closed_at + days * day).filter(expiry => expiry > now);
    if (pendingCancellation) next.push(now + day);
    if (next.length > 0) await env.DB.prepare("UPDATE relay_accounts SET cleanup_after=? WHERE id=?").bind(Math.min(...next), subject.id).run();
    if (!pendingCancellation && subject.closed_at <= now - Math.max(policy.accountClosedDays, policy.providerIdentityDays, policy.paymentDays) * day) {
      let purged = false;
      try {
        const quota = await env.ACCOUNT_QUOTA.get(env.ACCOUNT_QUOTA.idFromName(subject.id)).fetch("https://internal/purge", { method: "POST", body: JSON.stringify({ accountId: subject.id }) });
        await quota.text(); purged = quota.ok;
      } catch { /* Leave the account durable when its quota authority is unavailable. */ }
      if (!purged) {
        await env.DB.prepare("UPDATE relay_accounts SET cleanup_after=? WHERE id=?").bind(now + 3_600_000, subject.id).run();
        continue;
      }
      await env.DB.batch([
        env.DB.prepare("DELETE FROM relay_accounts WHERE id=?").bind(subject.id),
        env.DB.prepare('DELETE FROM "user" WHERE id=?').bind(subject.id),
      ]);
    }
  }
  await env.DB.prepare("DELETE FROM operational_legal_holds WHERE id IN(SELECT id FROM operational_legal_holds WHERE expires_at<=? ORDER BY expires_at LIMIT 100)").bind(now).run();
}
