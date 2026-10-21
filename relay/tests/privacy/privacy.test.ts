/// <reference types="@cloudflare/vitest-plugin/types" />
import { env } from "cloudflare:workers";
import { runInDurableObject } from "cloudflare:test";
import { describe, expect, it as workerTest } from "vitest";
import type { Env } from "../../src/types.js";
import { buyerScope, parseConfig, type CommercialConfig } from "../../src/privacy/config.js";
import { directProfile, usageSample } from "../../src/privacy/schema.js";
import { PrivacyService, type Actor } from "../../src/privacy/service.js";
import type { Binding, SqlStore } from "../../src/privacy/store.js";
import { accountPage, optOutPage } from "../../src/privacy/html.js";

const DAY = 86_400_000;
const START = Date.UTC(2026, 9, 18, 12);
let activeStorage: DurableObjectStorage | null = null;
function it(name: string, callback: () => void | Promise<void>): void {
  workerTest(name, async () => {
    const bindings = env as unknown as Env;
    const stub = bindings.PRIVACY_AUTHORITY.get(bindings.PRIVACY_AUTHORITY.idFromName(crypto.randomUUID()));
    await runInDurableObject(stub, async (_instance, state) => {
      activeStorage = state.storage;
      try { await callback(); } finally { activeStorage = null; }
    });
  });
}
class SQLiteStore implements SqlStore {
  private depth = 0;
  private readonly storage: DurableObjectStorage;
  constructor() {
    if (activeStorage === null) throw new Error("SQLite fixture requires the Durable Object context");
    this.storage = activeStorage;
    for (const row of this.exec<{ name: string }>("SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'privacy_%'")) {
      if (!/^privacy_[a-z_]+$/u.test(row.name)) throw new Error("Unexpected fixture table");
      this.storage.sql.exec(`DROP TABLE ${row.name}`);
    }
  }
  exec<T extends Record<string, unknown>>(query: string, ...bindings: Binding[]): T[] { return this.storage.sql.exec(query, ...bindings).toArray() as T[]; }
  transaction<T>(callback: () => T): T {
    if (this.depth > 0) return callback();
    return this.storage.transactionSync(() => { this.depth += 1; try { return callback(); } finally { this.depth -= 1; } });
  }
}
function configuration(): CommercialConfig {
  return { enabled: true, approval: "fixture-operator-approval", notice: "fixture-reviewed-notice", applicability: "fixture-jurisdiction-decision", riskAssessment: "fixture-prior-risk-assessment", recipientProcess: "fixture-recipient-contract-process", rightsProcess: "fixture-rights-runbook", providerRetention: "fixture-provider-retention-notice", reidentificationAssessment: "fixture-overlap-assessment", jurisdictionPolicies: [{ jurisdiction: "fixture", acknowledgementDays: 10, responseDays: 30, extensionDays: 30, evidence: "fixture-deadline-policy" }], retention: { profileWithdrawalDays: 30, usageDays: 90, consentDays: 730, disclosureDays: 730, rightsDays: 730, browserDays: 730, stagingHours: 1, operationalTaskDays: 30 }, buyers: [{ id: "buyer-a", name: "Fixture buyer A", purpose: "Fixture developer-market research", noticeVersion: "fixture-v1", noticeUrl: "https://buyer.invalid/fixture-notice", fields: { profile: ["country", "industry", "organizationSize", "role"], usage: ["activeDays", "cliMajor", "connectedHours", "osFamily", "traffic"] }, retentionDays: 30, contractEvidence: "fixture-signed-terms", approvedAt: START - DAY, expiresAt: START + 500 * DAY }] };
}
function fixture(config: CommercialConfig | null = configuration()) {
  const store = new SQLiteStore(); let now = START;
  const service = new PrivacyService(store, config, () => now);
  return { service, store, config, advance: (ms: number) => { now += ms; }, clock: () => now };
}
function actor(id = "account-a", options: Partial<Actor> = {}): Actor { return { accountId: id, browserId: null, gpc: false, operator: false, ...options }; }
const operator = actor("operator", { operator: true });
function consent(service: PrivacyService, who: Actor, config: CommercialConfig, kind: "profile" | "usage" = "profile", buyerId = "buyer-a") {
  const buyer = config.buyers.find((entry) => entry.id === buyerId)!;
  return service.acceptConsent(who, { buyerId, category: kind, noticeVersion: buyer.noticeVersion, noticeScope: buyerScope(buyer, kind), fields: buyer.fields[kind], accepted: true });
}
function enroll(service: PrivacyService, who: Actor, config: CommercialConfig = configuration(), role = "engineering") {
  service.adult(who, { adult: true }); consent(service, who, config); service.profile(who, { provenance: "direct-user", fields: { role, industry: "software", country: "US" } });
}
function prepare(service: PrivacyService, ids = ["account-a"], category = "profile", fields = ["role"]) {
  return service.prepare(operator, { buyerId: "buyer-a", category, fields, accountIds: ids, approved: true }) as { manifestId: string };
}

