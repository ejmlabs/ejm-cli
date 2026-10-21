// Product framing only. Snow in WebAssembly owns every cryptographic operation.
export const LIMITS = Object.freeze({ frame: 16368, header: 22, chunk: 16346, message: 1048576, prompt: 16384, events: 512, output: 262144, blocks: 1024 });
const encoder = new TextEncoder();
const decoder = new TextDecoder("utf-8", { fatal: true, ignoreBOM: false });
export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
export interface Wire { version: 1; body: { direction: "client" | "server"; message: Record<string, unknown> } }
export interface RemoteEvent { sequence: number; session_id: string; run_id: string | null; type: string; [key: string]: unknown }
export interface PendingAction { sequence: number; type: ActionType; text: string; state: "pending" | "unknown"; }
export type ActionType = "submit" | "steer" | "queue" | "cancel";
export type Outcome = { type: "pending" | "unknown" | "queued" } | { type: "accepted"; run_id: string | null } | { type: "rejected"; code: string; message: string };
export type ServerMessage =
  | { type: "snapshot"; session: { id: string; title: string; cwd: string; status: string }; events: RemoteEvent[]; through_sequence: number; mutation_high_water: number; active_run: string | null; role: "view" | "control"; gap: unknown }
  | { type: "events"; events: RemoteEvent[]; through_sequence: number; gap: unknown }
  | { type: "outcome"; sequence: number; outcome: Outcome }
  | { type: "stopped"; reason: string }
  | { type: "error"; code: string; message: string };

function requireValue(condition: unknown, message = "Invalid remote message"): asserts condition {
  if (!condition) throw new Error(message);
}
function object(value: unknown): Record<string, unknown> {
  requireValue(value !== null && typeof value === "object" && !Array.isArray(value));
  return value as Record<string, unknown>;
}
function integer(value: unknown): asserts value is number { requireValue(Number.isSafeInteger(value) && Number(value) >= 0); }
function id(value: unknown): asserts value is string { requireValue(typeof value === "string" && value.length > 0 && encoder.encode(value).length <= 256); }
function optionalId(value: unknown): asserts value is string | null { if (value !== null) id(value); }
function text(value: unknown, limit: number): asserts value is string { requireValue(typeof value === "string" && encoder.encode(value).length <= limit); }

export function validateServer(value: unknown): ServerMessage {
  const wire = object(value);
  const body = object(wire.body);
  requireValue(wire.version === 1 && body.direction === "server");
  const message = object(body.message);
  switch (message.type) {
    case "snapshot": {
      const session = object(message.session);
      id(session.id); text(session.title, LIMITS.message); text(session.cwd, LIMITS.message);
      requireValue(["idle", "running", "interrupted"].includes(String(session.status)));
      optionalId(message.active_run);
      integer(message.mutation_high_water);
      requireValue(message.role === "view" || message.role === "control");
      validateEvents(message);
      for (const event of message.events as RemoteEvent[]) requireValue(event.session_id === session.id);
      break;
    }
    case "events": validateEvents(message); break;
    case "outcome": {
      integer(message.sequence); requireValue(message.sequence > 0);
      const outcome = object(message.outcome);
      requireValue(["accepted", "queued", "rejected", "unknown", "pending"].includes(String(outcome.type)));
      if (outcome.type === "accepted") optionalId(outcome.run_id);
      if (outcome.type === "rejected") { id(outcome.code); text(outcome.message, 4096); }
      break;
    }
    case "stopped": requireValue(["local_stop", "host_exit", "session_changed", "expired", "transport_closed"].includes(String(message.reason))); break;
    case "error": id(message.code); text(message.message, 4096); break;
    default: throw new Error("Unsupported remote message");
  }
  return message as ServerMessage;
}

function validateEvents(message: Record<string, unknown>): void {
  integer(message.through_sequence);
  requireValue(Array.isArray(message.events) && message.events.length <= LIMITS.events);
  let previous = -1;
  for (const item of message.events) {
    const event = object(item);
    integer(event.sequence); id(event.session_id); optionalId(event.run_id); id(event.type);
    requireValue(event.sequence > previous && event.sequence <= message.through_sequence);
    previous = event.sequence;
  }
  if (message.gap !== null) {
    const gap = object(message.gap);
    integer(gap.requested_after_sequence); integer(gap.available_after_sequence);
    requireValue(["retention", "output_truncated", "slow_consumer"].includes(String(gap.reason)));
  }
}

export function client(message: Record<string, unknown>): Wire { return { version: 1, body: { direction: "client", message } }; }

