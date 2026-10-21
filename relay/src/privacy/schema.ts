/** Commercial projections are intentionally separate from operational account records. */
export const PROFILE_FIELDS = ["role", "industry", "organizationSize", "country"] as const;
export const USAGE_FIELDS = ["osFamily", "cliMajor", "connectedHours", "traffic", "activeDays"] as const;
export type Category = "profile" | "usage";
export type ProfileField = typeof PROFILE_FIELDS[number];
export type UsageField = typeof USAGE_FIELDS[number];
export type CommercialField = ProfileField | UsageField;
export type Quality = "self-reported" | "measured" | "estimated";

const roles = ["engineering", "design", "product", "research", "operations", "education", "other"];
const industries = ["software", "finance", "health", "education", "manufacturing", "public-sector", "nonprofit", "other"];
const sizes = ["solo", "2-10", "11-50", "51-250", "251-1000", "1001+"];
const countries = new Set("AD AE AF AG AI AL AM AO AQ AR AS AT AU AW AX AZ BA BB BD BE BF BG BH BI BJ BL BM BN BO BQ BR BS BT BV BW BY BZ CA CC CD CF CG CH CI CK CL CM CN CO CR CU CV CW CX CY CZ DE DJ DK DM DO DZ EC EE EG EH ER ES ET FI FJ FK FM FO FR GA GB GD GE GF GG GH GI GL GM GN GP GQ GR GS GT GU GW GY HK HM HN HR HT HU ID IE IL IM IN IO IQ IR IS IT JE JM JO JP KE KG KH KI KM KN KP KR KW KY KZ LA LB LC LI LK LR LS LT LU LV LY MA MC MD ME MF MG MH MK ML MM MN MO MP MQ MR MS MT MU MV MW MX MY MZ NA NC NE NF NG NI NL NO NP NR NU NZ OM PA PE PF PG PH PK PL PM PN PR PS PT PW PY QA RE RO RS RU RW SA SB SC SD SE SG SH SI SJ SK SL SM SN SO SR SS ST SV SX SY SZ TC TD TF TG TH TJ TK TL TM TN TO TR TT TV TW TZ UA UG UM US UY UZ VA VC VE VG VI VN VU WF WS YE YT ZA ZM ZW".split(" "));
export const OS_FAMILIES = ["macos", "linux", "windows", "other"] as const;
export type Profile = Partial<Record<ProfileField, string>>;
export type DataRecord = Record<string, string | number>;

export class PrivacyError extends Error {
  constructor(public readonly code: string, public readonly status = 400) { super(code); }
}

export function object(value: unknown): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new PrivacyError("invalid_object");
  return value as Record<string, unknown>;
}

export function exactKeys(value: Record<string, unknown>, allowed: readonly string[]): void {
  if (Object.keys(value).some((key) => !allowed.includes(key))) throw new PrivacyError("unapproved_field");
}

export function text(value: unknown, max = 128): string {
  if (typeof value !== "string" || value.length === 0 || value.length > max || /[\u0000-\u001f]/u.test(value)) throw new PrivacyError("invalid_text");
  return value;
}

export function fieldSet(value: unknown, category: Category): CommercialField[] {
  const allowed = category === "profile" ? PROFILE_FIELDS : USAGE_FIELDS;
  if (!Array.isArray(value) || value.length < 1 || value.length > allowed.length) throw new PrivacyError("invalid_fields");
  const fields = value.map((field) => text(field, 32));
  if (new Set(fields).size !== fields.length || fields.some((field) => !(allowed as readonly string[]).includes(field))) throw new PrivacyError("unapproved_field");
  return fields.sort() as CommercialField[];
}

export function category(value: unknown): Category {
  if (value !== "profile" && value !== "usage") throw new PrivacyError("invalid_category");
  return value;
}

export function directProfile(input: unknown): Profile {
  const value = object(input);
  exactKeys(value, ["provenance", "fields"]);
  if (value.provenance !== "direct-user") throw new PrivacyError("provider_data_forbidden");
  const fields = object(value.fields);
  exactKeys(fields, PROFILE_FIELDS);
  if (Object.keys(fields).length === 0) throw new PrivacyError("empty_profile");
  for (const [field, raw] of Object.entries(fields)) {
    const entry = text(raw, 32);
    const allowed = field === "role" ? roles : field === "industry" ? industries : field === "organizationSize" ? sizes : countries;
    if (!(allowed instanceof Set ? allowed.has(entry) : allowed.includes(entry))) throw new PrivacyError("invalid_profile_choice");
  }
  return fields as Profile;
}

export function finiteCount(value: unknown, max: number): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0 || value > max) throw new PrivacyError("invalid_count");
  return value;
}

export function usageSample(input: unknown): {
  accountId: string; bytes: number; frames: number; connectedMs: number;
  osFamily?: string; cliMajor?: number; quality: "measured" | "estimated"; measuredAt: number;
} {
  const value = object(input);
  exactKeys(value, ["accountId", "bytes", "frames", "connectedMs", "osFamily", "cliMajor", "quality", "measuredAt"]);
  if (value.quality !== "measured" && value.quality !== "estimated") throw new PrivacyError("invalid_quality");
  if (value.osFamily !== undefined && !(OS_FAMILIES as readonly unknown[]).includes(value.osFamily)) throw new PrivacyError("invalid_os");
  return {
    accountId: text(value.accountId), bytes: finiteCount(value.bytes, 1 << 30), frames: finiteCount(value.frames, 1_000_000),
    connectedMs: finiteCount(value.connectedMs, 86_400_000), quality: value.quality,
    measuredAt: finiteCount(value.measuredAt, Number.MAX_SAFE_INTEGER),
    ...(value.osFamily === undefined ? {} : { osFamily: value.osFamily as string }),
    ...(value.cliMajor === undefined ? {} : { cliMajor: finiteCount(value.cliMajor, 9999) }),
  };
}

export function coarseBand(value: number, thresholds: readonly number[]): string {
  let lower = 0;
  for (const upper of thresholds) {
    if (value < upper) return `${lower}-${upper}`;
    lower = upper;
  }
  return `${lower}+`;
}

export function usageProjection(value: { bytes: number; connectedMs: number; days: number; osFamily: string | null; cliMajor: number | null }, fields: CommercialField[]): DataRecord {
  const all: DataRecord = {
    connectedHours: coarseBand(value.connectedMs / 3_600_000, [1, 5, 20, 80]),
    traffic: coarseBand(value.bytes / 1_048_576, [1, 10, 100, 1000]),
    activeDays: coarseBand(value.days, [2, 5, 10, 20]),
  };
  if (value.osFamily !== null) all.osFamily = value.osFamily;
  if (value.cliMajor !== null) all.cliMajor = value.cliMajor;
  return Object.fromEntries(fields.filter((field) => all[field] !== undefined).map((field) => [field, all[field] as string | number]));
}