describe("commercial activation, consent and provenance", () => {
  it("defaults off and rejects incomplete launch evidence", () => {
    expect(parseConfig(undefined)).toBeNull(); expect(parseConfig("{}")).toBeNull();
    for (const key of ["approval", "notice", "applicability", "riskAssessment", "recipientProcess", "rightsProcess", "providerRetention", "reidentificationAssessment", "jurisdictionPolicies", "retention", "buyers"] as const) {
      const config = configuration() as unknown as Record<string, unknown>; delete config[key]; expect(parseConfig(JSON.stringify(config))).toBeNull();
    }
    const { service } = fixture(null); expect(service.status(actor())).toMatchObject({ programEnabled: false, consents: [], serviceAccessChanged: false });
    expect(() => service.adult(actor(), { adult: true })).toThrow("commercial_program_disabled");
    expect(service.recordUsage({ forbidden: "secret" })).toEqual({ collected: false });
    expect(() => prepare(service)).toThrow("commercial_program_disabled");
  });
  it("requires an adult and separate default-off category choices", () => {
    const { service } = fixture();
    expect(service.status(actor())).toMatchObject({ adultAffirmed: false, consents: [] });
    expect(() => consent(service, actor(), configuration())).toThrow("participation_ineligible");
    service.adult(actor(), { adult: true }); consent(service, actor(), configuration());
    service.profile(actor(), { provenance: "direct-user", fields: { role: "engineering" } });
    expect(service.recordUsage({ accountId: "account-a", bytes: 10, frames: 1, connectedMs: 1000, quality: "measured", measuredAt: START })).toEqual({ collected: false });
    service.adult(actor(), { adult: false }); expect(() => prepare(service)).toThrow("export_eligibility_changed");
  });
  it("rejects forbidden fields, copied provider data, enrichment and unknown choices", () => {
    expect(() => directProfile({ provenance: "github", fields: { role: "engineering" } })).toThrow("provider_data_forbidden");
    for (const field of ["email", "name", "company", "repository", "rawIp", "path", "token", "prompt", "birthDate"]) expect(() => directProfile({ provenance: "direct-user", fields: { [field]: "secret" } })).toThrow("unapproved_field");
    expect(() => directProfile({ provenance: "direct-user", fields: { country: "not-a-country" } })).toThrow("invalid_profile_choice");
    expect(() => usageSample({ accountId: "a", bytes: 1, frames: 1, connectedMs: 1, quality: "measured", measuredAt: START, command: "secret" })).toThrow("unapproved_field");
  });
  it("blocks new buyers, changed notices, fields and purposes until fresh consent", () => {
    const { service, store } = fixture(); enroll(service, actor());
    const config = configuration(); config.buyers[0]!.purpose = "A different purpose";
    const changed = new PrivacyService(store, config, () => START);
    expect(() => prepare(changed)).toThrow("export_eligibility_changed");
    expect(() => consent(changed, actor(), configuration())).toThrow("notice_scope_mismatch");
    config.buyers[0]!.noticeVersion = "fixture-v2";
    expect(() => consent(changed, actor(), configuration())).toThrow("notice_scope_mismatch");
    expect(() => service.prepare(operator, { buyerId: "unknown", category: "profile", fields: ["role"], accountIds: ["account-a"], approved: true })).toThrow("buyer_not_approved");
  });
  it("keeps relay authority and price outside every commercial mutation", () => {
    const { service, store } = fixture(); enroll(service, actor());
    expect(service.withdraw(actor(), { category: "profile" })).toMatchObject({ serviceAccessChanged: false });
    expect(service.optOut(actor())).toMatchObject({ optedOut: true, loginRequired: false });
    expect(store.exec<{ name: string }>("SELECT name FROM sqlite_master WHERE type='table'").every((table) => !["accounts", "subscriptions", "devices", "rooms"].includes(table.name))).toBe(true);
  });
});

