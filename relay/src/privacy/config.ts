import { category, exactKeys, fieldSet, finiteCount, object, PrivacyError, text, type Category, type CommercialField } from "./schema.js";

export interface Buyer {
  id: string; name: string; purpose: string; noticeVersion: string; noticeUrl: string;
  fields: { profile: CommercialField[]; usage: CommercialField[] };
  retentionDays: number; contractEvidence: string; approvedAt: number; expiresAt: number;
}
export interface RightsPolicy { jurisdiction: string; acknowledgementDays: number; responseDays: number; extensionDays: number; evidence: string }
export interface CommercialConfig {
  enabled: boolean; approval: string; notice: string; applicability: string; riskAssessment: string;
  recipientProcess: string; rightsProcess: string; providerRetention: string; reidentificationAssessment: string;
  jurisdictionPolicies: RightsPolicy[];
  retention: { profileWithdrawalDays: number; usageDays: number; consentDays: number; disclosureDays: number; rightsDays: number; browserDays: number; stagingHours: number; operationalTaskDays: number };
  buyers: Buyer[];
}

export const DEFAULT_RETENTION = { profileWithdrawalDays: 30, usageDays: 90, consentDays: 2190, disclosureDays: 2190, rightsDays: 2190, browserDays: 730, stagingHours: 1, operationalTaskDays: 30 };

export function parseConfig(raw: string | undefined): CommercialConfig | null {
  if (!raw) return null;
  try {
    const value = object(JSON.parse(raw));
    exactKeys(value, ["enabled", "approval", "notice", "applicability", "riskAssessment", "recipientProcess", "rightsProcess", "providerRetention", "reidentificationAssessment", "jurisdictionPolicies", "retention", "buyers"]);
    if (value.enabled !== true) return null;
    const retention = object(value.retention);
    exactKeys(retention, Object.keys(DEFAULT_RETENTION));
    const parsedRetention = Object.fromEntries(Object.keys(DEFAULT_RETENTION).map((key) => [key, positive(retention[key], key === "stagingHours" ? 24 : 3650)])) as CommercialConfig["retention"];
    if (parsedRetention.profileWithdrawalDays > 30 || parsedRetention.usageDays > 90) throw new PrivacyError("retention_exceeds_published_limit");
    if (parsedRetention.browserDays < 366 || parsedRetention.consentDays < 366) throw new PrivacyError("retention_cannot_preserve_optout");
    if (!Array.isArray(value.jurisdictionPolicies) || value.jurisdictionPolicies.length === 0 || value.jurisdictionPolicies.length > 32) throw new PrivacyError("missing_rights_policy");
    const policies = value.jurisdictionPolicies.map((entry) => {
      const policy = object(entry); exactKeys(policy, ["jurisdiction", "acknowledgementDays", "responseDays", "extensionDays", "evidence"]);
      const parsed = { jurisdiction: text(policy.jurisdiction, 64), acknowledgementDays: positive(policy.acknowledgementDays, 10), responseDays: positive(policy.responseDays, 45), extensionDays: finiteCount(policy.extensionDays, 90), evidence: text(policy.evidence, 512) };
      if (parsed.acknowledgementDays > parsed.responseDays) throw new PrivacyError("invalid_deadline");
      return parsed;
    });
    if (!Array.isArray(value.buyers) || value.buyers.length === 0 || value.buyers.length > 32) throw new PrivacyError("missing_named_buyer");
    const buyers = value.buyers.map((entry) => parseBuyer(entry));
    if (new Set(buyers.map((buyer) => buyer.id)).size !== buyers.length) throw new PrivacyError("duplicate_buyer");
    return { enabled: true, approval: text(value.approval, 512), notice: text(value.notice, 512), applicability: text(value.applicability, 512), riskAssessment: text(value.riskAssessment, 512), recipientProcess: text(value.recipientProcess, 512), rightsProcess: text(value.rightsProcess, 512), providerRetention: text(value.providerRetention, 512), reidentificationAssessment: text(value.reidentificationAssessment, 512), jurisdictionPolicies: policies, retention: parsedRetention, buyers };
  } catch { return null; }
}

function positive(value: unknown, max: number): number { const count = finiteCount(value, max); if (count < 1) throw new PrivacyError("invalid_retention"); return count; }
function parseBuyer(input: unknown): Buyer {
  const value = object(input);
  exactKeys(value, ["id", "name", "purpose", "noticeVersion", "noticeUrl", "fields", "retentionDays", "contractEvidence", "approvedAt", "expiresAt"]);
  const fields = object(value.fields); exactKeys(fields, ["profile", "usage"]);
  const url = new URL(text(value.noticeUrl, 512)); if (url.protocol !== "https:") throw new PrivacyError("insecure_notice");
  const buyer = { id: text(value.id, 64), name: text(value.name, 128), purpose: text(value.purpose, 512), noticeVersion: text(value.noticeVersion, 64), noticeUrl: url.href, fields: { profile: fieldSet(fields.profile, "profile"), usage: fieldSet(fields.usage, "usage") }, retentionDays: positive(value.retentionDays, 365), contractEvidence: text(value.contractEvidence, 512), approvedAt: finiteCount(value.approvedAt, Number.MAX_SAFE_INTEGER), expiresAt: finiteCount(value.expiresAt, Number.MAX_SAFE_INTEGER) };
  if (buyer.approvedAt >= buyer.expiresAt) throw new PrivacyError("invalid_buyer_approval");
  return buyer;
}

export function requireConfig(config: CommercialConfig | null): CommercialConfig { if (config === null) throw new PrivacyError("commercial_program_disabled", 503); return config; }
export function approvedBuyer(config: CommercialConfig | null, id: string, now: number): Buyer {
  const buyer = requireConfig(config).buyers.find((entry) => entry.id === id);
  if (!buyer || buyer.approvedAt > now || buyer.expiresAt <= now) throw new PrivacyError("buyer_not_approved", 403);
  return buyer;
}
export function buyerScope(buyer: Buyer, kind: Category): string {
  return JSON.stringify([buyer.id, buyer.name, buyer.purpose, buyer.noticeVersion, buyer.noticeUrl, buyer.fields[category(kind)], buyer.retentionDays, buyer.contractEvidence, buyer.approvedAt, buyer.expiresAt]);
}
