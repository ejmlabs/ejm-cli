import { spawnSync } from "node:child_process";
import { stat } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { findNativeTestBinary } from "../../scripts/native-test-binary.ts";

export default async function setup() {
  const relay = fileURLToPath(new URL("../..", import.meta.url));
  // The repository gate compiles both native tests and debug loopback WASM first.
  // Browser runs only refresh frontend sources, so Chrome never starts Cargo.
  const wasm = new URL("../../web/.fixture-dist/assets/ejm_relay_web_bg.wasm", import.meta.url);
  if (!(await stat(wasm).catch(() => null))?.isFile()) throw new Error("Build debug browser fixture WASM with npm run build:web -- --fixture first");
  await findNativeTestBinary("browser_worker_native_host_fixture");
  const build = spawnSync(process.execPath, ["web/build.mjs", "--source-only", "--fixture"], { cwd: relay, stdio: "inherit" });
  if (build.error || build.status !== 0) throw new Error("Browser fixture frontend build failed");
}
