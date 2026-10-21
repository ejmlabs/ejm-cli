import init, { BrowserPeer } from "./ejm_relay_web.js";
import { FrameDecoder, FrameEncoder, SessionState, Transcript, eventText, displayText, type ActionType, type Wire } from "./protocol.js";

interface PeerDetails { origin: string; routingId: string; role: "view" | "control"; hostFingerprint: string; controllerFingerprint: string; shareExpiresAtMs: number }
function element<T extends HTMLElement>(id: string): T {
  const node = document.getElementById(id);
  if (!node) throw new Error("Controller page is incomplete");
  return node as T;
}
const controls = {
  pairForm: element<HTMLFormElement>("pair-form"), pairPanel: element("pair-panel"), peerPanel: element("peer-panel"),
  invitation: element<HTMLInputElement>("invitation"), trust: element<HTMLInputElement>("trust-accepted"), pair: element<HTMLButtonElement>("pair"),
  state: element("connection-state"), status: element("status"), access: element("access"), connection: element<HTMLDetailsElement>("connection-details"),
  host: element("host-fingerprint"), controller: element("controller-fingerprint"), expiry: element("share-expiry"), peerSessionId: element("peer-session-id"),
  reconnect: element<HTMLButtonElement>("reconnect"), disconnect: element<HTMLButtonElement>("disconnect"),
  entryTemplate: element<HTMLTemplateElement>("transcript-entry-template"),
  transcript: element("transcript"), context: element("session-context"), heading: element("session-heading"), sessionId: element("session-id"), gap: element("gap"),
  follow: element<HTMLInputElement>("follow"), prompt: element<HTMLTextAreaElement>("prompt"), action: element<HTMLSelectElement>("action"),
  form: element<HTMLFormElement>("action-form"), send: element<HTMLButtonElement>("send"), cancel: element<HTMLButtonElement>("cancel"),
  role: element("role-note"), actionStatus: element("action-status"), pending: element("pending-panel"),
  pendingStatus: element("pending-status"), check: element<HTMLButtonElement>("check-outcome"),
  pendingDraft: element("pending-draft"), pendingText: element("pending-text"),
};
let peer: BrowserPeer | null = null;
let details: PeerDetails | null = null;
let socket: WebSocket | null = null;
let browserDeviceId: string | null = null;
let frames = new FrameEncoder();
let decoder = new FrameDecoder();
let session = new SessionState();
const transcript = new Transcript();
const transcriptNodes = new Map<{ label: string; text: string }, HTMLElement>();
const transcriptLabels: Record<string, string> = { Prompt: "you", Assistant: "ejm-cli", Steer: "you · steer", Queued: "you · queued", Progress: "progress", Tool: "tool", "Tool error": "tool · error", "Tool result": "tool · result", "Run finished": "run finished" };
let attempt = 0;
let connecting = false;
let engineReady = false;
let expiryTimer: ReturnType<typeof setTimeout> | undefined;
let handshakeTimer: ReturnType<typeof setTimeout> | undefined;

function now(): bigint { return BigInt(Date.now()); }
function status(label: string, message: string, kind = "idle"): void {
  controls.state.textContent = label; controls.state.dataset.kind = kind;
  controls.status.textContent = displayText(message); controls.status.dataset.kind = kind;
}
function safeError(error: unknown): string { return error instanceof Error ? error.message : "The connection failed. Reconnect after checking the host."; }
async function api(path: string, data?: unknown): Promise<Record<string, unknown>> {
  const response = await fetch(path, {
    method: data === undefined ? "GET" : "POST", credentials: "same-origin", cache: "no-store", redirect: "error",
    headers: data === undefined ? { accept: "application/json" } : { "content-type": "application/json", accept: "application/json" },
    ...(data === undefined ? {} : { body: JSON.stringify(data) }), signal: AbortSignal.timeout(10000),
  });
  if (response.status === 401) throw new Error("Sign in to your relay account again, then pair a fresh invitation.");
  if (response.status === 403) throw new Error("This account or device cannot connect. Check current terms and device access in Account.");
  if (response.status === 429) throw new Error("The relay allowance is exhausted. Check your plan in Account.");
  if (!response.ok) throw new Error("The relay could not admit this connection. Check the host and your account.");
  const text = await response.text();
  if (text.length > 16384) throw new Error("Invalid relay response");
  const result: unknown = JSON.parse(text);
  if (!result || typeof result !== "object" || Array.isArray(result)) throw new Error("Invalid relay response");
  return result as Record<string, unknown>;
}