export class FrameEncoder {
  private next = 1n;
  encode(wire: Wire): Uint8Array[] {
    const bytes = encoder.encode(JSON.stringify(wire));
    requireValue(bytes.length > 0 && bytes.length <= LIMITS.message, "Remote message exceeds the limit");
    const count = Math.ceil(bytes.length / LIMITS.chunk);
    const result: Uint8Array[] = [];
    const sequence = this.next++;
    requireValue(sequence < 0xffffffffffffffffn, "Frame sequence exhausted; reconnect");
    for (let index = 0; index < count; index++) {
      const chunk = bytes.subarray(index * LIMITS.chunk, (index + 1) * LIMITS.chunk);
      const frame = new Uint8Array(LIMITS.header + chunk.length);
      const header = new DataView(frame.buffer);
      frame.set([69, 74, 77, 82]);
      header.setUint16(4, 1); header.setBigUint64(6, sequence); header.setUint32(14, bytes.length);
      header.setUint16(18, index); header.setUint16(20, count); frame.set(chunk, LIMITS.header);
      result.push(frame);
    }
    bytes.fill(0);
    return result;
  }
}

export class FrameDecoder {
  private next = 1n;
  private failed = false;
  private partial: { id: bigint; total: number; count: number; next: number; bytes: Uint8Array; offset: number } | null = null;
  clear(): void { this.partial?.bytes.fill(0); this.partial = null; this.failed = true; }
  push(frame: Uint8Array): ServerMessage | null {
    try { return this.read(frame); }
    catch (error) { this.clear(); throw error; }
  }
  private read(frame: Uint8Array): ServerMessage | null {
    requireValue(!this.failed && frame.length > LIMITS.header && frame.length <= LIMITS.frame, "Invalid or closed remote stream");
    const header = new DataView(frame.buffer, frame.byteOffset, frame.byteLength);
    requireValue(header.getUint32(0) === 0x454a4d52 && header.getUint16(4) === 1);
    const sequence = header.getBigUint64(6), total = header.getUint32(14), index = header.getUint16(18), count = header.getUint16(20);
    requireValue(sequence === this.next && total > 0 && total <= LIMITS.message && count === Math.ceil(total / LIMITS.chunk));
    if (!this.partial) {
      requireValue(index === 0);
      this.partial = { id: sequence, total, count, next: 0, bytes: new Uint8Array(total), offset: 0 };
    }
    const partial = this.partial;
    requireValue(sequence === partial.id && total === partial.total && count === partial.count && index === partial.next && index < count);
    const chunk = frame.subarray(LIMITS.header);
    const remaining = total - partial.offset;
    requireValue(chunk.length === Math.min(remaining, LIMITS.chunk));
    partial.bytes.set(chunk, partial.offset); partial.offset += chunk.length; partial.next++;
    if (partial.next !== count) return null;
    requireValue(partial.offset === total);
    const message: unknown = JSON.parse(decoder.decode(partial.bytes));
    partial.bytes.fill(0); this.partial = null; this.next++;
    return validateServer(message);
  }
}

// Share-level mutation state survives fresh transport handshakes in the same tab.
export class SessionState {
  sessionId: string | null = null;
  activeRun: string | null = null;
  cursor = 0;
  role: "view" | "control" = "view";
  ready = false;
  stopped = false;
  pending: PendingAction | null = null;
  private sequence = 0;

