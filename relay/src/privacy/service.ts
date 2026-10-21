import { approvedBuyer, buyerScope, DEFAULT_RETENTION, requireConfig, type Buyer, type CommercialConfig } from "./config.js";
import { category, directProfile, exactKeys, fieldSet, object, PrivacyError, text, usageProjection, usageSample, type Category, type CommercialField, type Profile } from "./schema.js";
import { initialize, type Binding, type SqlStore } from "./store.js";

const DAY = 86_400_000;
export const MAX_EXPORT = 128;
const MAX_RIGHTS = 32;
export interface OperationalTaskCursor { requestedAt: number; requestId: string }
export interface Actor { accountId: string | null; browserId: string | null; operator: boolean; gpc: boolean }
interface Subject extends Record<string, unknown> { account_id: string; generation: number; adult: number; opted_out: number; opted_out_at: number | null; closed: number; profile: string | null; profile_purge_at: number | null; updated_at: number }
interface Consent extends Record<string, unknown> { account_id: string; buyer_id: string; category: Category; scope: string; accepted_at: number; withdrawn_at: number | null; expires_at: number }
interface Manifest extends Record<string, unknown> { id: string; operator_id: string; buyer_id: string; category: Category; fields: string; scope: string; candidates: string; state: string; created_at: number; expires_at: number }
interface Candidate { id: string; generation: number }
interface Usage extends Record<string, unknown> { month: string; bytes: number; frames: number; connected_ms: number; active_days: string; os_family: string | null; cli_major: number | null; quality: "measured" | "estimated"; expires_at: number }
interface UsageReference { accountId: string; month: string; quality: Usage["quality"]; expiresAt: number }
interface Rights extends Record<string, unknown> { id: string; account_id: string; kind: string; jurisdiction: string; state: string; requested_at: number; acknowledged_at: number | null; acknowledgement_due: number; response_due: number; extended: number; extension_notice: string | null; retained_exceptions: string; recipients: string; operational_state: string; completed_at: number | null; expires_at: number }

/** One SQLite-backed authority serializes every privacy write and disclosure commit. */
export class PrivacyService {
  private readonly retentionPolicy: CommercialConfig["retention"];
  constructor(private readonly store: SqlStore, private readonly config: CommercialConfig | null, private readonly clock: () => number = Date.now) {
    initialize(store);
    if (config !== null) store.exec("INSERT INTO privacy_policy(id,retention) VALUES(1,?) ON CONFLICT(id) DO UPDATE SET retention=excluded.retention", JSON.stringify(config.retention));
    const previous = store.exec<{ retention: string }>("SELECT retention FROM privacy_policy WHERE id=1")[0]?.retention;
    this.retentionPolicy = config?.retention ?? (previous === undefined ? DEFAULT_RETENTION : JSON.parse(previous) as CommercialConfig["retention"]);
  }
  private now(): number { return this.clock(); }
  private rows<T extends Record<string, unknown>>(sql: string, ...bindings: Binding[]): T[] { return this.store.exec<T>(sql, ...bindings); }
  private retention() { return this.retentionPolicy; }
  private account(actor: Actor): string { if (!actor.accountId) throw new PrivacyError("authentication_required", 401); return text(actor.accountId); }
  private operator(actor: Actor): string { const id = this.account(actor); if (!actor.operator) throw new PrivacyError("operator_required", 403); return id; }
  private ensure(id: string): Subject {
    this.store.exec("INSERT OR IGNORE INTO privacy_subjects(account_id,updated_at) VALUES(?,?)", id, this.now());
    const subject = this.rows<Subject>("SELECT account_id,generation,adult,opted_out,opted_out_at,closed,profile,profile_purge_at,updated_at FROM privacy_subjects WHERE account_id=?", id)[0];
    if (!subject) throw new PrivacyError("subject_unavailable", 500);
    return subject;
  }
  private bump(id: string): void { this.store.exec("UPDATE privacy_subjects SET generation=generation+1,updated_at=? WHERE account_id=?", this.now(), id); this.invalidate(id); }
  private invalidate(id: string): void {
    this.store.exec("UPDATE privacy_manifests SET state='invalidated' WHERE state='prepared' AND EXISTS(SELECT 1 FROM json_each(candidates) WHERE json_extract(value,'$.id')=?)", id);
  }
  private evidence(id: string, kind: string, action: string, scope: string): void {
    this.store.exec("INSERT INTO privacy_evidence(id,account_id,category,action,scope,created_at,expires_at) VALUES(?,?,?,?,?,?,?)", crypto.randomUUID(), id, kind, action, scope, this.now(), this.now() + this.retention().consentDays * DAY);
  }
  private browserOut(id: string | null): boolean { return id !== null && this.rows("SELECT browser_id FROM privacy_browsers WHERE browser_id=? AND opted_out=1 AND expires_at>?", id, this.now()).length > 0; }
  private blocked(subject: Subject, actor?: Actor): boolean { return subject.adult !== 1 || subject.opted_out === 1 || subject.closed === 1 || actor?.gpc === true || this.browserOut(actor?.browserId ?? null); }
  private consent(id: string, buyer: Buyer, kind: Category): Consent | null {
    const row = this.rows<Consent>("SELECT account_id,buyer_id,category,scope,accepted_at,withdrawn_at,expires_at FROM privacy_consents WHERE account_id=? AND buyer_id=? AND category=?", id, buyer.id, kind)[0];
    return row && row.withdrawn_at === null && row.expires_at > this.now() && row.scope === buyerScope(buyer, kind) ? row : null;
  }
  private anyConsent(id: string, kind: Category): boolean {
    return this.config?.buyers.some((buyer) => buyer.approvedAt <= this.now() && buyer.expiresAt > this.now() && this.consent(id, buyer, kind) !== null) ?? false;
  }
  private profileExpiry(id: string): number {
    const expiries = this.config?.buyers.map((buyer) => this.consent(id, buyer, "profile")?.expires_at ?? 0) ?? [];
    return Math.max(this.now(), ...expiries) + this.retention().profileWithdrawalDays * DAY;
  }
  private eligible(id: string, buyer: Buyer, kind: Category, generation?: number): Subject {
    const subject = this.ensure(id);
    if (this.blocked(subject) || (generation !== undefined && subject.generation !== generation) || !this.consent(id, buyer, kind)) throw new PrivacyError("export_eligibility_changed", 409);
    return subject;
  }