function refreshActions(): void {
  const connected = socket?.readyState === WebSocket.OPEN && session.ready && !session.stopped;
  const allowed = connected && session.role === "control" && !session.pending;
  controls.prompt.disabled = !allowed; controls.action.disabled = !allowed; controls.send.disabled = !allowed;
  controls.cancel.disabled = !allowed || session.activeRun === null;
  for (const option of controls.action.options) option.disabled = option.value === "submit" ? session.activeRun !== null : session.activeRun === null;
  if (session.activeRun === null && controls.action.value !== "submit") controls.action.value = "submit";
  if (session.activeRun !== null && controls.action.value === "submit") controls.action.value = "steer";
  controls.role.textContent = session.ready ? (session.role === "view" ? "View access" : session.activeRun ? "Control access, run active" : "Control access, ready") : "Waiting for a session";
  controls.reconnect.disabled = !peer || connecting || socket?.readyState === WebSocket.OPEN || session.stopped;
  controls.pending.hidden = session.pending === null;
  controls.pendingText.textContent = session.pending?.text ?? "";
  controls.pendingDraft.hidden = !session.pending?.text;
  controls.check.disabled = !connected || !session.pending;
  if (session.pending) {
    controls.pendingStatus.textContent = session.pending.state === "pending"
      ? `Action ${session.pending.sequence} is pending. Further actions wait for its outcome.`
      : `Action ${session.pending.sequence} has an unknown outcome. It may have run. Check its outcome; do not resend it. If no outcome remains, check the host before starting a new share.`;
  }
}

function renderTranscript(): void {
  const retained = new Set(transcript.entries);
  for (const [item, node] of transcriptNodes) {
    if (!retained.has(item)) { node.remove(); transcriptNodes.delete(item); }
  }
  if (!transcriptNodes.size) controls.transcript.replaceChildren();
  for (const item of transcript.entries) {
    let entry = transcriptNodes.get(item);
    if (!entry) {
      entry = controls.entryTemplate.content.firstElementChild?.cloneNode(true) as HTMLElement | undefined;
      if (!entry) throw new Error("Controller entry template is missing");
      entry.dataset.kind = item.label === "Prompt" || item.label === "Steer" || item.label === "Queued" ? "prompt" : item.label === "Assistant" ? "assistant" : item.label === "Tool error" ? "error" : "output";
      const label = entry.querySelector("[data-terminal-label]");
      if (!label) throw new Error("Controller entry label is missing");
      label.textContent = transcriptLabels[item.label] ?? item.label;
      transcriptNodes.set(item, entry); controls.transcript.append(entry);
    }
    const text = entry.querySelector("[data-terminal-text]");
    if (text && text.textContent !== item.text) text.textContent = item.text;
  }
  controls.gap.hidden = !transcript.truncated;
  if (controls.follow.checked) controls.transcript.scrollTop = controls.transcript.scrollHeight;
}

function send(wire: Wire): void {
  if (!peer || !socket || socket.readyState !== WebSocket.OPEN) throw new Error("The host is disconnected");
  if (socket.bufferedAmount > 262144) throw new Error("The connection is too slow. Reconnect to recover the outcome.");
  for (const plain of frames.encode(wire)) {
    let ciphertext: Uint8Array | undefined;
    try { ciphertext = peer.encrypt(plain, now()); socket.send(ciphertext as Uint8Array<ArrayBuffer>); }
    finally { plain.fill(0); ciphertext?.fill(0); }
  }
}

function closeTransport(): void {
  clearTimeout(handshakeTimer); handshakeTimer = undefined;
  const current = socket; socket = null;
  current?.close(1000, "Controller disconnected"); decoder.clear(); session.disconnected();
}

function transportFailed(error: unknown): void {
  closeTransport(); connecting = false;
  status("Disconnected", safeError(error), "error"); refreshActions();
}

