import { createLocalFixture } from "./local-fixture.js";
const fixture = await createLocalFixture();
try {
  const worker = fixture.server.getWorker(); const room = "opaque_local_load_fixture";
  const ticket = async (role: string): Promise<string> => {
    const response = await fetch(`${fixture.origin}/v1/rooms/${room}/tickets`, { method: "POST", headers: { authorization: `Bearer ${fixture.token}`, "content-type": "application/json" }, body: JSON.stringify({ role, deviceId: fixture.deviceId }) });
    const result = await response.json() as { ticket: string };
    if (!response.ok) throw new Error("load_fixture_ticket_failed"); return result.ticket;
  };
  const hostTicket = await ticket("host"); const controllerTicket = await ticket("controller");
  const connect = async (role: string, secret: string) => {
    const response = await worker.fetch(`${fixture.origin}/v1/rooms/${room}/${role}`, { headers: { upgrade: "websocket", authorization: `Bearer ${secret}` } });
    if (response.status !== 101 || !response.webSocket) throw new Error("load_fixture_upgrade_failed");
    response.webSocket.accept(); return response.webSocket;
  };
  const host = await connect("host", hostTicket); const controller = await connect("controller", controllerTicket);
  const quota = await worker.getDurableObjectStorage("ACCOUNT_QUOTA", { name: fixture.accountId });
  const before = await quota.exec("SELECT day,frames,bytes,controls FROM windows");
  const frame = new Uint8Array(4096); const sentinel = "SENTINEL_local_load_ciphertext"; frame.set(new TextEncoder().encode(sentinel));
  const timings: number[] = []; const started = performance.now();
  for (let index = 0; index < 90; index++) {
    const sender = index % 2 === 0 ? host : controller; const peer = index % 2 === 0 ? controller : host;
    const receiving = new Promise<number>((resolve, reject) => { const onClose = (): void => reject(new Error("load_fixture_closed")); peer.addEventListener("close", onClose, { once: true }); peer.addEventListener("message", event => { peer.removeEventListener("close", onClose); resolve((event.data as ArrayBuffer).byteLength); }, { once: true }); });
    const one = performance.now(); sender.send(frame);
    if (await receiving !== frame.byteLength) throw new Error("load_fixture_frame_mismatch"); timings.push(performance.now() - one);
  }
  const elapsedMs = performance.now() - started; timings.sort((a, b) => a - b);
  const after = await quota.exec("SELECT day,frames,bytes,controls FROM windows");
  if (JSON.stringify(before) !== JSON.stringify(after)) throw new Error("load_fixture_ledger_changed");
  const roomStorage = await worker.getDurableObjectStorage("ROOM", { name: room });
  const roomTables = await roomStorage.exec("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE '_cf_%'");
  const log = JSON.stringify(fixture.server.getLogs());
  if ([sentinel, fixture.token, hostTicket, controllerTicket].some(value => log.includes(value))) throw new Error("load_fixture_log_contains_secret");
  console.log(JSON.stringify({ fixture: "relay_90_frames", frames: 90, forwardedBytes: 90 * frame.byteLength, elapsedMs, p50Ms: timings[45], p95Ms: timings[85], quotaBefore: before, quotaAfter: after, roomApplicationTables: roomTables, capturedLogEntries: fixture.server.getLogs().length }));
  host.close(); controller.close();
} finally { await fixture.dispose(); }
