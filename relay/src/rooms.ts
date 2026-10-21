import { authenticate } from "./auth.js";
import { DurableObject } from "cloudflare:workers";
import type { Env, Lease, PeerRole } from "./types.js";
import { LIMITS } from "./types.js";
import { failure, hashToken, jsonObject, onlyKeys, randomToken, validId } from "./http.js";
import type { AccountSession } from "./types.js";

interface Attachment extends Lease {
  generation: string;
  paired: boolean;
  burstSecond: number;
  burstCount: number;
  usageStartedAt: number;
  measuredFrames: number;
  measuredBytes: number;
}
export async function roomRoute(request: Request, env: Env, auth: AccountSession | null): Promise<Response | null> {
  const path = new URL(request.url).pathname;
  if (path === "/v1/rooms" && request.method === "GET") {
    if (!auth) return failure(401, "sign_in_required");
    return env.ACCOUNT_QUOTA.get(env.ACCOUNT_QUOTA.idFromName(auth.accountId)).fetch("https://internal/rooms", { method: "POST", body: JSON.stringify({ accountId: auth.accountId }) });
  }
  const ticketRoute = /^\/v1\/rooms\/([a-zA-Z0-9_-]{16,128})\/tickets$/.exec(path);
  if (ticketRoute && request.method === "POST") {
    if (!auth || !auth.deviceId || auth.termsVersion !== env.TERMS_VERSION) return failure(403, "registered_device_required");
    const body = await jsonObject(request);
    if (!body || !["host", "controller"].includes(String(body.role)) || body.deviceId !== auth.deviceId || !onlyKeys(body, ["role", "deviceId"])) return failure(400, "invalid_ticket_scope");
    const ticket = randomToken();
    const hash = await hashToken(ticket);
    const room = ticketRoute[1]!;
    let expiresAt = Date.now() + LIMITS.ticketMs;
    const scope = { accountId: auth.accountId, deviceId: auth.deviceId, sessionId: auth.sessionId, room, role: body.role, expiresAt, hash };
    const result = await env.ACCOUNT_QUOTA.get(env.ACCOUNT_QUOTA.idFromName(auth.accountId)).fetch("https://internal/issue", { method: "POST", body: JSON.stringify(scope) });
    if (!result.ok) return result;
    const issued = await result.json<{ expiresAt: number; shareExpiresAt: number; created: boolean }>();
    const claim = await env.ROOM.get(env.ROOM.idFromName(room)).fetch("https://internal/claim", { method: "POST", body: JSON.stringify({ accountId: auth.accountId, expiresAt: issued.shareExpiresAt, created: issued.created && body.role === "host" }) });
    if (!claim.ok) {
      if (issued.created && [403, 409].includes(claim.status)) {
        const discarded = await env.ACCOUNT_QUOTA.get(env.ACCOUNT_QUOTA.idFromName(auth.accountId)).fetch("https://internal/discard-issued", { method: "POST", body: JSON.stringify({ accountId: auth.accountId, room, hash, expiresAt: issued.shareExpiresAt }) });
        await discarded.text();
        if (!discarded.ok) { await claim.text(); return failure(503, "quota_authority_unavailable"); }
      }
      return claim;
    }
    await claim.text();
    expiresAt = Math.min(expiresAt, issued.expiresAt);
    if (expiresAt <= Date.now()) return failure(401, "invalid_ticket");
    await env.DB.prepare("INSERT INTO connection_tickets(hash,account_id,room,role,expires_at) VALUES(?,?,?,?,?)").bind(hash, auth.accountId, room, body.role, expiresAt).run();
    return Response.json({ ticket, expiresAt, room, role: body.role, leaseMaximumSeconds: 900 });
  }
  const stopRoute = /^\/v1\/rooms\/([a-zA-Z0-9_-]{16,128})\/stop$/.exec(path);
  if (stopRoute && request.method === "POST") {
    if (!auth) return failure(401, "sign_in_required");
    const stub = env.ACCOUNT_QUOTA.get(env.ACCOUNT_QUOTA.idFromName(auth.accountId));
    const result = await stub.fetch("https://internal/stop", { method: "POST", body: JSON.stringify({ accountId: auth.accountId, room: stopRoute[1] }) });
    if (!result.ok) return result;
    const stopped = await result.json<{ expiresAt: number }>();
    return env.ROOM.get(env.ROOM.idFromName(stopRoute[1] as string)).fetch("https://internal/close", { method: "POST", body: JSON.stringify({ accountId: auth.accountId, room: stopRoute[1], expiresAt: stopped.expiresAt }) });
  }
  const socketRoute = /^\/v1\/rooms\/([a-zA-Z0-9_-]{16,128})\/(host|controller)$/.exec(path);
  if (!socketRoute || request.method !== "GET") return null;
  if (request.headers.get("upgrade")?.toLowerCase() !== "websocket") return failure(426, "websocket_required");
  if (new URL(request.url).search) return failure(400, "url_credentials_not_allowed");
  const requestOrigin = request.headers.get("origin");
  if (requestOrigin !== null && requestOrigin !== env.RELAY_ORIGIN) return failure(403, "origin_not_allowed");
  const authorization = request.headers.get("authorization");
  const protocols = request.headers.get("sec-websocket-protocol");
  let ticket: string;
  let browser = false;
  let browserAccount: AccountSession | null = null;
  if (protocols !== null) {
    if (authorization !== null || protocols.length > 256 || requestOrigin !== env.RELAY_ORIGIN) return failure(403, "browser_origin_required");
    const offered = protocols.split(",").map(value => value.trim());
    const secret = offered.find(value => /^ticket\.[a-zA-Z0-9_-]{43}$/.test(value));
    if (offered.length !== 2 || !offered.includes("ejm-relay-v1") || !secret) return failure(401, "ticket_required");
    browserAccount = auth ?? await authenticate(request, env);
    if (!browserAccount?.deviceId) return failure(401, "registered_browser_required");
    ticket = secret.slice(7); browser = true;
  } else {
    if (!authorization || !/^Bearer [a-zA-Z0-9_-]{43}$/.test(authorization)) return failure(401, "ticket_required");
    ticket = authorization.slice(7);
  }
  const hash = await hashToken(ticket);
  const row = await env.DB.prepare("SELECT account_id,room,role,expires_at FROM connection_tickets WHERE hash=?").bind(hash).first<{ account_id: string; room: string; role: PeerRole; expires_at: number }>();
  if (!row || row.expires_at <= Date.now() || row.room !== socketRoute[1] || row.role !== socketRoute[2]) return failure(401, "invalid_ticket");
  if (browserAccount && browserAccount.accountId !== row.account_id) return failure(403, "account_mismatch");
  const quota = env.ACCOUNT_QUOTA.get(env.ACCOUNT_QUOTA.idFromName(row.account_id));
  const redeemed = await quota.fetch("https://internal/redeem", { method: "POST", body: JSON.stringify({ accountId: row.account_id, hash, room: row.room, role: row.role, expectedSessionId: browserAccount?.sessionId, expectedDeviceId: browserAccount?.deviceId }) });
  if (!redeemed.ok) return redeemed;
  const lease = await redeemed.json<Lease>();
  return env.ROOM.get(env.ROOM.idFromName(row.room)).fetch("https://internal/connect", { method: "GET", headers: { upgrade: "websocket", "x-internal-lease": JSON.stringify(lease), ...(browser ? { "x-internal-browser": "true" } : {}) } });
}
export class Room extends DurableObject<Env> {
  private serial: Promise<unknown> = Promise.resolve();
  private owner: { accountId: string; expiresAt: number } | undefined;
  private retirement: { accountId: string; expiresAt: number } | undefined;
  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    ctx.storage.sql.exec("CREATE TABLE IF NOT EXISTS owner(singleton INTEGER PRIMARY KEY CHECK(singleton=1),account_id TEXT NOT NULL,expires_at INTEGER NOT NULL)");
    ctx.storage.sql.exec("CREATE TABLE IF NOT EXISTS retirement(singleton INTEGER PRIMARY KEY CHECK(singleton=1),account_id TEXT NOT NULL,expires_at INTEGER NOT NULL)");
    ctx.storage.sql.exec("DELETE FROM owner WHERE expires_at<=?", Date.now());
    ctx.storage.sql.exec("DELETE FROM retirement WHERE expires_at<=?", Date.now());
    this.owner = ctx.storage.sql.exec<{ accountId: string; expiresAt: number }>("SELECT account_id AS accountId,expires_at AS expiresAt FROM owner WHERE singleton=1").toArray()[0];
    this.retirement = ctx.storage.sql.exec<{ accountId: string; expiresAt: number }>("SELECT account_id AS accountId,expires_at AS expiresAt FROM retirement WHERE singleton=1").toArray()[0];
    // Legacy attachments cannot establish the original ownership of a share.
    if (!this.owner || this.retirement) for (const socket of ctx.getWebSockets()) socket.close(1008, "transport_replaced");
    if (this.owner || this.retirement) ctx.blockConcurrencyWhile(() => this.scheduleExpiry());
  }
  override alarm(): Promise<void> {
    const work = this.serial.then(async () => {
      this.expireControl(Date.now());
      await this.scheduleExpiry();
    });
    this.serial = work.catch(() => undefined);
    return work;
  }
  override fetch(request: Request): Promise<Response> {
    const work = this.serial.then(() => this.handle(request));
    this.serial = work.catch(() => undefined);
    return work;
  }
  private async handle(request: Request): Promise<Response> {
    const path = new URL(request.url).pathname;
    this.expireControl(Date.now());
    if (path === "/claim" && request.method === "POST") {
      const body = await request.json<{ accountId?: string; expiresAt?: number; created?: boolean }>();
      const now = Date.now(); const expiresAt = body.expiresAt;
      if (!validId(body.accountId) || typeof expiresAt !== "number" || !Number.isSafeInteger(expiresAt) || expiresAt <= now || expiresAt > now + LIMITS.shareMs) return failure(400, "invalid_owner");
      if (this.retirement) return failure(403, "room_retired");
      if (this.owner && this.owner.accountId !== body.accountId) return failure(403, "account_mismatch");
      if (this.owner && this.owner.expiresAt !== expiresAt) return failure(409, "share_expiry_mismatch");
      if (!this.owner) {
        if (body.created !== true) return failure(409, "fresh_share_required");
        this.ctx.storage.sql.exec("INSERT INTO owner(singleton,account_id,expires_at) VALUES(1,?,?)", body.accountId, expiresAt);
        this.owner = { accountId: body.accountId, expiresAt };
      }
      await this.scheduleExpiry();
      return Response.json({ expiresAt: this.owner.expiresAt });
    }
    if (path === "/close" && request.method === "POST") {
      const body = await request.json<{ accountId?: string; room?: string; expiresAt?: number }>().catch((): { accountId?: string; room?: string; expiresAt?: number } => ({}));
      const now = Date.now();
      const expiresAt = body.expiresAt;
      if (!validId(body.accountId) || !validId(body.room) || typeof expiresAt !== "number" || !Number.isSafeInteger(expiresAt) || expiresAt <= 0 || expiresAt > now + LIMITS.shareMs) return failure(400, "invalid_retirement");
      const sockets = this.ctx.getWebSockets();
      if ((this.owner && (this.owner.accountId !== body.accountId || this.owner.expiresAt !== expiresAt)) || (this.retirement && (this.retirement.accountId !== body.accountId || this.retirement.expiresAt !== expiresAt))) return failure(403, "account_mismatch");
      // An unowned legacy Room is already inert; do not create a caller-owned fence.
      if (!this.owner && !this.retirement) { for (const socket of sockets) socket.close(1000, "sharing_stopped"); return this.acknowledgeClosure(body.accountId, body.room, expiresAt); }
      const retirement = this.retirement && this.retirement.expiresAt > now ? this.retirement : { accountId: body.accountId, expiresAt };
      this.ctx.storage.sql.exec("INSERT INTO retirement(singleton,account_id,expires_at) VALUES(1,?,?) ON CONFLICT(singleton) DO UPDATE SET account_id=excluded.account_id,expires_at=excluded.expires_at", retirement.accountId, retirement.expiresAt);
      this.retirement = retirement;
      await this.scheduleExpiry();
      for (const socket of sockets) socket.close(1000, "sharing_stopped");
      const acknowledged = await this.acknowledgeClosure(body.accountId, body.room, retirement.expiresAt);
      for (const socket of sockets) await this.flush(socket).catch(() => undefined);
      return acknowledged;
    }
    if (path !== "/connect" || request.headers.get("upgrade") !== "websocket") return failure(404, "not_found");
    let lease: Lease;
    try { lease = JSON.parse(request.headers.get("x-internal-lease") ?? "null") as Lease; } catch { return failure(400, "invalid_lease"); }
    if (!lease || !validId(lease.accountId) || !validId(lease.deviceId) || !validId(lease.room) || !["host", "controller"].includes(lease.role) || lease.expiresAt <= Date.now()) return failure(403, "invalid_lease");
    if (!this.owner || this.owner.accountId !== lease.accountId || lease.expiresAt > this.owner.expiresAt) return failure(403, "room_not_owned");
    if (this.retirement && this.retirement.expiresAt > Date.now()) return failure(403, "room_retired");
    if (this.retirement) { this.ctx.storage.sql.exec("DELETE FROM retirement"); this.retirement = undefined; }
    const sockets = this.ctx.getWebSockets();
    for (const socket of sockets) {
      const previous = socket.deserializeAttachment() as Lease | null;
      if (socket.readyState === WebSocket.OPEN && previous && previous.expiresAt > Date.now() && (previous.accountId !== lease.accountId || previous.room !== lease.room)) return failure(403, "account_mismatch");
    }
    const retired = new Set<WebSocket>();
    for (const socket of sockets) {
      const previous = this.attachment(socket);
      if (!previous || socket.readyState !== WebSocket.OPEN || previous.expiresAt <= Date.now()) {
        for (const peer of this.closeGeneration(socket, 1008, previous ? "lease_expired" : "transport_replaced")) retired.add(peer);
      }
    }
    let peer = this.ctx.getWebSockets().find(socket => socket.readyState === WebSocket.OPEN);
    // A surviving endpoint has an old cipher even after its partner disappears.
    if (peer && this.attachment(peer)?.paired && this.ctx.getWebSockets().filter(socket => socket.readyState === WebSocket.OPEN).length === 1) {
      for (const socket of this.closeGeneration(peer, 1000, "connection_closed")) retired.add(socket);
      peer = undefined;
    }
    // Close the complete old generation before optional collection can wait.
    for (const socket of retired) await this.flush(socket).catch(() => undefined);
    if (peer && (peer.readyState !== WebSocket.OPEN || this.attachment(peer)!.expiresAt <= Date.now())) {
      for (const socket of this.closeGeneration(peer, 1008, "lease_expired")) await this.flush(socket).catch(() => undefined);
      peer = undefined;
    }
    const live = this.ctx.getWebSockets().filter(socket => socket.readyState === WebSocket.OPEN);
    for (const socket of live) {
      const previous = this.attachment(socket)!;
      if (previous.accountId !== lease.accountId || previous.room !== lease.room) return failure(403, "account_mismatch");
      if (previous.role === lease.role) return failure(409, "peer_already_connected");
    }
    if (lease.expiresAt <= Date.now()) return failure(403, "invalid_lease");
    const pair = new WebSocketPair();
    const client = pair[0]; const server = pair[1];
    this.ctx.acceptWebSocket(server, [lease.role]);
    const previous = peer ? this.attachment(peer)! : undefined;
    const attachment: Attachment = { ...lease, generation: previous?.generation ?? crypto.randomUUID(), paired: !!peer, burstSecond: 0, burstCount: 0, usageStartedAt: Date.now(), measuredFrames: 0, measuredBytes: 0 };
    if (peer && previous) peer.serializeAttachment({ ...previous, paired: true });
    server.serializeAttachment(attachment);
    return new Response(null, { status: 101, webSocket: client, ...(request.headers.get("x-internal-browser") === "true" ? { headers: { "sec-websocket-protocol": "ejm-relay-v1" } } : {}) });
  }
  override webSocketMessage(socket: WebSocket, message: string | ArrayBuffer): Promise<void> {
    const work = this.serial.then(() => this.forward(socket, message)).catch(() => { this.closeGeneration(socket, 1011, "quota_unavailable"); });
    this.serial = work;
    return work;
  }
  private async forward(socket: WebSocket, message: string | ArrayBuffer): Promise<void> {
    const now = Date.now();
    if (this.retirement && this.retirement.expiresAt > now) { this.closeGeneration(socket, 1000, "sharing_stopped"); return; }
    let state = this.attachment(socket);
    if (!this.owner || this.owner.expiresAt <= now || this.owner.accountId !== state?.accountId) { this.closeGeneration(socket, 1008, "room_not_owned"); return; }
    if (!state || !state.paired || socket.readyState !== WebSocket.OPEN) { this.closeGeneration(socket, 1008, "peer_not_connected"); return; }
    if (typeof message === "string" || message.byteLength === 0 || message.byteLength > LIMITS.frameBytes) { this.closeGeneration(socket, 1009, "binary_frame_limit"); return; }
    if (state.expiresAt <= now) {
      const retired = this.closeGeneration(socket, 1008, "lease_expired");
      for (const peer of retired) await this.flush(peer).catch(() => undefined);
      return;
    }
    const second = Math.floor(now / 1000);
    const sockets = this.ctx.getWebSockets().filter(candidate => candidate.readyState === WebSocket.OPEN && this.attachment(candidate)?.generation === state!.generation);
    const currentBurst = sockets.reduce((count, candidate) => { const value = this.attachment(candidate)!; return count + (value.burstSecond === second ? value.burstCount : 0); }, 0);
    if (currentBurst >= LIMITS.burstFrames) { this.closeGeneration(socket, 1008, "burst_limit"); return; }
    const peer = sockets.find(candidate => candidate !== socket && this.attachment(candidate)!.role !== state!.role);
    if (!peer) { this.closeGeneration(socket, 1008, "peer_not_connected"); return; }
    const peerState = this.attachment(peer)!;
    if (!peerState.paired || peerState.accountId !== state.accountId || peerState.room !== state.room || peerState.expiresAt <= now) { this.closeGeneration(socket, 1008, "peer_lease_expired"); return; }
    // No transcript queue. Runtime buffers are checked when the platform exposes them.
    const buffered = (peer as WebSocket & { bufferedAmount?: number }).bufferedAmount;
    if (buffered !== undefined && buffered > 64 * 1024) { this.closeGeneration(socket, 1013, "peer_backpressure"); return; }
    if (state.frames <= 0 || state.bytes < message.byteLength) {
      await this.flush(socket);
      const response = await this.env.ACCOUNT_QUOTA.get(this.env.ACCOUNT_QUOTA.idFromName(state.accountId)).fetch("https://internal/reserve", { method: "POST", body: JSON.stringify(state) });
      if (!response.ok) { await response.text(); this.closeGeneration(socket, 1008, response.status === 429 ? "daily_quota_exhausted" : "authority_expired"); return; }
      const next = await response.json<Lease>();
      state = { ...state, ...next, usageStartedAt: now, measuredFrames: 0, measuredBytes: 0 };
    }
    const sendingAt = Date.now();
    if (!this.owner || this.owner.expiresAt <= sendingAt || this.owner.accountId !== state.accountId) { this.closeGeneration(socket, 1008, "room_not_owned"); return; }
    if (this.retirement && this.retirement.expiresAt > sendingAt) { this.closeGeneration(socket, 1000, "sharing_stopped"); return; }
    if (state.expiresAt <= sendingAt) { this.closeGeneration(socket, 1008, "lease_expired"); return; }
    const currentPeer = this.attachment(peer);
    if (socket.readyState !== WebSocket.OPEN || this.attachment(socket)?.generation !== state.generation || !currentPeer?.paired || peer.readyState !== WebSocket.OPEN || currentPeer.generation !== state.generation || currentPeer.accountId !== state.accountId || currentPeer.room !== state.room || currentPeer.role !== (state.role === "host" ? "controller" : "host") || currentPeer.expiresAt <= sendingAt) { this.closeGeneration(socket, 1008, "peer_lease_expired"); return; }
    if (state.frames <= 0 || state.bytes < message.byteLength) { this.closeGeneration(socket, 1008, "daily_quota_exhausted"); return; }
    state.frames -= 1; state.bytes -= message.byteLength;
    state.burstCount = state.burstSecond === second ? state.burstCount + 1 : 1; state.burstSecond = second;
    state.measuredFrames += 1; state.measuredBytes += message.byteLength;
    socket.serializeAttachment(state);
    try { peer.send(message); } catch { this.closeGeneration(socket, 1013, "peer_unavailable"); }
  }
  override webSocketClose(socket: WebSocket): Promise<void> {
    const generation = this.attachment(socket)?.generation;
    const work = this.serial.then(async () => {
      const peers = this.closeGeneration(socket, 1000, "connection_closed", generation);
      // Optional usage collection must never delay transport revocation.
      for (const peer of peers) await this.flush(peer).catch(() => undefined);
    }).catch(() => { socket.close(1011, "connection_unavailable"); });
    this.serial = work;
    return work;
  }
  override webSocketError(socket: WebSocket): Promise<void> { return this.webSocketClose(socket); }
  private attachment(socket: WebSocket): Attachment | null {
    const state = socket.deserializeAttachment() as Attachment | null;
    return state && typeof state.generation === "string" && typeof state.paired === "boolean" ? state : null;
  }
  private expireControl(now: number): void {
    if (this.owner && this.owner.expiresAt <= now) { this.ctx.storage.sql.exec("DELETE FROM owner"); this.owner = undefined; }
    if (this.retirement && this.retirement.expiresAt <= now) { this.ctx.storage.sql.exec("DELETE FROM retirement"); this.retirement = undefined; }
  }
  private async scheduleExpiry(): Promise<void> {
    const expiry = Math.min(this.owner?.expiresAt ?? Infinity, this.retirement?.expiresAt ?? Infinity);
    if (Number.isFinite(expiry)) await this.ctx.storage.setAlarm(expiry);
  }
  private async acknowledgeClosure(accountId: string, room: string, expiresAt: number): Promise<Response> {
    const response = await this.env.ACCOUNT_QUOTA.get(this.env.ACCOUNT_QUOTA.idFromName(accountId)).fetch("https://internal/ack-close", { method: "POST", body: JSON.stringify({ accountId, room, expiresAt }) });
    if (!response.ok) { await response.text(); return failure(503, "room_revocation_pending"); }
    const result = await response.json<{ pending: boolean }>();
    return Response.json({ stopped: true, pending: result.pending });
  }
  private closeGeneration(socket: WebSocket, code: number, reason: string, generation = this.attachment(socket)?.generation): WebSocket[] {
    const peers = [socket, ...this.ctx.getWebSockets().filter(peer => peer !== socket && generation !== undefined && this.attachment(peer)?.generation === generation)];
    socket.close(code, reason);
    for (const peer of peers.slice(1)) peer.close(1000, "connection_closed");
    return peers;
  }
  private async flush(socket: WebSocket): Promise<void> {
    const state = socket.deserializeAttachment() as Attachment | null;
    if (!state || state.measuredFrames === 0) return;
    const request = { accountId: state.accountId, bytes: state.measuredBytes, frames: state.measuredFrames, connectedMs: Math.max(0, Math.min(Date.now(), state.expiresAt) - state.usageStartedAt), quality: "measured", measuredAt: state.usageStartedAt };
    state.measuredFrames = 0; state.measuredBytes = 0; state.usageStartedAt = Date.now();
    socket.serializeAttachment(state);
    // The privacy authority discards these operational counters unless a current scoped choice permits collection.
    const response = await this.env.PRIVACY_AUTHORITY.get(this.env.PRIVACY_AUTHORITY.idFromName("privacy-v1")).fetch("https://internal/internal/usage", { method: "POST", body: JSON.stringify(request) });
    await response.text();
  }
}