async function connect(): Promise<void> {
  if (!peer || !details || connecting || session.stopped) return;
  closeTransport(); connecting = true;
  const generation = ++attempt;
  const currentPeer = peer, currentDetails = details;
  status("Connecting", "Checking this browser's relay device and requesting one connection ticket."); refreshActions();
  try {
    if (browserDeviceId === null) {
      const device = await api("/v1/devices", { label: "Browser controller", osFamily: "other", cliMajor: 1 });
      if (generation !== attempt) return;
      if (typeof device.id !== "string" || !/^[a-zA-Z0-9_-]{16,128}$/.test(device.id)) throw new Error("Invalid browser device response");
      browserDeviceId = device.id;
    }
    const ticket = await api(`/v1/rooms/${currentDetails.routingId}/tickets`, { role: "controller", deviceId: browserDeviceId });
    if (generation !== attempt) return;
    if (typeof ticket.ticket !== "string" || !/^[a-zA-Z0-9_-]{43}$/.test(ticket.ticket) || ticket.room !== currentDetails.routingId || ticket.role !== "controller") throw new Error("Invalid connection ticket");
    const url = new URL(`/v1/rooms/${currentDetails.routingId}/controller`, currentDetails.origin);
    url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
    const current = new WebSocket(url, ["ejm-relay-v1", `ticket.${ticket.ticket}`]);
    // This object is short lived. Tickets never enter URLs, UI, logs or storage.
    delete ticket.ticket;
    socket = current; current.binaryType = "arraybuffer";
    frames = new FrameEncoder(); decoder = new FrameDecoder();
    let handshaking = true;
    current.onopen = () => {
      if (generation !== attempt || socket !== current) return;
      try {
        if (current.protocol !== "ejm-relay-v1") throw new Error("The relay selected an invalid protocol");
        const first = currentPeer.begin(now()); current.send(first as Uint8Array<ArrayBuffer>); first.fill(0);
        status("Verifying host", "Authenticating the host from your private invitation.");
        handshakeTimer = setTimeout(() => transportFailed(new Error("The host did not complete pairing. Keep its share open, then reconnect.")), 15000);
      } catch (error) { transportFailed(error); }
    };
    current.onmessage = event => {
      if (generation !== attempt || socket !== current) return;
      try {
        if (!(event.data instanceof ArrayBuffer) || event.data.byteLength > 16384) throw new Error("The relay sent an invalid binary frame");
        const bytes = new Uint8Array(event.data);
        if (handshaking) {
          const third = currentPeer.acceptHandshake(bytes, now()); current.send(third as Uint8Array<ArrayBuffer>); third.fill(0); bytes.fill(0);
          clearTimeout(handshakeTimer); handshaking = false; connecting = false;
          status("Waiting for host", "Compare this tab's full fingerprint on the host and approve it there. Session data appears only after host approval.");
          send(session.snapshotRequest()); refreshActions(); return;
        }
        const plain = currentPeer.decrypt(bytes, now()); bytes.fill(0);
        let message;
        try { message = decoder.push(plain); } finally { plain.fill(0); }
        if (!message) return;
        if (message.type === "snapshot" && message.role !== currentDetails.role) throw new Error("The host changed the invitation's access role");
        const pendingText = session.pending?.text;
        const newlyApproved = message.type === "snapshot" && session.sessionId === null;
        const result = session.apply(message);
        if (result.gap) transcript.truncated = true;
        for (const event of result.events) { const output = eventText(event); if (output) transcript.append(output.label, output.text); }
        if (result.events.length || result.gap) renderTranscript();
        if (message.type === "snapshot") {
          controls.heading.textContent = displayText(message.session.title || "Shared session");
          controls.context.textContent = displayText(message.session.cwd);
          controls.context.title = displayText(message.session.cwd);
          controls.sessionId.textContent = displayText(message.session.id); controls.sessionId.title = displayText(message.session.id); controls.sessionId.hidden = false;
          controls.peerSessionId.textContent = displayText(message.session.id);
          if (newlyApproved) {
            const detailsFocused = controls.connection.contains(document.activeElement);
            controls.connection.open = false;
            if (detailsFocused) controls.connection.querySelector("summary")?.focus();
          }
          controls.access.textContent = message.role === "view" ? "View only" : "Control";
          controls.actionStatus.textContent = message.role === "view" ? "The host shared view access. You can read the session." : "Actions use the host's current session and run state.";
          status("Connected", "The host approved this tab. Session traffic is encrypted between these endpoints.", "connected");
          const lookup = session.outcomeRequest(); if (lookup) send(lookup);
        }
        if (result.resolved) {
          if (result.resolved.type === "rejected") {
            if (!controls.prompt.value && pendingText) controls.prompt.value = pendingText;
            controls.actionStatus.textContent = `Action rejected: ${displayText(result.resolved.message)} Your draft is preserved.`;
          } else controls.actionStatus.textContent = result.resolved.type === "queued" ? "Queue acknowledged. It may still be discarded; watch the transcript for its Prompt entry when it starts." : "Action accepted by the host.";
        }
        if (message.type === "error") controls.actionStatus.textContent = displayText(message.message);
        if (message.type === "stopped") {
          closeTransport(); status("Sharing stopped", stopReason(message.reason), "error");
        }
        refreshActions();
      } catch (error) { transportFailed(error); }
    };
    current.onerror = () => { if (generation === attempt && socket === current) transportFailed(new Error("The relay connection failed. Check that the host is sharing and your account has relay access.")); };
    current.onclose = () => {
      if (generation !== attempt || socket !== current) return;
      socket = null; clearTimeout(handshakeTimer); decoder.clear(); connecting = false; session.disconnected();
      status("Disconnected", "The connection closed. Keep the host's share open and reconnect. Any unresolved action keeps its original sequence.", "error"); refreshActions();
    };
  } catch (error) { if (generation === attempt) transportFailed(error); }
}