describe("persistent opt-outs and deliberate rejoining", () => {
  it("saves public browser GPC without an account and keeps it after the signal disappears", () => {
    const { service, store } = fixture(); const browser = crypto.randomUUID();
    expect(service.optOut({ accountId: null, operator: false, gpc: true, browserId: browser })).toMatchObject({ optedOut: true, loginRequired: false });
    service.adult(actor(), { adult: true });
    const returning = actor("account-a", { browserId: browser });
    expect(service.status(returning)).toMatchObject({ optedOut: true, maySolicit: false });
    expect(() => consent(service, returning, configuration())).toThrow("participation_ineligible");
    const restarted = new PrivacyService(store, configuration(), () => START + DAY);
    expect(restarted.status(returning)).toMatchObject({ optedOut: true });
  });
  it("requires two actions, refuses active GPC and does not restore previous choices", () => {
    const { service } = fixture(); enroll(service, actor()); service.optOut(actor());
    expect(service.status(actor())).toMatchObject({ maySolicit: false });
    const request = service.rejoin(actor(), { step: "request" }) as { token: string };
    expect(() => consent(service, actor(), configuration())).toThrow("participation_ineligible");
    expect(() => service.rejoin(actor("account-a", { gpc: true }), { step: "confirm", token: request.token, confirmed: true })).toThrow("active_gpc");
    expect(() => service.rejoin(actor("other"), { step: "confirm", token: request.token, confirmed: true })).toThrow("participation_ineligible");
    expect(service.rejoin(actor(), { step: "confirm", token: request.token, confirmed: true })).toMatchObject({ rejoined: true, salesEnabled: false, requiresFreshConsent: true });
    expect(() => prepare(service)).toThrow("export_eligibility_changed");
    expect(() => service.rejoin(actor(), { step: "confirm", token: request.token, confirmed: true })).toThrow("rejoin_expired");
  });
  it("suppresses solicitation for twelve months and expires abandoned rejoin challenges", () => {
    const { service, advance } = fixture(); enroll(service, actor()); service.optOut(actor());
    const request = service.rejoin(actor(), { step: "request" }) as { token: string }; advance(11 * 60_000);
    expect(() => service.rejoin(actor(), { step: "confirm", token: request.token, confirmed: true })).toThrow("rejoin_expired");
    advance(365 * DAY); expect(service.status(actor())).toMatchObject({ maySolicit: false });
    advance(2 * DAY); expect(service.status(actor())).toMatchObject({ maySolicit: true, optedOut: true });
  });
});

