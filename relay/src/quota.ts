import { DurableObject } from "cloudflare:workers";
import { serializedBilling } from "./billing.js";
import type { Env, Lease, TicketScope } from "./types.js";
import { LIMITS } from "./types.js";
import { failure, integer, validId } from "./http.js";

export function windowBoundary(now: number): number { return (Math.floor(now / 86_400_000) + 1) * 86_400_000; }
export function leaseExpiry(now: number): number { return Math.min(now + LIMITS.leaseMs, windowBoundary(now)); }
function time(value: unknown): number {
  if (typeof value === "number") return value;
  if (typeof value === "string") return /^\d+$/.test(value) ? Number(value) : Date.parse(value);
  return 0;
}
export interface Authority { plan: "free" | "paid"; frames: number; bytes: number; expiresAt: number }
export async function authority(env: Env, accountId: string, deviceId: string, sessionId: string): Promise<Authority | null> {
  const startedAt = Date.now();
  const row = await env.DB.prepare('SELECT a.status,a.terms_version,d.revoked_at,s."expiresAt" AS session_expires,b.plan,b.status AS payment_status,b.paid_until FROM relay_accounts a JOIN relay_devices d ON d.account_id=a.id JOIN "session" s ON s.id=d.session_id AND s."userId"=a.id LEFT JOIN relay_billing b ON b.account_id=a.id WHERE a.id=? AND d.id=? AND d.session_id=?').bind(accountId, deviceId, sessionId).first<{ status: string; terms_version: string; revoked_at: number | null; session_expires: unknown; plan: string | null; payment_status: string | null; paid_until: number | null }>();
  if (!row || row.status !== "active" || row.terms_version !== env.TERMS_VERSION || row.revoked_at !== null) return null;
  const now = Date.now();
  // The D1 snapshot cannot extend authority by taking longer to return.
  const expiresAt = Math.min(startedAt + LIMITS.leaseMs, time(row.session_expires));
  if (!Number.isFinite(expiresAt) || expiresAt <= now) return null;
  const paid = row.plan === "paid" && ["active", "trialing"].includes(row.payment_status ?? "") && (row.paid_until ?? 0) > now;
  return { plan: paid ? "paid" : "free", frames: paid ? integer(env.PAID_DAILY_FRAMES, LIMITS.dailyFrames, LIMITS.dailyFrames) : LIMITS.dailyFrames, bytes: paid ? integer(env.PAID_DAILY_BYTES, LIMITS.dailyBytes, LIMITS.dailyBytes) : LIMITS.dailyBytes, expiresAt: paid ? Math.min(expiresAt, row.paid_until ?? 0) : expiresAt };
}
interface ScopeRequest extends TicketScope { hash: string }
interface Window extends Record<string, number> { frames: number; bytes: number; controls: number }
export class AccountQuota extends DurableObject<Env> {
  private serial: Promise<unknown> = Promise.resolve();
  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    ctx.storage.sql.exec("CREATE TABLE IF NOT EXISTS windows(day INTEGER PRIMARY KEY,frames INTEGER NOT NULL,bytes INTEGER NOT NULL,controls INTEGER NOT NULL)");
    ctx.storage.sql.exec("CREATE TABLE IF NOT EXISTS tickets(hash TEXT PRIMARY KEY,scope TEXT NOT NULL,expires_at INTEGER NOT NULL,used INTEGER NOT NULL DEFAULT 0)");
    ctx.storage.sql.exec("CREATE INDEX IF NOT EXISTS tickets_expiry ON tickets(expires_at)");
    ctx.storage.sql.exec("CREATE TABLE IF NOT EXISTS room(singleton INTEGER PRIMARY KEY CHECK(singleton=1),room TEXT NOT NULL,expires_at INTEGER NOT NULL)");
    ctx.storage.sql.exec("CREATE TABLE IF NOT EXISTS retired(room TEXT PRIMARY KEY,expires_at INTEGER NOT NULL)");
    ctx.storage.sql.exec("CREATE INDEX IF NOT EXISTS retired_expiry ON retired(expires_at)");
  }
  override fetch(request: Request): Promise<Response> {
    const work = this.serial.then(() => this.handle(request)).catch(() => failure(503, "quota_authority_unavailable"));
    this.serial = work.catch(() => undefined);
    return work;
  }
  private async handle(request: Request): Promise<Response> {
    const path = new URL(request.url).pathname;
    if (request.method !== "POST") return failure(405, "method_not_allowed");
    const now = Date.now();
    if (path === "/cleanup") {
      this.cleanup(now);
      return Response.json({ cleaned: true });
    }
    const data = await request.json<Record<string, unknown>>();
    if (!validId(data.accountId)) return failure(400, "invalid_scope");
    if (path === "/purge") {
      const account = await this.env.DB.prepare("SELECT status FROM relay_accounts WHERE id=?").bind(data.accountId).first<{ status: string }>();
      if (account?.status !== "closed") return failure(403, "closed_account_required");
      await this.ctx.storage.deleteAll();
      return Response.json({ purged: true });
    }
    if (path === "/access") {
      const windows = this.ctx.storage.sql.exec("SELECT day,frames,bytes,controls FROM windows ORDER BY day DESC LIMIT 3").toArray();
      return Response.json({ kind: "reserved_allowance", measuredTraffic: false, windows });
    }
    if (path === "/rooms") {
      const current = this.ctx.storage.sql.exec<{ room: string; expires_at: number }>("SELECT room,expires_at FROM room WHERE singleton=1 AND expires_at>?", now).toArray()[0];
      return Response.json({ rooms: current ? [{ id: current.room, expiresAt: current.expires_at }] : [] });
    }
    if (path.startsWith("/billing/")) return serializedBilling(this.env, path, data);
    if (path === "/revoke") {
      const row = this.ctx.storage.sql.exec<{ room: string; expires_at: number }>("SELECT room,expires_at FROM room WHERE singleton=1").toArray()[0];
      this.ctx.storage.sql.exec("DELETE FROM room"); this.ctx.storage.sql.exec("DELETE FROM tickets");
      return Response.json({ room: row?.room, expiresAt: row?.expires_at });
    }
    if (path === "/stop") {
      this.cleanup(now);
      if (!validId(data.room)) return failure(403, "room_not_owned");
      const retired = this.ctx.storage.sql.exec<{ expires_at: number }>("SELECT expires_at FROM retired WHERE room=?", data.room).toArray()[0];
      const current = retired ?? this.ctx.storage.sql.exec<{ expires_at: number }>("SELECT expires_at FROM room WHERE room=?", data.room).toArray()[0];
      if (!current) return failure(403, "room_not_owned");
      if (!retired && this.retiredCount() >= LIMITS.dailyControls * 2) return failure(503, "retirement_capacity_exhausted");
      this.ctx.storage.transactionSync(() => {
        this.ctx.storage.sql.exec("INSERT INTO retired(room,expires_at) VALUES(?,?) ON CONFLICT(room) DO NOTHING", data.room as string, current.expires_at);
        this.ctx.storage.sql.exec("DELETE FROM room WHERE room=?", data.room as string);
        this.ctx.storage.sql.exec("DELETE FROM tickets WHERE json_extract(scope,'$.room')=?", data.room as string);
      });
      return Response.json({ stopped: true, expiresAt: current.expires_at });
    }
    this.cleanup(now);
    const day = Math.floor(now / 86_400_000);
    const budget = this.ctx.storage.sql.exec<Window>("SELECT frames,bytes,controls FROM windows WHERE day=?", day).toArray()[0] ?? { frames: 0, bytes: 0, controls: 0 };
    if (budget.controls >= LIMITS.dailyControls) return failure(429, "control_quota_exhausted");
    this.ctx.storage.sql.exec("INSERT INTO windows(day,frames,bytes,controls) VALUES(?,0,0,1) ON CONFLICT(day) DO UPDATE SET controls=controls+1", day);
    if (validId(data.room) && this.ctx.storage.sql.exec("SELECT room FROM retired WHERE room=?", data.room).toArray().length) return failure(403, "room_retired");
    if (path === "/issue") {
      const scope = data as unknown as ScopeRequest;
      if (!validId(scope.deviceId) || !validId(scope.sessionId) || !validId(scope.room) || !/^[a-f0-9]{64}$/.test(scope.hash) || !["host", "controller"].includes(scope.role)) return failure(400, "invalid_scope");
      if (!(await authority(this.env, scope.accountId, scope.deviceId, scope.sessionId))) return failure(403, "account_or_device_inactive");
      const issuedAt = Date.now();
      let current = this.ctx.storage.sql.exec<{ room: string; expires_at: number }>("SELECT room,expires_at FROM room WHERE singleton=1").toArray()[0];
      if (current && current.expires_at <= issuedAt) { this.ctx.storage.sql.exec("DELETE FROM room"); current = undefined; }
      if (current && current.room !== scope.room) return failure(409, "one_room_per_account");
      if (!current && scope.role !== "host") return failure(409, "host_room_required");
      if (!current && this.retiredCount() >= LIMITS.dailyControls * 2) return failure(429, "retirement_capacity_exhausted");
      const pending = this.ctx.storage.sql.exec<{ n: number }>("SELECT COUNT(*) AS n FROM tickets").toArray()[0];
      if ((pending?.n ?? 32) >= 32) return failure(429, "ticket_limit");
      const expiresAt = Math.min(issuedAt + LIMITS.ticketMs, Number(scope.expiresAt));
      if (!Number.isFinite(expiresAt) || expiresAt <= issuedAt) return failure(400, "invalid_expiry");
      this.ctx.storage.transactionSync(() => {
        if (!current) this.ctx.storage.sql.exec("INSERT INTO room(singleton,room,expires_at) VALUES(1,?,?)", scope.room, issuedAt + LIMITS.shareMs);
        this.ctx.storage.sql.exec("INSERT INTO tickets(hash,scope,expires_at,used) VALUES(?,?,?,0)", scope.hash, JSON.stringify({ accountId: scope.accountId, deviceId: scope.deviceId, sessionId: scope.sessionId, room: scope.room, role: scope.role, expiresAt }), expiresAt);
      });
      return Response.json({ expiresAt });
    }
    if (path === "/redeem") {
      if (typeof data.hash !== "string") return failure(401, "invalid_ticket");
      const ticket = this.ctx.storage.sql.exec<{ scope: string; expires_at: number; used: number }>("SELECT scope,expires_at,used FROM tickets WHERE hash=?", data.hash).toArray()[0];
      if (!ticket || ticket.used !== 0 || ticket.expires_at <= now) return failure(401, "invalid_ticket");
      const scope = JSON.parse(ticket.scope) as TicketScope;
      if (scope.accountId !== data.accountId || scope.room !== data.room || scope.role !== data.role) return failure(401, "invalid_ticket");
      if ((data.expectedSessionId !== undefined && data.expectedSessionId !== scope.sessionId) || (data.expectedDeviceId !== undefined && data.expectedDeviceId !== scope.deviceId)) return failure(403, "device_mismatch");
      const permitted = await authority(this.env, scope.accountId, scope.deviceId, scope.sessionId);
      const redeemedAt = Date.now();
      if (ticket.expires_at <= redeemedAt) return failure(401, "invalid_ticket");
      if (!permitted) return failure(403, "account_or_device_inactive");
      // Redemption and reservation share one transaction, including across object replacement.
      return this.ctx.storage.transactionSync(() => {
        this.ctx.storage.sql.exec("UPDATE tickets SET used=1 WHERE hash=? AND used=0", data.hash as string);
        return this.reserve(scope, permitted, redeemedAt);
      });
    }
    if (path === "/reserve") {
      const scope = data as unknown as TicketScope;
      if (!validId(scope.deviceId) || !validId(scope.sessionId) || !validId(scope.room) || !["host", "controller"].includes(scope.role)) return failure(400, "invalid_scope");
      const permitted = await authority(this.env, scope.accountId, scope.deviceId, scope.sessionId);
      if (!permitted) return failure(403, "account_or_device_inactive");
      return this.ctx.storage.transactionSync(() => this.reserve(scope, permitted, Date.now()));
    }
    return failure(404, "not_found");
  }
  private reserve(scope: TicketScope, allowed: Authority, now: number): Response {
    if (allowed.expiresAt <= now) return failure(403, "account_or_device_inactive");
    const current = this.ctx.storage.sql.exec<{ room: string; expires_at: number }>("SELECT room,expires_at FROM room WHERE singleton=1").toArray()[0];
    if (!current || current.room !== scope.room || current.expires_at <= now) return failure(403, "room_expired");
    const day = Math.floor(now / 86_400_000);
    const row = this.ctx.storage.sql.exec<Window>("SELECT frames,bytes,controls FROM windows WHERE day=?", day).toArray()[0];
    const frames = Math.min(LIMITS.blockFrames, allowed.frames - (row?.frames ?? 0));
    const bytes = Math.min(LIMITS.blockBytes, allowed.bytes - (row?.bytes ?? 0));
    if (frames <= 0 || bytes <= 0) return failure(429, "daily_quota_exhausted");
    this.ctx.storage.sql.exec("INSERT INTO windows(day,frames,bytes,controls) VALUES(?,?,?,0) ON CONFLICT(day) DO UPDATE SET frames=frames+excluded.frames,bytes=bytes+excluded.bytes", day, frames, bytes);
    const lease: Lease = { accountId: scope.accountId, deviceId: scope.deviceId, sessionId: scope.sessionId, room: scope.room, role: scope.role, expiresAt: Math.min(leaseExpiry(now), allowed.expiresAt, current.expires_at), frames, bytes };
    return Response.json(lease);
  }
  private cleanup(now: number): void {
    this.ctx.storage.sql.exec("DELETE FROM retired WHERE expires_at<=?", now);
    this.ctx.storage.sql.exec("DELETE FROM tickets WHERE hash IN (SELECT hash FROM tickets WHERE expires_at<=? ORDER BY expires_at LIMIT 128)", now);
    this.ctx.storage.sql.exec("DELETE FROM windows WHERE day<?", Math.floor(now / 86_400_000) - 2);
    this.ctx.storage.sql.exec("DELETE FROM room WHERE expires_at<=?", now);
  }
  private retiredCount(): number { return this.ctx.storage.sql.exec<{ n: number }>("SELECT COUNT(*) AS n FROM retired").one().n; }
}