function stopReason(reason: string): string {
  switch (reason) {
    case "local_stop": return "The host stopped sharing. Pair a new invitation to start another share.";
    case "host_exit": return "The host exited. Start a new share on the host.";
    case "session_changed": return "The host changed sessions. Pair a new invitation for that session.";
    case "expired": return "This share expired. Start a new share on the host.";
    default: return "The host closed this share. Start a new share on the host.";
  }
}

function disconnect(): void {
  ++attempt; closeTransport(); clearTimeout(expiryTimer); expiryTimer = undefined; connecting = false;
  peer?.free(); peer = null; details = null; browserDeviceId = null; session = new SessionState(); transcript.clear();
  transcriptNodes.clear();
  controls.transcript.replaceChildren(); controls.prompt.value = ""; controls.invitation.value = "";
  controls.host.textContent = ""; controls.controller.textContent = ""; controls.expiry.textContent = "";
  controls.heading.textContent = "Shared session"; controls.context.textContent = "Pair a host to view its session."; controls.context.title = "";
  controls.sessionId.textContent = ""; controls.sessionId.title = ""; controls.sessionId.hidden = true; controls.connection.open = false;
  controls.peerSessionId.textContent = "Awaiting host approval";
  controls.peerPanel.hidden = true; controls.pairPanel.hidden = false; controls.pair.disabled = !engineReady;
  controls.gap.hidden = true; controls.actionStatus.textContent = "View access keeps these actions disabled.";
  status("Not paired", "Private keys and reachable session buffers were cleared. Pair a fresh invitation to connect."); refreshActions();
}

controls.pairForm.addEventListener("submit", event => {
  event.preventDefault();
  if (!engineReady || !controls.trust.checked || peer) return;
  const invitation = controls.invitation.value; controls.invitation.value = "";
  try {
    peer = new BrowserPeer(invitation, location.origin, now());
    details = JSON.parse(peer.metadata()) as PeerDetails;
    controls.host.textContent = details.hostFingerprint; controls.controller.textContent = details.controllerFingerprint;
    controls.expiry.textContent = new Date(details.shareExpiresAtMs).toLocaleString();
    controls.access.textContent = details.role === "view" ? "View only, awaiting approval" : "Control, awaiting approval";
    controls.pairPanel.hidden = true; controls.peerPanel.hidden = false; controls.connection.open = true;
    controls.connection.querySelector("summary")?.focus();
    expiryTimer = setTimeout(() => { session.stopped = true; closeTransport(); status("Share expired", "Start a new share on the host and pair its fresh invitation.", "error"); refreshActions(); }, Math.max(0, details.shareExpiresAtMs - Date.now()));
    void connect();
  } catch (error) { peer?.free(); peer = null; details = null; status("Pairing failed", safeError(error), "error"); }
});
controls.reconnect.addEventListener("click", () => { void connect(); });
controls.disconnect.addEventListener("click", disconnect);
controls.form.addEventListener("submit", event => { event.preventDefault(); act(controls.action.value as ActionType); });
controls.cancel.addEventListener("click", () => act("cancel"));
controls.check.addEventListener("click", () => { try { const lookup = session.outcomeRequest(); if (lookup) send(lookup); } catch (error) { transportFailed(error); } });
controls.follow.addEventListener("change", () => { if (controls.follow.checked) controls.transcript.scrollTop = controls.transcript.scrollHeight; });
window.addEventListener("pagehide", disconnect);

function act(type: ActionType): void {
  try {
    const wire = session.mutation(type, controls.prompt.value);
    send(wire); if (type !== "cancel") controls.prompt.value = "";
    controls.actionStatus.textContent = "Waiting for the host to confirm this action's outcome.";
  } catch (error) {
    if (session.pending) transportFailed(error);
    else controls.actionStatus.textContent = safeError(error);
  }
  refreshActions();
}

void init().then(() => {
  engineReady = true; controls.pair.disabled = false;
  status("Not paired", "The encryption engine is ready. Paste a fresh invitation from your host.");
}).catch(() => status("Encryption unavailable", "The encryption engine could not load. Reload this page or use the native controller.", "error"));