describe("bounded manual disclosure and withdrawal races", () => {
  it("stages only a manifest and no downloadable projection", () => {
    const { service, store } = fixture(); enroll(service, actor()); const manifest = prepare(service);
    const rows = store.exec<{ candidates: string; fields: string }>("SELECT candidates,fields FROM privacy_manifests WHERE id=?", manifest.manifestId);
    expect(rows[0]?.candidates).not.toContain("engineering"); expect(rows[0]?.candidates).not.toContain("US");
    expect(() => service.prepare(actor(), { buyerId: "buyer-a", category: "profile", fields: ["role"], accountIds: ["account-a"], approved: true })).toThrow("operator_required");
    expect(() => service.prepare(operator, { buyerId: "buyer-a", category: "profile", fields: ["email"], accountIds: ["account-a"], approved: true })).toThrow("unapproved_field");
    expect(() => service.prepare(operator, { buyerId: "buyer-a", category: "profile", fields: ["role"], accountIds: Array.from({ length: 129 }, (_, index) => `a-${index}`), approved: true })).toThrow("export_bound_exceeded");
  });
  it("atomically blocks release when withdrawal wins even after projection was built", async () => {
    const { service, store } = fixture(); enroll(service, actor()); const manifest = prepare(service);
    const pending = service.release(operator, { manifestId: manifest.manifestId, approved: true });
    service.withdraw(actor(), { category: "profile" });
    await expect(pending).rejects.toThrow("manifest_not_releasable");
    expect(store.exec("SELECT id FROM privacy_ledger")).toHaveLength(0);
    await expect(service.release(operator, { manifestId: manifest.manifestId, approved: true })).rejects.toThrow("manifest_not_releasable");
  });
  it("commits the whole cohort or none when deletion or correction races release", async () => {
    for (const action of ["delete", "correct"] as const) {
      const { service, store } = fixture(); enroll(service, actor()); enroll(service, actor("account-b")); const manifest = prepare(service, ["account-a", "account-b"]);
      const pending = service.release(operator, { manifestId: manifest.manifestId, approved: true });
      if (action === "delete") service.close("account-b"); else service.profile(actor("account-b"), { provenance: "direct-user", fields: { role: "design" } });
      await expect(pending).rejects.toThrow(); expect(store.exec("SELECT id FROM privacy_ledger")).toHaveLength(0);
    }
  });
  it("records a release once and treats later withdrawal as recipient follow-up", async () => {
    const { service, store } = fixture(); enroll(service, actor()); const manifest = prepare(service);
    const released = await service.release(operator, { manifestId: manifest.manifestId, approved: true }) as { payload: { records: { subject: string; fields: unknown }[] }; sha256: string };
    expect(released.payload.records[0]?.subject).not.toBe("account-a"); expect(released.payload.records[0]?.fields).toEqual({ role: "engineering" }); expect(released.sha256).toMatch(/^[a-f0-9]{64}$/u);
    service.withdraw(actor(), { category: "profile" });
    expect(store.exec("SELECT id FROM privacy_ledger")).toHaveLength(1);
    await expect(service.release(operator, { manifestId: manifest.manifestId, approved: true })).rejects.toThrow("manifest_not_releasable");
    const rights = await service.rightsRequest(actor(), { kind: "delete", jurisdiction: "fixture" }) as { requestId: string };
    const row = store.exec<{ recipients: string }>("SELECT recipients FROM privacy_rights WHERE id=?", rights.requestId)[0]!;
    expect(JSON.parse(row.recipients)).toMatchObject([{ buyerId: "buyer-a", state: "pending" }]);
    expect(() => service.rightsUpdate(operator, { requestId: rights.requestId, action: "complete" })).toThrow("cross_store_work_incomplete");
  });
  it("does not let a different operator reuse prepared authority", async () => {
    const { service } = fixture(); enroll(service, actor()); const manifest = prepare(service);
    await expect(service.release(actor("another-operator", { operator: true }), { manifestId: manifest.manifestId, approved: true })).rejects.toThrow("manifest_not_found");
  });
  it("rechecks persisted provenance rather than assuming all stored values are direct", async () => {
    const { service, store } = fixture(); enroll(service, actor()); const manifest = prepare(service);
    store.exec("UPDATE privacy_subjects SET profile=? WHERE account_id=?", JSON.stringify({ provenance: "github", fields: { role: "engineering" } }), "account-a");
    await expect(service.release(operator, { manifestId: manifest.manifestId, approved: true })).rejects.toThrow("provider_data_forbidden");
    expect(store.exec("SELECT id FROM privacy_ledger")).toHaveLength(0);
  });
});

