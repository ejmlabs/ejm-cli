import { describe, expect, it } from "vitest";
import { FrameDecoder, FrameEncoder, LIMITS, SessionState, Transcript, client, displayText, eventText, validateServer, type ServerMessage, type Wire } from "../../web/protocol.js";

function snapshot(role: "view" | "control" = "control", cursor = 0): ServerMessage {
  return { type: "snapshot", session: { id: "session-one", title: "Offline fixture", cwd: "/private/work", status: "idle" }, events: [], through_sequence: cursor, mutation_high_water: 0, active_run: null, role, gap: null };
}
function server(message: ServerMessage): Wire { return { version: 1, body: { direction: "server", message: message as unknown as Record<string, unknown> } }; }

describe("native product framing", () => {
  it("writes the native big-endian header and bounded client JSON", () => {
    const frames = new FrameEncoder().encode(client({ type: "snapshot", after_sequence: 0, limit: 512 }));
    expect(frames).toHaveLength(1);
    const frame = frames[0]!;
    expect(Array.from(frame.slice(0, 14))).toEqual([69, 74, 77, 82, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1]);
    expect(new TextDecoder().decode(frame.subarray(22))).toBe('{"version":1,"body":{"direction":"client","message":{"type":"snapshot","after_sequence":0,"limit":512}}}');
    const view = new DataView(frame.buffer);
    expect(view.getUint32(14)).toBe(frame.length - 22); expect(view.getUint16(18)).toBe(0); expect(view.getUint16(20)).toBe(1);
    expect(() => new FrameEncoder().encode(client({ text: "x".repeat(LIMITS.message) }))).toThrow();
  });
  it("reassembles Unicode split across chunks and advances only complete messages", () => {
    const encoder = new FrameEncoder(), decoder = new FrameDecoder();
    const message = snapshot();
    if (message.type !== "snapshot") throw new Error("fixture");
    message.events = [{ sequence: 1, session_id: "session-one", run_id: null, type: "text_delta", text: "🦀".repeat(20000) }]; message.through_sequence = 1;
    const frames = encoder.encode(server(message));
    expect(frames.length).toBeGreaterThan(1);
    expect(frames.every(frame => frame.length <= LIMITS.frame)).toBe(true);
    for (const frame of frames.slice(0, -1)) expect(decoder.push(frame)).toBeNull();
    expect(decoder.push(frames.at(-1)!)).toEqual(message);
    expect(decoder.push(encoder.encode(server({ type: "outcome", sequence: 1, outcome: { type: "unknown" } }))[0]!)).toEqual({ type: "outcome", sequence: 1, outcome: { type: "unknown" } });
  });
  it("poisons gaps, replay, oversized declarations, bad counts and invalid server bodies", () => {
    for (const mutation of ["gap", "version", "size", "count", "direction", "json"] as const) {
      const decoder = new FrameDecoder();
      const clean = new FrameEncoder().encode(server(snapshot()))[0]!;
      const frame = clean.slice(), header = new DataView(frame.buffer);
      if (mutation === "gap") header.setBigUint64(6, 2n);
      if (mutation === "version") header.setUint16(4, 2);
      if (mutation === "size") header.setUint32(14, LIMITS.message + 1);
      if (mutation === "count") header.setUint16(20, 2);
      if (mutation === "direction") frame.set(new TextEncoder().encode("client"), new TextDecoder().decode(frame).indexOf("server"));
      if (mutation === "json") frame[22] = 0xff;
      expect(() => decoder.push(frame)).toThrow(); expect(() => decoder.push(clean)).toThrow();
    }
    const decoder = new FrameDecoder(), clean = new FrameEncoder().encode(server(snapshot()))[0]!;
    decoder.push(clean); expect(() => decoder.push(clean)).toThrow();
  });
  it("rejects unsafe event cursors and cross-session events before exposing output", () => {
    const base = snapshot();
    for (const sequence of [Number.MAX_SAFE_INTEGER + 1, -1, 1.5]) {
      const message = { ...base, through_sequence: sequence };
      expect(() => validateServer(server(message as ServerMessage))).toThrow();
    }
    const event = { sequence: 1, session_id: "another-session", run_id: null, type: "text_delta", text: "wrong session" };
    expect(() => validateServer(server({ ...base, events: [event], through_sequence: 1 } as ServerMessage))).toThrow();
  });
});