  status(actor: Actor): Record<string, unknown> {
    const id = this.account(actor);
    const subject = this.ensure(id);
    const suppressUntil = subject.opted_out_at === null ? null : subject.opted_out_at + 366 * DAY;
    return {
      programEnabled: this.config !== null, adultAffirmed: subject.adult === 1, optedOut: subject.opted_out === 1 || this.browserOut(actor.browserId) || actor.gpc,
      closed: subject.closed === 1, generation: subject.generation, solicitAfter: suppressUntil,
      maySolicit: subject.closed !== 1 && !actor.gpc && (suppressUntil === null || this.now() >= suppressUntil) && !this.browserOut(actor.browserId),
      profile: subject.profile === null ? null : { fields: directProfile(JSON.parse(subject.profile)), provenance: "direct-user", quality: "self-reported" },
      notices: this.config?.buyers.filter((buyer) => buyer.approvedAt <= this.now() && buyer.expiresAt > this.now()).map((buyer) => ({ id: buyer.id, name: buyer.name, purpose: buyer.purpose, noticeVersion: buyer.noticeVersion, noticeUrl: buyer.noticeUrl, fields: buyer.fields, retentionDays: buyer.retentionDays, scopes: { profile: buyerScope(buyer, "profile"), usage: buyerScope(buyer, "usage") } })) ?? [],
      consents: this.rows<Consent>("SELECT account_id,buyer_id,category,scope,accepted_at,withdrawn_at,expires_at FROM privacy_consents WHERE account_id=? LIMIT 64", id).map((entry) => ({ buyerId: entry.buyer_id, category: entry.category, acceptedAt: entry.accepted_at, withdrawnAt: entry.withdrawn_at, expiresAt: entry.expires_at, current: this.config?.buyers.some((buyer) => buyer.id === entry.buyer_id && this.consent(id, buyer, entry.category) !== null) ?? false })),
      serviceAccessChanged: false,
    };
  }

  adult(actor: Actor, input: unknown): unknown {
    requireConfig(this.config);
    const value = object(input); exactKeys(value, ["adult"]); if (typeof value.adult !== "boolean") throw new PrivacyError("adult_affirmation_required");
    const id = this.account(actor);
    return this.store.transaction(() => {
      if (this.ensure(id).closed === 1) throw new PrivacyError("account_closed", 403);
      this.store.exec("UPDATE privacy_subjects SET adult=? WHERE account_id=?", value.adult ? 1 : 0, id);
      this.store.exec("UPDATE privacy_subjects SET usage_since=? WHERE account_id=?", this.now(), id);
      this.store.exec("DELETE FROM privacy_usage WHERE account_id=?", id);
      if (!value.adult) this.suppressData(id, "ineligible");
      this.bump(id); this.evidence(id, "eligibility", value.adult ? "adult-affirmed" : "ineligible", "self-affirmation; no birth date collected");
      return { adultAffirmed: value.adult };
    });
  }

  acceptConsent(actor: Actor, input: unknown): unknown {
    const value = object(input); exactKeys(value, ["buyerId", "category", "noticeVersion", "noticeScope", "fields", "accepted"]);
    const id = this.account(actor); const kind = category(value.category); const buyer = approvedBuyer(this.config, text(value.buyerId), this.now());
    const fields = fieldSet(value.fields, kind);
    if (value.accepted !== true || value.noticeVersion !== buyer.noticeVersion || value.noticeScope !== buyerScope(buyer, kind) || JSON.stringify(fields) !== JSON.stringify([...buyer.fields[kind]].sort())) throw new PrivacyError("notice_scope_mismatch", 409);
    return this.store.transaction(() => {
      if (this.blocked(this.ensure(id), actor)) throw new PrivacyError("participation_ineligible", 403);
      const scope = buyerScope(buyer, kind);
      this.store.exec("INSERT INTO privacy_consents(account_id,buyer_id,category,scope,accepted_at,withdrawn_at,expires_at) VALUES(?,?,?,?,?,NULL,?) ON CONFLICT(account_id,buyer_id,category) DO UPDATE SET scope=excluded.scope,accepted_at=excluded.accepted_at,withdrawn_at=NULL,expires_at=excluded.expires_at", id, buyer.id, kind, scope, this.now(), buyer.expiresAt);
      if (kind === "usage") { this.store.exec("UPDATE privacy_subjects SET usage_since=? WHERE account_id=?", this.now(), id); this.store.exec("DELETE FROM privacy_usage WHERE account_id=?", id); }
      this.evidence(id, kind, "consented", scope); this.bump(id);
      if (kind === "profile") this.store.exec("UPDATE privacy_subjects SET profile_purge_at=? WHERE account_id=? AND profile IS NOT NULL", this.profileExpiry(id), id);
      return { buyerId: buyer.id, category: kind, accepted: true, serviceAccessChanged: false };
    });
  }

  profile(actor: Actor, input: unknown): unknown {
    requireConfig(this.config); const id = this.account(actor); const profile = directProfile(input);
    return this.store.transaction(() => {
      const subject = this.ensure(id);
      if (this.blocked(subject, actor) || !this.anyConsent(id, "profile")) throw new PrivacyError("profile_consent_required", 403);
      const supplied = Object.keys(profile);
      if (!this.config?.buyers.some((buyer) => this.consent(id, buyer, "profile") && supplied.every((field) => buyer.fields.profile.includes(field as CommercialField)))) throw new PrivacyError("profile_fields_not_consented", 403);
      this.store.exec("UPDATE privacy_subjects SET profile=?,profile_purge_at=? WHERE account_id=?", JSON.stringify({ provenance: "direct-user", fields: profile }), this.profileExpiry(id), id);
      this.bump(id); return { fields: profile, provenance: "direct-user", quality: "self-reported" };
    });
  }