describe("coarse measurements, estimates and retention", () => {
  it("collects only fields covered by current scoped usage consents", async () => {
    const config = configuration(); config.buyers[0]!.fields.usage = ["activeDays"];
    config.buyers.push({ ...config.buyers[0]!, id: "buyer-b", name: "Fixture buyer B", fields: { profile: ["role"], usage: ["traffic", "osFamily"] }, expiresAt: START + DAY });
    const { service, store, advance, clock } = fixture(config); enroll(service, actor(), config); consent(service, actor(), config, "usage");
    const sample = { accountId: "account-a", bytes: 25_000_000, frames: 100, connectedMs: 7_200_000, quality: "measured", osFamily: "linux", cliMajor: 1, measuredAt: START };
    service.recordUsage(sample);
    const stored = () => store.exec("SELECT bytes,frames,connected_ms,active_days,os_family,cli_major FROM privacy_usage WHERE account_id='account-a'")[0];
    expect(stored()).toMatchObject({ bytes: 0, frames: 0, connected_ms: 0, active_days: '["18"]', os_family: null, cli_major: null });
    const manifest = prepare(service, ["account-a"], "usage", ["activeDays"]);
    const released = await service.release(operator, { manifestId: manifest.manifestId, approved: true }) as { payload: { records: { summaries: { fields: unknown }[] }[] } };
    expect(released.payload.records[0]?.summaries[0]?.fields).toEqual({ activeDays: "0-2" });
    consent(service, actor(), config, "usage", "buyer-b"); service.recordUsage(sample);
    expect(stored()).toMatchObject({ bytes: 25_000_000, frames: 0, connected_ms: 0, active_days: '["18"]', os_family: "linux", cli_major: null });
    advance(2 * DAY); service.recordUsage({ ...sample, measuredAt: clock() });
    expect(stored()).toMatchObject({ bytes: 0, frames: 0, connected_ms: 0, active_days: '["18","20"]', os_family: null, cli_major: null });
  });
  it("expires a monthly aggregate at its earliest included contribution deadline", async () => {
    const { service, advance, store, clock } = fixture(); enroll(service, actor()); consent(service, actor(), configuration(), "usage");
    const sample = { accountId: "account-a", bytes: 25_000_000, frames: 100, connectedMs: 1000, quality: "measured", measuredAt: START };
    service.recordUsage(sample); advance(10 * DAY); service.recordUsage({ ...sample, measuredAt: clock() });
    advance(79 * DAY); expect(service.access(actor())).toMatchObject({ usage: [expect.objectContaining({ bytes: 50_000_000 })] });
    advance(DAY); expect(service.access(actor())).toMatchObject({ usage: [] });
    const manifest = prepare(service, ["account-a"], "usage", ["traffic"]);
    await expect(service.release(operator, { manifestId: manifest.manifestId, approved: true })).rejects.toThrow("usage_unavailable");
    service.cleanup(1); expect(store.exec("SELECT account_id FROM privacy_usage")).toHaveLength(0);
  });
  it("starts expired same-month usage from zero before cleanup runs", async () => {
    const config = configuration(); config.retention.usageDays = 1;
    const { service, advance, clock } = fixture(config); enroll(service, actor(), config); consent(service, actor(), config, "usage");
    service.recordUsage({ accountId: "account-a", bytes: 25_000_000, frames: 100, connectedMs: 7_200_000, quality: "measured", osFamily: "linux", cliMajor: 1, measuredAt: START });
    advance(2 * DAY); service.recordUsage({ accountId: "account-a", bytes: 1000, frames: 1, connectedMs: 1000, quality: "measured", measuredAt: clock() });
    const manifest = prepare(service, ["account-a"], "usage", ["traffic", "connectedHours", "activeDays", "osFamily", "cliMajor"]);
    const released = await service.release(operator, { manifestId: manifest.manifestId, approved: true }) as { payload: { records: { summaries: { fields: unknown }[] }[] } };
    expect(released.payload.records[0]?.summaries[0]?.fields).toEqual({ activeDays: "0-2", connectedHours: "0-1", traffic: "0-1" });
  });
  it("collects only after usage consent and exports bands with honest quality labels", async () => {
    const { service } = fixture(); enroll(service, actor()); consent(service, actor(), configuration(), "usage");
    service.recordUsage({ accountId: "account-a", bytes: 25_000_000, frames: 100, connectedMs: 7_200_000, quality: "measured", osFamily: "linux", cliMajor: 1, measuredAt: START });
    service.recordUsage({ accountId: "account-a", bytes: 1000, frames: 0, connectedMs: 1000, quality: "estimated", measuredAt: START });
    const manifest = prepare(service, ["account-a"], "usage", ["traffic", "connectedHours", "activeDays", "osFamily", "cliMajor"]);
    const released = await service.release(operator, { manifestId: manifest.manifestId, approved: true }) as { payload: { records: { summaries: { quality: string; fields: unknown }[] }[] } };
    const summaries = released.payload.records[0]!.summaries;
    expect(summaries).toHaveLength(2); expect(summaries.map((summary) => summary.quality).sort()).toEqual(["estimated", "measured"]);
    expect(summaries.find((summary) => summary.quality === "measured")?.fields).toEqual({ activeDays: "0-2", cliMajor: 1, connectedHours: "1-5", osFamily: "linux", traffic: "10-100" });
    expect(JSON.stringify(released)).not.toContain("25000000");
  });
  it("drops blocks spanning consent changes and never sells historical collection to a new buyer", () => {
    const { service, advance, store } = fixture(); enroll(service, actor()); advance(1000); consent(service, actor(), configuration(), "usage");
    expect(service.recordUsage({ accountId: "account-a", bytes: 100, frames: 1, connectedMs: 1000, quality: "measured", measuredAt: START })).toMatchObject({ collected: false, reason: "sample_predates_current_consent" });
    expect(store.exec("SELECT account_id FROM privacy_usage")).toHaveLength(0);
  });
  it("purges withdrawn profiles, usage and expired manifests in bounded indexed batches", () => {
    const { service, advance, store } = fixture(); enroll(service, actor()); consent(service, actor(), configuration(), "usage");
    service.recordUsage({ accountId: "account-a", bytes: 100, frames: 1, connectedMs: 1000, quality: "measured", measuredAt: START }); prepare(service);
    service.withdraw(actor(), { category: "profile" }); advance(31 * DAY); service.cleanup(1);
    expect(store.exec<{ profile: string | null }>("SELECT profile FROM privacy_subjects WHERE account_id='account-a'")[0]?.profile).toBeNull();
    expect(store.exec("SELECT id FROM privacy_manifests")).toHaveLength(0);
    advance(61 * DAY); service.cleanup(1); expect(store.exec("SELECT account_id FROM privacy_usage")).toHaveLength(0);
    const plan = store.exec<{ detail: string }>("EXPLAIN QUERY PLAN SELECT rowid FROM privacy_usage WHERE expires_at<=? ORDER BY expires_at LIMIT ?", START, 128);
    expect(plan.some((entry) => entry.detail.includes("privacy_usage_expiry"))).toBe(true);
    expect(() => service.cleanup(257)).toThrow("invalid_cleanup_limit");
  });
  it("suppresses small, overlapping and custom cohort cuts without claiming anonymity", async () => {
    const { service } = fixture(); const ids = Array.from({ length: 20 }, (_, index) => `subject-${index}`);
    ids.forEach((id, index) => enroll(service, actor(id), configuration(), index < 10 ? "engineering" : "design"));
    const small = prepare(service, ids.slice(0, 10)); expect(await service.cohort(operator, { manifestId: small.manifestId, approved: true })).toMatchObject({ suppressed: true });
    const manifest = prepare(service, ids); const report = await service.cohort(operator, { manifestId: manifest.manifestId, approved: true }) as { payload: unknown };
    expect(report.payload).toMatchObject({ classification: "personal-data", anonymous: false, cells: [{ value: "engineering", count: 10 }, { value: "design", count: 10 }] });
    const overlapping = prepare(service, ids); await expect(service.cohort(operator, { manifestId: overlapping.manifestId, approved: true })).rejects.toThrow("overlapping_cohort_forbidden");
  });
  it("purges profiles after buyer approval expires and preserves retention when commercial activation is removed", () => {
    const config = configuration(); config.buyers[0]!.expiresAt = START + DAY; config.retention.profileWithdrawalDays = 10;
    const { service, store, advance, clock } = fixture(config); enroll(service, actor(), config);
    advance(12 * DAY); const disabled = new PrivacyService(store, null, clock); disabled.cleanup();
    expect(store.exec<{ profile: string | null }>("SELECT profile FROM privacy_subjects WHERE account_id='account-a'")[0]?.profile).toBeNull();
    expect(disabled.status(actor())).toMatchObject({ programEnabled: false });
    expect(store.exec<{ retention: string }>("SELECT retention FROM privacy_policy WHERE id=1")[0]?.retention).toContain('"profileWithdrawalDays":10');
  });
});