describe("browser action state", () => {
  it.each(["submit", "steer", "queue"] as const)("bounds %s instructions by UTF-8 bytes without consuming rejected actions", type => {
    for (const exact of ["x".repeat(16384), "🦀".repeat(4096)]) {
      const state = new SessionState(), initial = snapshot();
      if (initial.type !== "snapshot") throw new Error("fixture");
      initial.active_run = type === "submit" ? null : "run-one"; state.apply(initial);
      expect(() => state.mutation(type, exact + "x")).toThrow();
      expect(state.pending).toBeNull();
      expect(state.mutation(type, exact).body.message).toMatchObject({ mutation: { sequence: 1, action: { type, text: exact } } });
    }
  });
  it("advances new actions above host high water while retaining an uncertain action's original lookup", () => {
    const state = new SessionState(), initial = snapshot();
    if (initial.type !== "snapshot") throw new Error("fixture");
    initial.mutation_high_water = 120; state.apply(initial);
    expect(state.mutation("submit", "new action").body.message).toMatchObject({ mutation: { sequence: 121 } });
    state.disconnected(); initial.mutation_high_water = 200; state.apply(initial);
    expect(state.outcomeRequest()?.body.message).toEqual({ type: "outcome", sequence: 121 });
    expect(() => state.mutation("submit", "do not replay")).toThrow();
    state.apply({ type: "outcome", sequence: 121, outcome: { type: "rejected", code: "stale_state", message: "Run changed" } });
    expect(state.mutation("submit", "next action").body.message).toMatchObject({ mutation: { sequence: 201 } });
    for (const highWater of [undefined, -1, Number.MAX_SAFE_INTEGER + 1]) {
      expect(() => validateServer(server({ ...initial, mutation_high_water: highWater } as ServerMessage))).toThrow();
    }
  });
  it("enforces view access and targets control actions at the displayed session/run/cursor", () => {
    const state = new SessionState(); state.apply(snapshot("view"));
    expect(() => state.mutation("submit", "denied")).toThrow();
    const connected = snapshot("control", 8);
    if (connected.type !== "snapshot") throw new Error("fixture");
    connected.active_run = "run-one"; state.apply(connected);
    expect(() => state.mutation("submit", "another run")).toThrow();
    const message = state.mutation("steer", "Use the local fixture");
    expect(message.body.message).toEqual({ type: "mutation", mutation: { sequence: 1, expected: { session_id: "session-one", run_id: "run-one", event_sequence: 8 }, action: { type: "steer", run_id: "run-one", text: "Use the local fixture" } } });
    expect(() => state.mutation("cancel", "")).toThrow();
  });
  it("keeps uncertain sequences across reconnect and uses outcome lookup without replaying mutation", () => {
    const state = new SessionState(); state.apply(snapshot());
    state.mutation("submit", "Do this once"); state.disconnected();
    expect(state.pending).toMatchObject({ sequence: 1, text: "Do this once", state: "unknown" });
    state.apply(snapshot());
    expect(state.snapshotRequest().body.message.type).toBe("snapshot");
    expect(state.outcomeRequest()?.body.message).toEqual({ type: "outcome", sequence: 1 });
    state.apply({ type: "outcome", sequence: 1, outcome: { type: "unknown" } });
    expect(() => state.mutation("submit", "duplicate")).toThrow();
    state.apply({ type: "outcome", sequence: 1, outcome: { type: "accepted", run_id: "run-one" } });
    expect(state.pending).toBeNull();
    expect(state.mutation("submit", "next action").body.message).toMatchObject({ mutation: { sequence: 2 } });
  });
  it("applies a snapshot's current run once and refuses session changes, backwards replay and stopped shares", () => {
    const state = new SessionState(), initial = snapshot("control", 3);
    if (initial.type !== "snapshot") throw new Error("fixture");
    initial.active_run = null;
    initial.events = [{ sequence: 1, session_id: "session-one", run_id: "old-run", type: "run_started" }];
    state.apply(initial); expect(state.activeRun).toBeNull();
    state.apply({ type: "events", events: [{ sequence: 4, session_id: "session-one", run_id: "next-run", type: "run_started" }], through_sequence: 4, gap: null });
    expect(state.activeRun).toBe("next-run");
    state.mutation("steer", "Preserve this instruction");
    state.apply({ type: "outcome", sequence: 1, outcome: { type: "unknown" } });
    const gap = state.apply({ type: "events", events: [], through_sequence: 10, gap: { requested_after_sequence: 4, available_after_sequence: 10, reason: "output_truncated" } });
    expect(state.ready).toBe(false);
    expect(state.activeRun).toBe("next-run");
    expect(state.cursor).toBe(10);
    expect(gap.gap).toBe(true);
    expect(gap.followup?.body.message).toEqual({ type: "snapshot", after_sequence: 10, limit: LIMITS.events });
    expect(() => state.mutation("submit", "No speculative retry")).toThrow();
    for (const through of [20, 30, 40, 50]) {
      const queued = state.apply({ type: "events", events: [{ sequence: through, session_id: "session-one", run_id: "queued-run", type: "run_started" }], through_sequence: through, gap: null });
      expect(queued.events).toEqual([]);
      expect(queued.followup).toBeNull();
      expect(state.cursor).toBe(10);
      expect(state.ready).toBe(false);
      expect(state.activeRun).toBe("next-run");
    }
    const recovered = snapshot("control", 11);
    if (recovered.type !== "snapshot") throw new Error("fixture");
    recovered.mutation_high_water = 6;
    recovered.gap = { requested_after_sequence: 10, available_after_sequence: 11, reason: "output_truncated" };
    const refreshed = state.apply(recovered);
    expect(refreshed.gap).toBe(true);
    expect(refreshed.followup).toBeNull();
    expect(state.ready).toBe(true);
    expect(state.activeRun).toBeNull();
    expect(state.cursor).toBe(11);
    expect(state.pending).toMatchObject({ sequence: 1, type: "steer", text: "Preserve this instruction", state: "unknown" });
    expect(state.outcomeRequest()?.body.message).toEqual({ type: "outcome", sequence: 1 });
    expect(() => state.mutation("submit", "Wait for the original outcome")).toThrow();
    state.apply({ type: "outcome", sequence: 1, outcome: { type: "accepted", run_id: "next-run" } });
    expect(state.pending).toBeNull();
    expect(state.mutation("submit", "Next action").body.message).toMatchObject({ mutation: { sequence: 7, expected: { session_id: "session-one", run_id: null, event_sequence: 11 }, action: { type: "submit", text: "Next action" } } });
    expect(() => state.apply(snapshot("control", 2))).toThrow();
    const changed = snapshot("control", 4); if (changed.type === "snapshot") changed.session.id = "another";
    expect(() => state.apply(changed)).toThrow();
    state.apply({ type: "stopped", reason: "local_stop" });
    expect(() => state.mutation("cancel", "")).toThrow(); expect(() => state.apply(snapshot("control", 4))).toThrow();
  });
});

describe("bounded text output", () => {
  it("keeps UTF-8 and entry memory bounded and retains explicit truncation", () => {
    const transcript = new Transcript(); transcript.append("Tool result", "🦀".repeat(100000));
    expect(new TextEncoder().encode(transcript.entries[0]!.text).length).toBeLessThanOrEqual(LIMITS.output);
    expect(transcript.entries[0]!.text).not.toContain("�"); expect(transcript.truncated).toBe(true);
    for (let i = 0; i < 2000; i++) transcript.append("Progress", "short");
    expect(transcript.entries.length).toBeLessThanOrEqual(LIMITS.blocks);
    transcript.clear(); expect(transcript.entries).toEqual([]); expect(transcript.truncated).toBe(false);
  });
  it("returns literal untrusted text without interpreting HTML or terminal controls", () => {
    const sentinel = '<img src=x onerror="globalThis.pwned=true">\u001b[31mprivate\u0007';
    const output = eventText({ sequence: 1, session_id: "session-one", run_id: null, type: "text_delta", text: sentinel });
    expect(output?.text).toBe('<img src=x onerror="globalThis.pwned=true">[31mprivate');
    expect(displayText("a\nb\tc\u0000")).toBe("a\nb\tc");
  });
});