  private suppressData(id: string, reason: string): void {
    this.store.exec("UPDATE privacy_consents SET withdrawn_at=COALESCE(withdrawn_at,?) WHERE account_id=?", this.now(), id);
    this.store.exec("UPDATE privacy_subjects SET profile_purge_at=? WHERE account_id=? AND profile IS NOT NULL", this.now() + this.retention().profileWithdrawalDays * DAY, id);
    this.store.exec("DELETE FROM privacy_usage WHERE account_id=?", id);
    this.store.exec("DELETE FROM privacy_rejoins WHERE account_id=?", id);
    this.evidence(id, "all", reason, "all current commercial permissions suppressed");
  }

  withdraw(actor: Actor, input: unknown): unknown {
    const id = this.account(actor); const value = object(input); exactKeys(value, ["category", "buyerId"]); const kind = category(value.category);
    const buyerId = value.buyerId === undefined ? null : text(value.buyerId);
    return this.store.transaction(() => {
      this.ensure(id);
      this.store.exec("UPDATE privacy_consents SET withdrawn_at=COALESCE(withdrawn_at,?) WHERE account_id=? AND category=? AND (? IS NULL OR buyer_id=?)", this.now(), id, kind, buyerId, buyerId);
      this.evidence(id, kind, "withdrawn", buyerId ?? "all buyers"); this.bump(id);
      if (kind === "usage") { this.store.exec("DELETE FROM privacy_usage WHERE account_id=?", id); this.store.exec("UPDATE privacy_subjects SET usage_since=? WHERE account_id=?", this.now(), id); }
      if (kind === "profile" && !this.anyConsent(id, "profile")) this.store.exec("UPDATE privacy_subjects SET profile_purge_at=? WHERE account_id=? AND profile IS NOT NULL", this.now() + this.retention().profileWithdrawalDays * DAY, id);
      else if (kind === "profile") this.store.exec("UPDATE privacy_subjects SET profile_purge_at=? WHERE account_id=? AND profile IS NOT NULL", this.profileExpiry(id), id);
      return { withdrawn: true, serviceAccessChanged: false };
    });
  }

  optOut(actor: Actor): unknown {
    return this.store.transaction(() => {
      const browserId = actor.browserId;
      if (browserId) this.store.exec("INSERT INTO privacy_browsers(browser_id,opted_out,opted_out_at,expires_at) VALUES(?,1,?,?) ON CONFLICT(browser_id) DO UPDATE SET opted_out=1,expires_at=excluded.expires_at", browserId, this.now(), this.now() + this.retention().browserDays * DAY);
      if (actor.accountId) {
        const subject = this.ensure(actor.accountId);
        if (subject.opted_out !== 1) {
          this.store.exec("UPDATE privacy_subjects SET opted_out=1,opted_out_at=? WHERE account_id=?", this.now(), actor.accountId);
          this.suppressData(actor.accountId, actor.gpc ? "gpc-optout" : "public-optout"); this.bump(actor.accountId);
        }
      }
      return { optedOut: true, confirmation: "Your sale and sharing preference is saved. Relay access and price are unchanged.", loginRequired: false };
    });
  }

  rejoin(actor: Actor, input: unknown): unknown {
    requireConfig(this.config); const id = this.account(actor); const value = object(input); exactKeys(value, ["step", "token", "confirmed"]);
    if (actor.gpc) throw new PrivacyError("active_gpc", 403);
    return this.store.transaction(() => {
      const subject = this.ensure(id); if (subject.closed === 1 || subject.adult !== 1) throw new PrivacyError("participation_ineligible", 403);
      if (value.step === "request") {
        const token = crypto.randomUUID(); this.store.exec("DELETE FROM privacy_rejoins WHERE account_id=?", id);
        this.store.exec("INSERT INTO privacy_rejoins(token,account_id,browser_id,expires_at) VALUES(?,?,?,?)", token, id, actor.browserId, this.now() + 600_000);
        return { token, next: "Review the current named buyer notice, then confirm in a separate action. Sales remain blocked.", salesEnabled: false };
      }
      if (value.step !== "confirm" || value.confirmed !== true) throw new PrivacyError("separate_confirmation_required");
      const token = text(value.token);
      const pending = this.rows("SELECT token FROM privacy_rejoins WHERE token=? AND account_id=? AND browser_id IS ? AND expires_at>?", token, id, actor.browserId, this.now());
      if (pending.length !== 1) throw new PrivacyError("rejoin_expired", 409);
      this.store.exec("DELETE FROM privacy_rejoins WHERE account_id=?", id);
      this.store.exec("UPDATE privacy_subjects SET opted_out=0 WHERE account_id=?", id);
      if (actor.browserId) this.store.exec("DELETE FROM privacy_browsers WHERE browser_id=?", actor.browserId);
      this.evidence(id, "all", "rejoined", "user initiated two separate affirmative steps; fresh category consent still required"); this.bump(id);
      return { rejoined: true, salesEnabled: false, requiresFreshConsent: true };
    });
  }