describe("rights isolation, deadlines and clear notices", () => {
  it("returns only the current account, tracks deadlines and prevents premature completion", async () => {
    const { service, advance } = fixture(); enroll(service, actor());
    const request = await service.rightsRequest(actor(), { kind: "export", jurisdiction: "fixture" }) as { requestId: string; responseDue: number };
    expect(service.rightsList(actor("another"))).toEqual([]);
    expect(() => service.rightsUpdate(actor("another"), { requestId: request.requestId, action: "extend", notice: "Need more time" })).toThrow("operator_required");
    expect(service.rightsUpdate(operator, { requestId: request.requestId, action: "extend", notice: "We need time to gather recorded recipient responses." })).toMatchObject({ extended: 1, extension_notice: "We need time to gather recorded recipient responses." });
    expect(() => service.rightsUpdate(operator, { requestId: request.requestId, action: "extend", notice: "Again" })).toThrow("extension_not_allowed");
    advance(61 * DAY); expect(service.rightsList(actor())[0]).toMatchObject({ responseOverdue: true });
    expect(service.cleanup()).toMatchObject({ overdueRights: [{ id: request.requestId }] });
    service.operationalUpdate({ requestId: request.requestId, state: "completed", result: { identity: "account owner only" } });
    expect(service.rightsUpdate(operator, { requestId: request.requestId, action: "complete" })).toMatchObject({ state: "completed" });
  });
  it("closes transfers before acknowledging and preserves cross-store work after retry", async () => {
    const { service, store } = fixture(); enroll(service, actor()); const manifest = prepare(service);
    const request = await service.rightsRequest(actor(), { kind: "close", jurisdiction: "fixture" }) as { requestId: string };
    expect(service.status(actor())).toMatchObject({ closed: true, optedOut: true });
    expect(store.exec<{ state: string }>("SELECT state FROM privacy_manifests WHERE id=?", manifest.manifestId)[0]?.state).toBe("invalidated");
    expect(service.operationalTasks()).toMatchObject([{ requestId: request.requestId, action: "close" }]);
    expect(new PrivacyService(store, configuration(), () => START).operationalTasks()).toMatchObject([{ requestId: request.requestId }]);
  });
  it("tracks a closed account request with a private hash-backed status key that grants no other access", async () => {
    const { service, store } = fixture(); enroll(service, actor());
    const request = await service.rightsRequest(actor(), { kind: "close", jurisdiction: "fixture" }) as { requestId: string; statusKey: string };
    expect(service.status(actor())).toMatchObject({ closed: true });
    expect(request.statusKey).toMatch(/^[a-f0-9]{64}$/u);
    const persisted = store.exec<{ status_token_hash: string }>("SELECT status_token_hash FROM privacy_rights WHERE id=?", request.requestId)[0]!;
    expect(persisted.status_token_hash).not.toBe(request.statusKey);
    expect(await service.rightsStatus({ requestId: request.requestId, statusKey: request.statusKey })).toMatchObject({ id: request.requestId, kind: "close", state: "acknowledged" });
    await expect(service.rightsStatus({ requestId: request.requestId, statusKey: "0".repeat(64) })).rejects.toThrow("request_not_found");
    await expect(service.rightsStatus({ requestId: "another-request", statusKey: request.statusKey })).rejects.toThrow("request_not_found");
    expect(JSON.stringify(await service.rightsStatus({ requestId: request.requestId, statusKey: request.statusKey }))).not.toContain("account-a");
  });
  it("offers accessible unchecked choices and a public optout without identity fields", () => {
    const { service } = fixture(); const who = actor(); const html = accountPage(service.status(who));
    expect(html).toContain('type="checkbox"'); expect(html).not.toContain(" checked"); expect(html).toContain("Refusing or withdrawing leaves relay access, price and allowance unchanged");
    expect(html).not.toContain('action="/api/privacy/profile"');
    enroll(service, who);
    expect(accountPage(service.status(who))).toContain('action="/api/privacy/profile"');
    service.optOut(who);
    const optedOut = accountPage(service.status(who));
    expect(optedOut).not.toContain('action="/api/privacy/profile"'); expect(optedOut).toContain('action="/api/privacy/rejoin"');
    expect(accountPage(service.status({ ...who, gpc: true }), true)).not.toContain('action="/api/privacy/rejoin"');
    const publicPage = optOutPage(false); expect(publicPage).toContain("No account or identity check"); expect(publicPage).not.toContain('name="email"');
  });
});