  snapshotRequest(): Wire { return client({ type: "snapshot", after_sequence: this.cursor, limit: LIMITS.events }); }
  outcomeRequest(): Wire | null { return this.pending ? client({ type: "outcome", sequence: this.pending.sequence }) : null; }
  disconnected(): void { this.ready = false; if (this.pending) this.pending.state = "unknown"; }
  mutation(type: ActionType, prompt: string): Wire {
    requireValue(this.ready && !this.stopped && this.role === "control" && this.sessionId && !this.pending, "This session cannot accept an action now");
    requireValue(this.sequence < Number.MAX_SAFE_INTEGER, "Action sequence exhausted");
    if (type !== "cancel") requireValue(prompt.trim().length > 0 && encoder.encode(prompt).length <= LIMITS.prompt, "Write a prompt of at most 16 KiB");
    if (type === "submit") requireValue(this.activeRun === null, "A run is already active");
    else requireValue(this.activeRun !== null, "This action needs an active run");
    const action = type === "cancel" ? { type, run_id: this.activeRun }
      : type === "submit" ? { type, text: prompt } : { type, run_id: this.activeRun, text: prompt };
    const sequence = ++this.sequence;
    this.pending = { sequence, type, text: type === "cancel" ? "" : prompt, state: "pending" };
    return client({ type: "mutation", mutation: { sequence, expected: { session_id: this.sessionId, run_id: this.activeRun, event_sequence: this.cursor }, action } });
  }
  apply(message: ServerMessage): { events: RemoteEvent[]; resolved: Outcome | null; gap: boolean } {
    const result = { events: [] as RemoteEvent[], resolved: null as Outcome | null, gap: false };
    switch (message.type) {
      case "snapshot":
        requireValue(!this.stopped && (this.sessionId === null || this.sessionId === message.session.id), "Host changed the shared session");
        this.sessionId = message.session.id; this.activeRun = message.active_run; this.role = message.role; this.ready = true;
        this.sequence = Math.max(this.sequence, message.mutation_high_water);
        // Snapshot state is authoritative even when its retained events describe older runs.
        this.applyEvents(message, result, false);
        break;
      case "events": requireValue(this.ready, "Session snapshot is missing"); this.applyEvents(message, result, true); break;
      case "outcome":
        if (this.pending?.sequence === message.sequence) {
          if (message.outcome.type === "pending" || message.outcome.type === "unknown") this.pending.state = message.outcome.type;
          else { result.resolved = message.outcome; this.pending = null; }
        }
        break;
      case "stopped": this.ready = false; this.stopped = true; if (this.pending) this.pending.state = "unknown"; break;
      case "error": break;
    }
    return result;
  }
  private applyEvents(message: Extract<ServerMessage, { events: RemoteEvent[] }>, result: { events: RemoteEvent[]; gap: boolean }, updateRun: boolean): void {
    requireValue(message.through_sequence >= this.cursor, "Replay cursor moved backwards");
    for (const event of message.events) {
      requireValue(event.session_id === this.sessionId, "Event belongs to another session");
      if (event.sequence <= this.cursor) continue;
      result.events.push(event);
      if (updateRun && event.type === "run_started") this.activeRun = event.run_id;
      if (updateRun && event.type === "run_finished" && event.run_id === this.activeRun) this.activeRun = null;
    }
    this.cursor = message.through_sequence; result.gap = message.gap !== null;
  }
}

export function displayText(value: string): string {
  // Text nodes never execute terminal sequences. Remove nonprinting controls too.
  return value.replace(/[\u0000-\u0008\u000b-\u001f\u007f-\u009f]/gu, "");
}

export function eventText(event: RemoteEvent): { label: string; text: string } | null {
  const read = (key: string): string => typeof event[key] === "string" ? String(event[key]) : "";
  switch (event.type) {
    case "text_delta": return { label: "Assistant", text: displayText(read("text")) };
    case "run_started": return { label: "Prompt", text: displayText(read("prompt")) };
    case "steered": return { label: "Steer", text: displayText(read("text")) };
    case "queued": return { label: "Queued", text: displayText(read("text")) };
    case "progress": return { label: "Progress", text: displayText(read("message")) };
    case "tool_started": {
      const call = object(event.call);
      return { label: "Tool", text: displayText(typeof call.name === "string" ? call.name : "Running") };
    }
    case "tool_finished": {
      const output = object(event.output);
      return { label: output.is_error === true ? "Tool error" : "Tool result", text: displayText(typeof output.content === "string" ? output.content : JSON.stringify(output)) };
    }
    case "run_finished": {
      const summary = object(event.summary);
      return { label: "Run finished", text: displayText(typeof summary.status === "string" ? summary.status : "Finished") };
    }
    default: return null;
  }
}

export class Transcript {
  entries: { label: string; text: string }[] = [];
  private bytes = 0;
  truncated = false;
  append(label: string, value: string): void {
    let text = displayText(value);
    let size = encoder.encode(text).length;
    if (size > LIMITS.output) {
      const bytes = encoder.encode(text);
      let offset = bytes.length - LIMITS.output;
      while (offset < bytes.length && (bytes[offset]! & 0xc0) === 0x80) offset++;
      text = new TextDecoder().decode(bytes.subarray(offset)); size = encoder.encode(text).length; this.truncated = true;
    }
    const previous = this.entries.at(-1);
    if (previous?.label === label && label === "Assistant" && encoder.encode(previous.text).length + size <= LIMITS.output) previous.text += text;
    else this.entries.push({ label, text });
    this.bytes += size;
    while (this.bytes > LIMITS.output || this.entries.length > LIMITS.blocks) {
      const removed = this.entries.shift();
      if (!removed) break;
      this.bytes -= encoder.encode(removed.text).length; this.truncated = true;
    }
  }
  clear(): void { for (const entry of this.entries) entry.text = ""; this.entries = []; this.bytes = 0; this.truncated = false; }
}