  recordUsage(input: unknown): unknown {
    const config = this.config; if (config === null) return { collected: false };
    const sample = usageSample(input); const id = sample.accountId;
    if (sample.measuredAt > this.now() + 60_000 || sample.measuredAt < this.now() - DAY) throw new PrivacyError("stale_usage_sample");
    return this.store.transaction(() => {
      const now = this.now();
      const subject = this.rows<Subject>("SELECT account_id,generation,adult,opted_out,opted_out_at,closed,profile,profile_purge_at,updated_at FROM privacy_subjects WHERE account_id=?", id)[0];
      if (!subject || this.blocked(subject)) return { collected: false };
      const fields = new Set<CommercialField>();
      for (const buyer of config.buyers) {
        if (buyer.approvedAt <= now && buyer.expiresAt > now && this.consent(id, buyer, "usage")) for (const field of buyer.fields.usage) fields.add(field);
      }
      if (fields.size === 0) return { collected: false };
      const since = this.rows<{ usage_since: number }>("SELECT usage_since FROM privacy_subjects WHERE account_id=?", id)[0]?.usage_since ?? now;
      if (sample.measuredAt < since) return { collected: false, reason: "sample_predates_current_consent" };
      const date = new Date(sample.measuredAt).toISOString(); const month = date.slice(0, 7); const day = date.slice(8, 10);
      const existing = this.rows<Usage>("SELECT month,bytes,frames,connected_ms,active_days,os_family,cli_major,quality,expires_at FROM privacy_usage WHERE account_id=? AND month=? AND quality=? AND expires_at>?", id, month, sample.quality, now)[0];
      const days = new Set<string>(fields.has("activeDays") && existing ? JSON.parse(existing.active_days) as string[] : []);
      if (fields.has("activeDays") && (sample.frames > 0 || sample.connectedMs > 0)) days.add(day);
      const bytes = fields.has("traffic") ? (existing?.bytes ?? 0) + sample.bytes : 0;
      const ms = fields.has("connectedHours") ? (existing?.connected_ms ?? 0) + sample.connectedMs : 0;
      if (![bytes, ms].every(Number.isSafeInteger)) throw new PrivacyError("usage_counter_overflow");
      const os = fields.has("osFamily") ? sample.osFamily ?? existing?.os_family ?? null : null;
      const cli = fields.has("cliMajor") ? sample.cliMajor ?? existing?.cli_major ?? null : null;
      const expiry = now + this.retention().usageDays * DAY;
      const expiresAt = existing ? Math.min(existing.expires_at, expiry) : expiry;
      this.store.exec("INSERT INTO privacy_usage(account_id,month,bytes,frames,connected_ms,active_days,os_family,cli_major,quality,expires_at) VALUES(?,?,?,?,?,?,?,?,?,?) ON CONFLICT(account_id,month,quality) DO UPDATE SET bytes=excluded.bytes,frames=excluded.frames,connected_ms=excluded.connected_ms,active_days=excluded.active_days,os_family=excluded.os_family,cli_major=excluded.cli_major,expires_at=excluded.expires_at", id, month, bytes, 0, ms, JSON.stringify([...days].sort()), os, cli, sample.quality, expiresAt);
      this.bump(id); return { collected: true, quality: sample.quality };
    });
  }

  prepare(actor: Actor, input: unknown): unknown {
    const operator = this.operator(actor); const value = object(input); exactKeys(value, ["buyerId", "category", "fields", "accountIds", "approved"]);
    if (value.approved !== true) throw new PrivacyError("operator_approval_required", 403);
    const kind = category(value.category); const buyer = approvedBuyer(this.config, text(value.buyerId), this.now()); const fields = fieldSet(value.fields, kind);
    if (fields.some((field) => !buyer.fields[kind].includes(field))) throw new PrivacyError("buyer_fields_not_approved", 403);
    if (!Array.isArray(value.accountIds) || value.accountIds.length < 1 || value.accountIds.length > MAX_EXPORT) throw new PrivacyError("export_bound_exceeded");
    const ids = value.accountIds.map((id) => text(id)); if (new Set(ids).size !== ids.length) throw new PrivacyError("duplicate_subject");
    return this.store.transaction(() => {
      const candidates = ids.map((id) => { const subject = this.eligible(id, buyer, kind); return { id, generation: subject.generation }; });
      for (const candidate of candidates) if (this.disclosures(candidate.id).length >= 512) throw new PrivacyError("disclosure_rights_capacity_reached", 429);
      const id = crypto.randomUUID(); const expiry = this.now() + this.retention().stagingHours * 3_600_000;
      this.store.exec("INSERT INTO privacy_manifests(id,operator_id,buyer_id,category,fields,scope,candidates,state,created_at,expires_at) VALUES(?,?,?,?,?,?,?,'prepared',?,?)", id, operator, buyer.id, kind, JSON.stringify(fields), buyerScope(buyer, kind), JSON.stringify(candidates), this.now(), expiry);
      return { manifestId: id, candidateCount: candidates.length, expiresAt: expiry, containsDownloadableData: false };
    });
  }

  private manifest(id: string, operator: string): Manifest {
    const manifest = this.rows<Manifest>("SELECT id,operator_id,buyer_id,category,fields,scope,candidates,state,created_at,expires_at FROM privacy_manifests WHERE id=?", id)[0];
    if (!manifest || manifest.operator_id !== operator) throw new PrivacyError("manifest_not_found", 404);
    if (manifest.state !== "prepared" || manifest.expires_at <= this.now()) throw new PrivacyError("manifest_not_releasable", 409);
    return manifest;
  }
  private projection(id: string, buyer: Buyer, kind: Category, fields: CommercialField[], includedUsage: UsageReference[]): Record<string, unknown> {
    const subject = this.eligible(id, buyer, kind);
    let pseudonym = this.rows<{ pseudonym: string }>("SELECT pseudonym FROM privacy_pseudonyms WHERE account_id=? AND buyer_id=?", id, buyer.id)[0]?.pseudonym;
    if (!pseudonym) { pseudonym = crypto.randomUUID(); this.store.exec("INSERT INTO privacy_pseudonyms(account_id,buyer_id,pseudonym,expires_at) VALUES(?,?,?,?)", id, buyer.id, pseudonym, this.now() + this.retention().disclosureDays * DAY); }
    else this.store.exec("UPDATE privacy_pseudonyms SET expires_at=MAX(expires_at,?) WHERE account_id=? AND buyer_id=?", this.now() + this.retention().disclosureDays * DAY, id, buyer.id);
    if (kind === "profile") {
      if (subject.profile === null || (subject.profile_purge_at !== null && subject.profile_purge_at <= this.now())) throw new PrivacyError("profile_unavailable", 409);
      const profile = directProfile(JSON.parse(subject.profile));
      const projection = Object.fromEntries(fields.filter((field) => profile[field as keyof Profile] !== undefined).map((field) => [field, profile[field as keyof Profile]]));
      if (Object.keys(projection).length === 0) throw new PrivacyError("profile_unavailable", 409);
      return { subject: pseudonym, quality: "self-reported", provenance: "direct-user", fields: projection };
    }
    const summaries = this.rows<Usage>("SELECT month,bytes,frames,connected_ms,active_days,os_family,cli_major,quality,expires_at FROM privacy_usage WHERE account_id=? AND expires_at>? ORDER BY month LIMIT 8", id, this.now());
    if (summaries.length === 0) throw new PrivacyError("usage_unavailable", 409);
    for (const summary of summaries) includedUsage.push({ accountId: id, month: summary.month, quality: summary.quality, expiresAt: summary.expires_at });
    return { subject: pseudonym, summaries: summaries.map((summary) => ({ month: summary.month, quality: summary.quality, provenance: summary.quality === "measured" ? "server-forwarding-counters" : "reservation-estimate", fields: usageProjection({ bytes: summary.bytes, connectedMs: summary.connected_ms, days: (JSON.parse(summary.active_days) as string[]).length, osFamily: summary.os_family, cliMajor: summary.cli_major }, fields) })) };
  }

