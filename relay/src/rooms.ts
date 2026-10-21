import { authenticate } from "./auth.js";
import { DurableObject } from "cloudflare:workers";
import type { Env, Lease, PeerRole } from "./types.js";
import { LIMITS } from "./types.js";
import { failure, hashToken, jsonObject, onlyKeys, randomToken, validId } from "./http.js";
import type { AccountSession } from "./types.js";

interface Attachment extends Lease {
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
    const room = ticketRoute[1];
    const expiresAt = Date.now() + LIMITS.ticketMs;
    const scope = { accountId: auth.accountId, deviceId: auth.deviceId, sessionId: auth.sessionId, room, role: body.role, expiresAt, hash };
    const result = await env.ACCOUNT_QUOTA.get(env.ACCOUNT_QUOTA.idFromName(auth.accountId)).fetch("https://internal/issue", { method: "POST", body: JSON.stringify(scope) });
    if (!result.ok) return result;
    await result.json();
    await env.DB.prepare("INSERT INTO connection_tickets(hash,account_id,room,role,expires_at) VALUES(?,?,?,?,?)").bind(hash, auth.accountId, room, body.role, expiresAt).run();
    return Response.json({ ticket, expiresAt, room, role: body.role, leaseMaximumSeconds: 900 });
  }
  const stopRoute = /^\/v1\/rooms\/([a-zA-Z0-9_-]{16,128})\/stop$/.exec(path);
  if (stopRoute && request.method === "POST") {
    if (!auth || !auth.deviceId) return failure(401, "registered_device_required");
    const stub = env.ACCOUNT_QUOTA.get(env.ACCOUNT_QUOTA.idFromName(auth.accountId));
    const result = await stub.fetch("https://internal/stop", { method: "POST", body: JSON.stringify({ accountId: auth.accountId, room: stopRoute[1] }) });
    if (!result.ok) return result;
    const stopped = await result.json<{ expiresAt: number }>();
    // Room checks the account again, so another account cannot stop a known routing ID.
    return env.ROOM.get(env.ROOM.idFromName(stopRoute[1] as string)).fetch("https://internal/close", { method: "POST", body: JSON.stringify({ accountId: auth.accountId, expiresAt: stopped.expiresAt }) });
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
  private retirement: { accountId: string; expiresAt: number } | undefined;
  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    ctx.storage.sql.exec("CREATE TABLE IF NOT EXISTS retirement(singleton INTEGER PRIMARY KEY CHECK(singleton=1),account_id TEXT NOT NULL,expires_at INTEGER NOT NULL)");
    ctx.storage.sql.exec("DELETE FROM retirement WHERE expires_at<=?", Date.now());
    this.retirement = ctx.storage.sql.exec<{ accountId: string; expiresAt: number }>("SELECT account_id AS accountId,expires_at AS expiresAt FROM retirement WHERE singleton=1").toArray()[0];
    const retirement = this.retirement;
    if (retirement) ctx.blockConcurrencyWhile(() => ctx.storage.setAlarm(retirement.expiresAt));
  }
  override alarm(): Promise<void> {
    const work = this.serial.then(async () => {
      const retirement = this.ctx.storage.sql.exec<{ expires_at: number }>("SELECT expires_at FROM retirement WHERE singleton=1").toArray()[0];
      if (retirement && retirement.expires_at > Date.now()) { await this.ctx.storage.setAlarm(retirement.expires_at); return; }
      this.ctx.storage.sql.exec("DELETE FROM retirement"); this.retirement = undefined;
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
    if (path === "/close" && request.method === "POST") {
      const body = await request.json<{ accountId?: string; expiresAt?: number }>().catch((): { accountId?: string; expiresAt?: number } => ({}));
      const now = Date.now();
      const expiresAt = body.expiresAt;
      if (!validId(body.accountId) || typeof expiresAt !== "number" || !Number.isSafeInteger(expiresAt) || expiresAt <= 0 || expiresAt > now + LIMITS.shareMs) return failure(400, "invalid_retirement");
      const sockets = this.ctx.getWebSockets();
      if ((this.retirement && this.retirement.expiresAt > now && this.retirement.accountId !== body.accountId) || sockets.some(socket => (socket.deserializeAttachment() as Attachment).accountId !== body.accountId)) return failure(403, "account_mismatch");
      const retirement = this.retirement && this.retirement.expiresAt > now ? this.retirement : { accountId: body.accountId, expiresAt };
      this.ctx.storage.sql.exec("INSERT INTO retirement(singleton,account_id,expires_at) VALUES(1,?,?) ON CONFLICT(singleton) DO UPDATE SET account_id=excluded.account_id,expires_at=excluded.expires_at", retirement.accountId, retirement.expiresAt);
      this.retirement = retirement;
      await this.ctx.storage.setAlarm(retirement.expiresAt);
      for (const socket of sockets) socket.close(1000, "sharing_stopped");
      for (const socket of sockets) await this.flush(socket).catch(() => undefined);
      return Response.json({ stopped: true });
    }
    if (path !== "/connect" || request.headers.get("upgrade") !== "websocket") return failure(404, "not_found");
    let lease: Lease;
    try { lease = JSON.parse(request.headers.get("x-internal-lease") ?? "null") as Lease; } catch { return failure(400, "invalid_lease"); }
    if (!lease || !validId(lease.accountId) || !validId(lease.deviceId) || !validId(lease.room) || !["host", "controller"].includes(lease.role) || lease.expiresAt <= Date.now()) return failure(403, "invalid_lease");
    if (this.retirement && this.retirement.expiresAt > Date.now()) return failure(403, "room_retired");
    if (this.retirement) { this.ctx.storage.sql.exec("DELETE FROM retirement"); this.retirement = undefined; }
    const sockets = this.ctx.getWebSockets();
    for (const socket of sockets) {
      const previous = socket.deserializeAttachment() as Attachment;
      if (previous.expiresAt <= Date.now()) { await this.flush(socket); socket.close(1008, "lease_expired"); continue; }
      if (previous.accountId !== lease.accountId || previous.room !== lease.room) return failure(403, "account_mismatch");
      if (previous.role === lease.role) return failure(409, "peer_already_connected");
    }
    const pair = new WebSocketPair();
    const client = pair[0]; const server = pair[1];
    this.ctx.acceptWebSocket(server, [lease.role]);
    const attachment: Attachment = { ...lease, burstSecond: 0, burstCount: 0, usageStartedAt: Date.now(), measuredFrames: 0, measuredBytes: 0 };
    server.serializeAttachment(attachment);
    return new Response(null, { status: 101, webSocket: client, ...(request.headers.get("x-internal-browser") === "true" ? { headers: { "sec-websocket-protocol": "ejm-relay-v1" } } : {}) });
  }
  override webSocketMessage(socket: WebSocket, message: string | ArrayBuffer): Promise<void> {
    const work = this.serial.then(() => this.forward(socket, message)).catch(() => { socket.close(1011, "quota_unavailable"); });
    this.serial = work;
    return work;
  }
  private async forward(socket: WebSocket, message: string | ArrayBuffer): Promise<void> {
    const now = Date.now();
    if (this.retirement && this.retirement.expiresAt > now) { socket.close(1000, "sharing_stopped"); return; }
    let state = socket.deserializeAttachment() as Attachment;
    if (typeof message === "string" || message.byteLength === 0 || message.byteLength > LIMITS.frameBytes) { socket.close(1009, "binary_frame_limit"); return; }
    if (state.expiresAt <= now) { await this.flush(socket); socket.close(1008, "lease_expired"); return; }
    const second = Math.floor(now / 1000);
    const sockets = this.ctx.getWebSockets();
    const currentBurst = sockets.reduce((count, candidate) => { const value = candidate.deserializeAttachment() as Attachment; return count + (value.burstSecond === second ? value.burstCount : 0); }, 0);
    if (currentBurst >= LIMITS.burstFrames) { socket.close(1008, "burst_limit"); return; }
    const peer = sockets.find(candidate => candidate !== socket && (candidate.deserializeAttachment() as Attachment).role !== state.role);
    if (!peer) { socket.close(1008, "peer_not_connected"); return; }
    const peerState = peer.deserializeAttachment() as Attachment;
    if (peerState.accountId !== state.accountId || peerState.room !== state.room || peerState.expiresAt <= now) { socket.close(1008, "peer_lease_expired"); return; }
    // No transcript queue. Runtime buffers are checked when the platform exposes them.
    const buffered = (peer as WebSocket & { bufferedAmount?: number }).bufferedAmount;
    if (buffered !== undefined && buffered > 64 * 1024) { socket.close(1013, "peer_backpressure"); return; }
    if (state.frames <= 0 || state.bytes < message.byteLength) {
      await this.flush(socket);
      const response = await this.env.ACCOUNT_QUOTA.get(this.env.ACCOUNT_QUOTA.idFromName(state.accountId)).fetch("https://internal/reserve", { method: "POST", body: JSON.stringify(state) });
      if (!response.ok) { await response.text(); socket.close(1008, response.status === 429 ? "daily_quota_exhausted" : "authority_expired"); return; }
      const next = await response.json<Lease>();
      state = { ...state, ...next, usageStartedAt: now, measuredFrames: 0, measuredBytes: 0 };
    }
    const sendingAt = Date.now();
    if (this.retirement && this.retirement.expiresAt > sendingAt) { socket.close(1000, "sharing_stopped"); return; }
    if (state.expiresAt <= sendingAt) { socket.close(1008, "lease_expired"); return; }
    const currentPeer = peer.deserializeAttachment() as Attachment;
    if (currentPeer.accountId !== state.accountId || currentPeer.room !== state.room || currentPeer.role !== (state.role === "host" ? "controller" : "host") || currentPeer.expiresAt <= sendingAt) { socket.close(1008, "peer_lease_expired"); return; }
    state.frames -= 1; state.bytes -= message.byteLength;
    state.burstCount = state.burstSecond === second ? state.burstCount + 1 : 1; state.burstSecond = second;
    state.measuredFrames += 1; state.measuredBytes += message.byteLength;
    socket.serializeAttachment(state);
    try { peer.send(message); } catch { socket.close(1013, "peer_unavailable"); }
  }
  override webSocketClose(socket: WebSocket): Promise<void> {
    const work = this.serial.then(async () => {
      // Each TCP connection carries one fresh Noise handshake. Close both ends
      // so reconnect cannot deliver handshake bytes into an old peer cipher.
      const peers = new Set([socket, ...this.ctx.getWebSockets()]);
      for (const peer of peers) peer.close(1000, "connection_closed");
      // Optional usage collection must never delay transport revocation.
      for (const peer of peers) await this.flush(peer).catch(() => undefined);
    }).catch(() => { socket.close(1011, "connection_unavailable"); });
    this.serial = work;
    return work;
  }
  override webSocketError(socket: WebSocket): Promise<void> { return this.webSocketClose(socket); }
  private async flush(socket: WebSocket): Promise<void> {
    const state = socket.deserializeAttachment() as Attachment;
    if (state.measuredFrames === 0) return;
    const request = { accountId: state.accountId, bytes: state.measuredBytes, frames: state.measuredFrames, connectedMs: Math.max(0, Math.min(Date.now(), state.expiresAt) - state.usageStartedAt), quality: "measured", measuredAt: state.usageStartedAt };
    state.measuredFrames = 0; state.measuredBytes = 0; state.usageStartedAt = Date.now();
    socket.serializeAttachment(state);
    // The privacy authority discards these operational counters unless a current scoped choice permits collection.
    const response = await this.env.PRIVACY_AUTHORITY.get(this.env.PRIVACY_AUTHORITY.idFromName("privacy-v1")).fetch("https://internal/internal/usage", { method: "POST", body: JSON.stringify(request) });
    await response.text();
  }
}
