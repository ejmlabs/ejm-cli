import { spawn } from "node:child_process";
import { chmod, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { URL } from "node:url";
import { createLocalFixture } from "./local-fixture.js";
import { findNativeTestBinary, nativeFixtureEnvironment } from "./native-test-binary.js";
import { LIMITS } from "../src/types.js";

const executable = await findNativeTestBinary("remote::transport::worker_fixture::worker_native_roundtrip_fixture");
const fixture = await createLocalFixture();
const temporary = await mkdtemp(path.join(tmpdir(), "ejm-native-roundtrip-"));
await chmod(temporary, 0o700);
const configPath = path.join(temporary, "config.json");
const readyPath = path.join(temporary, "ready.json");
const stopPath = path.join(temporary, "stop");
const statusPath = path.join(temporary, "status.json");
function rejectPayload(value: unknown, forbidden: string[]): void {
  const serialized = JSON.stringify(value);
  if (forbidden.some(secret => serialized.includes(secret))) throw new Error("native_fixture_relay_persisted_application_content_or_key");
}
function quoteIdentifier(identifier: string): string { return `"${identifier.replaceAll('"', '""')}"`; }
try {
  await writeFile(configPath, JSON.stringify({ origin: fixture.origin, accountId: fixture.accountId, token: fixture.token, deviceId: fixture.deviceId, role: "control", readyPath, stopPath, statusPath }), { mode: 0o600 });
  const result = await new Promise<{ code: number | null; signal: NodeJS.Signals | null; output: string }>((resolve, reject) => {
    const child = spawn(executable.path, ["--exact", executable.testName, "--ignored", "--nocapture"], { env: { ...nativeFixtureEnvironment(), EJM_REMOTE_FIXTURE_CONFIG: configPath }, stdio: ["ignore", "pipe", "pipe"] });
    let output = "";
    const record = (value: Buffer): void => { output = (output + value.toString()).slice(-16384); };
    child.stdout.on("data", record); child.stderr.on("data", record);
    let kill: ReturnType<typeof setTimeout> | undefined;
    const timeout = setTimeout(() => { child.kill("SIGTERM"); kill = setTimeout(() => child.kill("SIGKILL"), 2000); }, 90000);
    child.on("error", () => { clearTimeout(timeout); if (kill) clearTimeout(kill); reject(new Error("native_fixture_process_could_not_start")); });
    child.on("close", (code, signal) => { clearTimeout(timeout); if (kill) clearTimeout(kill); resolve({ code, signal, output }); });
  });
  // Child diagnostics stay private: assertion output can include application text.
  if (result.code !== 0 || result.signal || !result.output.includes("test result: ok. 1 passed; 0 failed;")) throw new Error("native_roundtrip_fixture_failed_or_timed_out");
  const ready = JSON.parse(await readFile(readyPath, "utf8")) as { invitation: string; origin: string; sessionId: string };
  const status = JSON.parse(await readFile(statusPath, "utf8")) as { paired?: boolean; sentinels?: unknown };
  const readyOrigin = new URL(ready.origin);
  if (readyOrigin.origin !== fixture.origin || readyOrigin.pathname !== "/" || readyOrigin.search || readyOrigin.hash || readyOrigin.username || readyOrigin.password || !ready.invitation.startsWith("ejm-remote:v1:") || !ready.sessionId || status.paired !== true || !Array.isArray(status.sentinels) || status.sentinels.length === 0 || status.sentinels.length > 16 || status.sentinels.some(value => typeof value !== "string" || value.length < 12 || value.length > 4096)) throw new Error("native_fixture_evidence_incomplete");
  const invitation = JSON.parse(Buffer.from(ready.invitation.slice("ejm-remote:v1:".length), "base64url").toString("utf8")) as { psk: string; host_public_key: string; routing_id: string };
  if (![invitation.psk, invitation.host_public_key, invitation.routing_id].every(value => typeof value === "string" && value.length >= 32)) throw new Error("native_fixture_invitation_invalid");
  const forbidden = [ready.invitation, invitation.psk, invitation.host_public_key, ready.sessionId, ...status.sentinels as string[]];
  const tables = await fixture.env.DB.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_cf_%'").all<{ name: string }>();
  let storageTables = 0;
  for (const table of tables.results) {
    const rows = await fixture.env.DB.prepare(`SELECT * FROM ${quoteIdentifier(table.name)} LIMIT 1001`).all();
    if (rows.results.length > 1000) throw new Error("native_fixture_unexpected_storage_size");
    rejectPayload(rows.results, forbidden); storageTables++;
  }
  const worker = fixture.server.getWorker();
  let durableObjects = 0;
  for (const binding of ["ROOM", "ACCOUNT_QUOTA", "PRIVACY_AUTHORITY"]) {
    for (const id of await worker.listDurableObjectIds(binding)) {
      const storage = await worker.getDurableObjectStorage(binding, { id });
      const names = await storage.exec("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_cf_%'");
      for (const row of names) {
        const name = String(row.name);
        if (binding === "ROOM" && !["__miniflare_do_name", "retirement"].includes(name)) throw new Error("native_fixture_room_has_application_storage");
        const rows = await storage.exec(`SELECT * FROM ${quoteIdentifier(name)} LIMIT 1001`);
        if (rows.length > 1000) throw new Error("native_fixture_unexpected_storage_size");
        if (binding === "ROOM" && name === "retirement") {
          const columns = await storage.exec('PRAGMA table_info("retirement")');
          const expected = ["singleton", "account_id", "expires_at"];
          if (columns.length !== expected.length || columns.some(column => !expected.includes(String(column.name))) || rows.length > 1 || rows.some(row => row.singleton !== 1 || row.account_id !== fixture.accountId || !Number.isSafeInteger(row.expires_at) || Number(row.expires_at) <= 0 || Number(row.expires_at) > Date.now() + LIMITS.shareMs)) throw new Error("native_fixture_invalid_room_retirement_metadata");
        }
        rejectPayload(rows, forbidden); storageTables++;
      }
      durableObjects++;
    }
  }
  const logs = fixture.server.getLogs();
  rejectPayload(logs, [...forbidden, fixture.token]);
  console.log(JSON.stringify({ fixture: "worker_two_native_clients", passed: true, applicationSentinelsChecked: status.sentinels.length, storageTablesChecked: storageTables, durableObjectsChecked: durableObjects, capturedLogEntries: logs.length }));
} finally {
  await writeFile(stopPath, "stop", { mode: 0o600 }).catch(() => {});
  try { await fixture.dispose(); } finally { await rm(temporary, { recursive: true, force: true }); }
}