  async release(actor: Actor, input: unknown): Promise<unknown> {
    const operator = this.operator(actor); const value = object(input); exactKeys(value, ["manifestId", "approved"]);
    if (value.approved !== true) throw new PrivacyError("operator_approval_required", 403);
    const id = text(value.manifestId); const manifest = this.manifest(id, operator); const buyer = approvedBuyer(this.config, manifest.buyer_id, this.now());
    if (manifest.scope !== buyerScope(buyer, manifest.category)) throw new PrivacyError("buyer_scope_changed", 409);
    const candidates = JSON.parse(manifest.candidates) as Candidate[]; const fields = fieldSet(JSON.parse(manifest.fields), manifest.category);
    const includedUsage: UsageReference[] = [];
    const records = this.store.transaction(() => candidates.map((candidate) => { this.eligible(candidate.id, buyer, manifest.category, candidate.generation); return this.projection(candidate.id, buyer, manifest.category, fields, includedUsage); }));
    const payload = JSON.stringify({ schema: "ejm-commercial-v1", buyer: buyer.name, purpose: buyer.purpose, category: manifest.category, noticeVersion: buyer.noticeVersion, retentionDays: buyer.retentionDays, dataClassification: "personal-data", records });
    if (new TextEncoder().encode(payload).byteLength > 262_144) throw new PrivacyError("export_bytes_exceeded");
    const digest = [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(payload)))].map((byte) => byte.toString(16).padStart(2, "0")).join("");
    // This synchronous transaction is the release boundary. Every writer uses this authority.
    this.store.transaction(() => {
      const current = this.manifest(id, operator); const currentBuyer = approvedBuyer(this.config, current.buyer_id, this.now());
      if (current.scope !== buyerScope(currentBuyer, current.category)) throw new PrivacyError("buyer_scope_changed", 409);
      for (const candidate of candidates) { this.eligible(candidate.id, currentBuyer, current.category, candidate.generation); if (this.disclosures(candidate.id).length >= 512) throw new PrivacyError("disclosure_rights_capacity_reached", 429); }
      if (includedUsage.length > 0 && this.rows("SELECT 1 AS unavailable FROM json_each(?) AS included WHERE NOT EXISTS(SELECT 1 FROM privacy_usage WHERE account_id=json_extract(included.value,'$.accountId') AND month=json_extract(included.value,'$.month') AND quality=json_extract(included.value,'$.quality') AND expires_at=json_extract(included.value,'$.expiresAt') AND expires_at>?) LIMIT 1", JSON.stringify(includedUsage), this.now()).length > 0) throw new PrivacyError("usage_unavailable", 409);
      this.store.exec("INSERT INTO privacy_ledger(id,manifest_id,buyer_id,purpose,category,fields,notice_version,subjects,digest,released_at,expires_at) VALUES(?,?,?,?,?,?,?,?,?,?,?)", crypto.randomUUID(), id, buyer.id, buyer.purpose, manifest.category, manifest.fields, buyer.noticeVersion, JSON.stringify(candidates.map((candidate) => candidate.id)), digest, this.now(), this.now() + this.retention().disclosureDays * DAY);
      this.store.exec("UPDATE privacy_manifests SET state='released' WHERE id=?", id);
    });
    return { payload: JSON.parse(payload), sha256: digest, releasedAt: this.now(), releaseBoundary: "authorization committed before response delivery; later withdrawal triggers recipient follow-up" };
  }

  access(actor: Actor): unknown {
    const id = this.account(actor);
    return { accountId: id, privacy: this.status(actor), usage: this.rows<Usage>("SELECT month,bytes,frames,connected_ms,active_days,os_family,cli_major,quality,expires_at FROM privacy_usage WHERE account_id=? AND expires_at>? ORDER BY month LIMIT 8", id, this.now()), evidence: this.rows("SELECT category,action,scope,created_at,expires_at FROM privacy_evidence WHERE account_id=? ORDER BY created_at DESC LIMIT 512", id), evidenceMayBeTruncated: this.rows<{ total: number }>("SELECT COUNT(*) AS total FROM privacy_evidence WHERE account_id=?", id)[0]!.total > 512, disclosures: this.disclosures(id), rights: this.rightsList(actor), otherStores: "Use a tracked access or export request to include operational identity, device, quota and payment records and a full paginated evidence export.", classification: "personal-data" };
  }
  evidencePage(actor: Actor, input: unknown): unknown {
    const id = this.account(actor); const value = object(input); exactKeys(value, ["before", "id"]);
    const before = value.before === undefined ? Number.MAX_SAFE_INTEGER : Number(value.before);
    if (!Number.isSafeInteger(before) || before < 0) throw new PrivacyError("invalid_cursor");
    const beforeId = value.id === undefined ? "~" : text(value.id);
    const rows = this.rows<{ id: string; created_at: number }>("SELECT id,category,action,scope,created_at,expires_at FROM privacy_evidence WHERE account_id=? AND (created_at<? OR (created_at=? AND id<?)) ORDER BY created_at DESC,id DESC LIMIT 129", id, before, before, beforeId);
    const page = rows.slice(0, 128); const last = page.at(-1);
    return { records: page, next: rows.length > 128 && last ? { before: last.created_at, id: last.id } : null, classification: "personal-data" };
  }
  private disclosures(id: string): Record<string, unknown>[] {
    return this.rows("SELECT id,buyer_id,purpose,category,fields,notice_version,digest,released_at,expires_at FROM privacy_ledger WHERE EXISTS(SELECT 1 FROM json_each(subjects) WHERE value=?) ORDER BY released_at DESC LIMIT 512", id);
  }

  async rightsRequest(actor: Actor, input: unknown): Promise<unknown> {
    const id = this.account(actor); const value = object(input); exactKeys(value, ["kind", "jurisdiction"]);
    if (!["access", "export", "correction", "delete", "close"].includes(value.kind as string)) throw new PrivacyError("invalid_rights_kind");
    const jurisdiction = text(value.jurisdiction, 64);
    const policy = this.config?.jurisdictionPolicies.find((entry) => entry.jurisdiction === jurisdiction);
    // Conservative fallback keeps rights available while commercial activation is disabled.
    const deadlines = policy ?? { acknowledgementDays: 10, responseDays: 30, extensionDays: 0 };
    const statusKey = [...crypto.getRandomValues(new Uint8Array(32))].map((byte) => byte.toString(16).padStart(2, "0")).join("");
    const statusHash = await sha256(statusKey);
    return this.store.transaction(() => {
      this.ensure(id);
      if (this.rows("SELECT id FROM privacy_rights WHERE account_id=? AND state!='completed' LIMIT 33", id).length >= MAX_RIGHTS) throw new PrivacyError("rights_request_limit", 429);
      if (value.kind === "delete" || value.kind === "close") this.close(id, value.kind === "close");
      const requestId = crypto.randomUUID(); const recipients = this.disclosures(id).map((entry) => ({ disclosureId: entry.id, buyerId: entry.buyer_id, state: "pending", note: "Recipient copies require contractual follow-up." }));
      this.store.exec("INSERT INTO privacy_rights(id,account_id,status_token_hash,kind,jurisdiction,state,requested_at,acknowledged_at,acknowledgement_due,response_due,retained_exceptions,recipients,expires_at) VALUES(?,?,?,?,?,'acknowledged',?,?,?,?,?,?,?)", requestId, id, statusHash, value.kind as string, jurisdiction, this.now(), this.now(), this.now() + deadlines.acknowledgementDays * DAY, this.now() + deadlines.responseDays * DAY, JSON.stringify([{ category: "consent-disclosure-rights", reason: "Restricted evidence retained under the published policy; excluded from sale datasets.", expiresAt: this.now() + this.retention().rightsDays * DAY }, { category: "payment-provider-records", reason: "Statutory and processor retention is reviewed in the operational task; no promise of immediate buyer or backup erasure." }]), JSON.stringify(recipients), this.now() + this.retention().rightsDays * DAY);
      return { requestId, statusKey, statusKeyNotice: "Save this private key to track this request after account closure. It grants only access to this request. It is returned once, never in a URL.", acknowledgedAt: this.now(), acknowledgementDue: this.now() + deadlines.acknowledgementDays * DAY, responseDue: this.now() + deadlines.responseDays * DAY, kind: value.kind, jurisdiction, operationalTask: { requestId, accountId: id, action: value.kind, requestedAt: this.now(), dueAt: this.now() + deadlines.responseDays * DAY, expiresAt: this.now() + this.retention().operationalTaskDays * DAY }, retainedExceptions: "Shown in the tracked request. Already delivered recipient copies require follow-up." };
    });
  }

  async rightsStatus(input: unknown): Promise<unknown> {
    const value = object(input); exactKeys(value, ["requestId", "statusKey"]);
    const key = text(value.statusKey, 64); if (!/^[a-f0-9]{64}$/u.test(key)) throw new PrivacyError("request_not_found", 404);
    const hash = await sha256(key);
    const row = this.rows<Rights>("SELECT id,account_id,kind,jurisdiction,state,requested_at,acknowledged_at,acknowledgement_due,response_due,extended,extension_notice,retained_exceptions,recipients,operational_state,operational_result,completed_at,expires_at FROM privacy_rights WHERE id=? AND status_token_hash=? AND (expires_at>? OR state!='completed')", text(value.requestId), hash, this.now())[0];
    if (!row) throw new PrivacyError("request_not_found", 404);
    const result = this.rightsView(row); delete result.account_id; return result;
  }

  close(id: string, closeAccount = true): unknown {
    return this.store.transaction(() => {
      this.ensure(id); this.suppressData(id, closeAccount ? "account-closed" : "deletion-requested");
      this.store.exec("UPDATE privacy_subjects SET opted_out=1,opted_out_at=COALESCE(opted_out_at,?),closed=CASE WHEN ?=1 THEN 1 ELSE closed END,profile=NULL,profile_purge_at=NULL WHERE account_id=?", this.now(), closeAccount ? 1 : 0, id);
      if (closeAccount) this.store.exec("UPDATE privacy_subjects SET expires_at=? WHERE account_id=?", this.now() + Math.max(this.retention().consentDays, this.retention().disclosureDays, this.retention().rightsDays) * DAY, id);
      this.bump(id); return { closed: closeAccount, commercialTransfersStopped: true, relayRevocationRequired: closeAccount };
    });
  }

  rightsList(actor: Actor): unknown[] { const id = this.account(actor); return this.rows<Rights>("SELECT id,account_id,kind,jurisdiction,state,requested_at,acknowledged_at,acknowledgement_due,response_due,extended,extension_notice,retained_exceptions,recipients,operational_state,operational_result,completed_at,expires_at FROM privacy_rights WHERE account_id=? ORDER BY requested_at DESC LIMIT 128", id).map((row) => this.rightsView(row)); }
  private rightsView(row: Rights): Record<string, unknown> { return { ...row, retained_exceptions: JSON.parse(row.retained_exceptions), recipients: JSON.parse(row.recipients), operational_result: row.operational_result ? JSON.parse(row.operational_result as string) as unknown : null, acknowledgementOverdue: row.acknowledged_at === null && this.now() > row.acknowledgement_due, responseOverdue: row.state !== "completed" && this.now() > row.response_due }; }
  rightsUpdate(actor: Actor, input: unknown): unknown {
    this.operator(actor); const value = object(input); exactKeys(value, ["requestId", "action", "notice", "recipients", "retainedExceptions", "operationalState"]);
    const id = text(value.requestId); const row = this.rows<Rights>("SELECT id,account_id,kind,jurisdiction,state,requested_at,acknowledged_at,acknowledgement_due,response_due,extended,extension_notice,retained_exceptions,recipients,operational_state,operational_result,completed_at,expires_at FROM privacy_rights WHERE id=?", id)[0];
    if (!row) throw new PrivacyError("request_not_found", 404);
    return this.store.transaction(() => {
      if (value.action === "extend") {
        const policy = this.config?.jurisdictionPolicies.find((entry) => entry.jurisdiction === row.jurisdiction);
        if (!policy || policy.extensionDays === 0 || row.extended !== 0 || this.now() > row.response_due || row.state === "completed") throw new PrivacyError("extension_not_allowed", 409);
        this.store.exec("UPDATE privacy_rights SET response_due=response_due+?,extended=1,extension_notice=? WHERE id=?", policy.extensionDays * DAY, text(value.notice, 2048), id);
      } else if (value.action === "progress") {
        const recipients = value.recipients === undefined ? JSON.parse(row.recipients) as unknown[] : this.recipientProgress(value.recipients, row);
        const exceptions = value.retainedExceptions === undefined ? JSON.parse(row.retained_exceptions) as unknown[] : this.exceptions(value.retainedExceptions);
        if (value.operationalState !== undefined && !["pending", "completed", "exception"].includes(value.operationalState as string)) throw new PrivacyError("invalid_operational_state");
        this.store.exec("UPDATE privacy_rights SET recipients=?,retained_exceptions=?,operational_state=? WHERE id=?", JSON.stringify(recipients), JSON.stringify(exceptions), value.operationalState as string ?? row.operational_state, id);
      } else if (value.action === "complete") {
        if (row.operational_state === "pending" || (JSON.parse(row.recipients) as { state: string }[]).some((recipient) => recipient.state !== "confirmed" && recipient.state !== "exception")) throw new PrivacyError("cross_store_work_incomplete", 409);
        this.store.exec("UPDATE privacy_rights SET state='completed',completed_at=? WHERE id=?", this.now(), id);
      } else throw new PrivacyError("invalid_rights_action");
      const updated = this.rows<Rights>("SELECT id,account_id,kind,jurisdiction,state,requested_at,acknowledged_at,acknowledgement_due,response_due,extended,extension_notice,retained_exceptions,recipients,operational_state,operational_result,completed_at,expires_at FROM privacy_rights WHERE id=?", id)[0];
      if (!updated) throw new PrivacyError("request_unavailable", 500); return this.rightsView(updated);
    });
  }
  private recipientProgress(input: unknown, row: Rights): unknown[] {
    if (!Array.isArray(input) || input.length > 512) throw new PrivacyError("invalid_recipient_progress");
    const previous = JSON.parse(row.recipients) as { disclosureId: string; buyerId: string }[];
    const result = input.map((entry) => { const item = object(entry); exactKeys(item, ["disclosureId", "buyerId", "state", "note"]); if (!["pending", "requested", "confirmed", "exception"].includes(item.state as string) || !previous.some((old) => old.disclosureId === item.disclosureId && old.buyerId === item.buyerId)) throw new PrivacyError("unknown_recipient"); return { disclosureId: text(item.disclosureId), buyerId: text(item.buyerId), state: item.state, note: text(item.note, 1024) }; });
    if (result.length !== previous.length || new Set(result.map((entry) => entry.disclosureId)).size !== previous.length) throw new PrivacyError("recipient_followup_cannot_be_dropped");
    return result;
  }
  private exceptions(input: unknown): unknown[] {
    if (!Array.isArray(input) || input.length > 32) throw new PrivacyError("invalid_retained_exceptions");
    return input.map((entry) => { const value = object(entry); exactKeys(value, ["category", "reason", "expiresAt"]); if (typeof value.expiresAt !== "number" || !Number.isSafeInteger(value.expiresAt) || value.expiresAt <= this.now()) throw new PrivacyError("exception_expiry_required"); return { category: text(value.category, 64), reason: text(value.reason, 1024), expiresAt: value.expiresAt }; });
  }

  async cohort(actor: Actor, input: unknown): Promise<unknown> {
    this.operator(actor); const value = object(input); exactKeys(value, ["manifestId", "approved"]);
    if (value.approved !== true) throw new PrivacyError("operator_approval_required", 403);
    const manifest = this.manifest(text(value.manifestId), this.account(actor)); const buyer = approvedBuyer(this.config, manifest.buyer_id, this.now());
    const fields = fieldSet(JSON.parse(manifest.fields), manifest.category);
    // No arbitrary filters, intersections or successive custom cuts are offered.
    if (manifest.category !== "profile" || fields.length !== 1) throw new PrivacyError("revealing_cohort_cut", 403);
    const candidates = JSON.parse(manifest.candidates) as Candidate[];
    const counts = new Map<string, number>(); for (const candidate of candidates) { const subject = this.eligible(candidate.id, buyer, "profile", candidate.generation); if (!subject.profile) throw new PrivacyError("profile_unavailable", 409); const profile = directProfile(JSON.parse(subject.profile)); const key = profile[fields[0] as keyof Profile] ?? "not-reported"; counts.set(key, (counts.get(key) ?? 0) + 1); }
    // Suppress the whole output if even one cell is small. A complement must not expose it.
    if (candidates.length < 20 || [...counts.values()].some((count) => count < 10)) return { suppressed: true, reason: "small_or_revealing_cohort", classification: "personal-data" };
    const universe = this.rows<{ account_id: string }>("SELECT account_id FROM privacy_subjects WHERE adult=1 AND opted_out=0 AND closed=0 AND profile IS NOT NULL ORDER BY account_id LIMIT 129").filter((row) => this.consent(row.account_id, buyer, "profile") !== null).map((row) => row.account_id).sort();
    if (universe.length > MAX_EXPORT || JSON.stringify(universe) !== JSON.stringify(candidates.map((candidate) => candidate.id).sort())) throw new PrivacyError("custom_cohort_cut_forbidden", 403);
    const payload = { buyer: buyer.name, purpose: buyer.purpose, field: fields[0], classification: "personal-data", anonymous: false, quality: "self-reported", cells: [...counts].map(([value, count]) => ({ value, count })) };
    const digest = [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(JSON.stringify(payload))))].map((byte) => byte.toString(16).padStart(2, "0")).join("");
    return this.store.transaction(() => {
      this.manifest(manifest.id, this.account(actor));
      for (const candidate of candidates) { this.eligible(candidate.id, buyer, "profile", candidate.generation); if (this.disclosures(candidate.id).length >= 512) throw new PrivacyError("disclosure_rights_capacity_reached", 429); }
      // Deny overlapping releases even across buyer or field changes. No custom cuts.
      if (this.rows("SELECT id FROM privacy_cohorts WHERE expires_at>? AND EXISTS(SELECT 1 FROM json_each(members) AS prior JOIN json_each(?) AS current ON prior.value=current.value) LIMIT 1", this.now(), JSON.stringify(universe)).length > 0) throw new PrivacyError("overlapping_cohort_forbidden", 403);
      this.store.exec("INSERT INTO privacy_cohorts(id,buyer_id,field,members,released_at,expires_at) VALUES(?,?,?,?,?,?)", crypto.randomUUID(), buyer.id, fields[0] as string, JSON.stringify(universe), this.now(), this.now() + this.retention().disclosureDays * DAY);
      this.store.exec("INSERT INTO privacy_ledger(id,manifest_id,buyer_id,purpose,category,fields,notice_version,subjects,digest,released_at,expires_at) VALUES(?,?,?,?,?,?,?,?,?,?,?)", crypto.randomUUID(), manifest.id, buyer.id, buyer.purpose, "profile", manifest.fields, buyer.noticeVersion, JSON.stringify(universe), digest, this.now(), this.now() + this.retention().disclosureDays * DAY);
      this.store.exec("UPDATE privacy_manifests SET state='released' WHERE id=?", manifest.id);
      return { payload, sha256: digest };
    });
  }

  operationalTasks(after?: OperationalTaskCursor): Record<string, unknown>[] {
    const rows = this.rows<Rights>("SELECT id,account_id,kind,requested_at,response_due FROM privacy_rights WHERE operational_state='pending' AND (requested_at>? OR (requested_at=? AND id>?)) ORDER BY requested_at,id LIMIT 2", after?.requestedAt ?? -1, after?.requestedAt ?? -1, after?.requestId ?? "");
    // Wrap after a complete pass. Enqueue failures remain pending and return on the next pass.
    if (rows.length === 0 && after) return this.operationalTasks();
    return rows.map((row) => ({ requestId: row.id, accountId: row.account_id, action: row.kind, requestedAt: row.requested_at, dueAt: row.response_due, expiresAt: row.requested_at + this.retention().operationalTaskDays * DAY }));
  }
  operationalUpdate(input: unknown): unknown {
    const value = object(input); exactKeys(value, ["requestId", "state", "result"]);
    if (value.state !== "completed" && value.state !== "exception") throw new PrivacyError("invalid_operational_state");
    const result = JSON.stringify(value.result ?? null); if (result.length > 65_536) throw new PrivacyError("operational_result_too_large");
    this.store.exec("UPDATE privacy_rights SET operational_state=?,operational_result=? WHERE id=?", value.state, result, text(value.requestId));
    return { saved: true };
  }

  cleanup(limit = 128): unknown {
    if (!Number.isInteger(limit) || limit < 1 || limit > 256) throw new PrivacyError("invalid_cleanup_limit");
    const categories = ["privacy_usage", "privacy_rejoins", "privacy_manifests", "privacy_browsers", "privacy_evidence", "privacy_consents", "privacy_ledger", "privacy_pseudonyms", "privacy_rights", "privacy_cohorts"];
    return this.store.transaction(() => {
      // Table names come from this fixed list. Expiry indexes bound each scan.
      for (const table of categories) this.store.exec(`DELETE FROM ${table} WHERE rowid IN (SELECT rowid FROM ${table} WHERE expires_at<=? ${table === "privacy_rights" ? "AND state='completed'" : ""} ORDER BY expires_at LIMIT ?)`, this.now(), limit);
      // A disabled program or changed buyer scope cannot retain an active sale profile forever.
      for (const subject of this.rows<{ account_id: string; profile_purge_at: number | null }>("SELECT account_id,profile_purge_at FROM privacy_subjects WHERE profile IS NOT NULL ORDER BY updated_at LIMIT ?", limit)) {
        if (!this.anyConsent(subject.account_id, "profile")) {
          const purgeAt = this.now() + this.retention().profileWithdrawalDays * DAY;
          if (subject.profile_purge_at === null || subject.profile_purge_at > purgeAt) { this.store.exec("UPDATE privacy_subjects SET profile_purge_at=? WHERE account_id=?", purgeAt, subject.account_id); this.bump(subject.account_id); }
        }
      }
      this.store.exec("UPDATE privacy_subjects SET profile=NULL,profile_purge_at=NULL,generation=generation+1 WHERE account_id IN (SELECT account_id FROM privacy_subjects WHERE profile_purge_at<=? ORDER BY profile_purge_at LIMIT ?)", this.now(), limit);
      this.store.exec("DELETE FROM privacy_subjects WHERE account_id IN (SELECT account_id FROM privacy_subjects WHERE closed=1 AND expires_at<=? ORDER BY expires_at LIMIT ?) AND NOT EXISTS(SELECT 1 FROM privacy_rights WHERE privacy_rights.account_id=privacy_subjects.account_id AND state!='completed')", this.now(), limit);
      return { categories, batchLimitPerCategory: limit, profileBatchLimit: limit, overdueRights: this.rows("SELECT id,account_id,response_due FROM privacy_rights WHERE state!='completed' AND response_due<? ORDER BY response_due LIMIT 128", this.now()) };
    });
  }
}

async function sha256(value: string): Promise<string> { return [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(value)))].map((byte) => byte.toString(16).padStart(2, "0")).join(""); }
