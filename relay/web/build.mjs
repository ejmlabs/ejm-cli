import { spawnSync } from "node:child_process";
import { mkdir, rm, copyFile, cp, stat } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const web = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(web, "../..");
const fixture = process.argv.includes("--fixture");
const sourceOnly = process.argv.includes("--source-only");
if (process.argv.slice(2).some(arg => !["--fixture", "--source-only"].includes(arg))) throw new Error("Unknown web build option");
const output = path.join(web, fixture ? ".fixture-dist" : "dist");
const assets = path.join(output, "assets");
function run(command, args, cwd = root, extraEnv = {}) {
  const result = spawnSync(command, args, { cwd, stdio: "inherit", env: { ...process.env, CARGO_BUILD_JOBS: "2", ...extraEnv } });
  if (result.error || result.status !== 0) throw new Error(`${command} failed`);
}
run(process.execPath, [path.join(web, "../node_modules/typescript/bin/tsc"), "--project", path.join(web, "ui/tsconfig.json")]);
run(process.execPath, [path.join(web, "ui/build-screens.mjs")]);
run(process.execPath, [path.join(web, "../node_modules/typescript/bin/tsc"), "--project", path.join(web, "tsconfig.json")]);
if (!sourceOnly) {
  const tool = spawnSync("wasm-bindgen", ["--version"], { encoding: "utf8" });
  if (tool.status !== 0 || tool.stdout.trim() !== "wasm-bindgen 0.2.129") throw new Error("Install wasm-bindgen-cli exactly 0.2.129 before building the controller");
  const compiler = spawnSync("rustup", ["which", "--toolchain", "1.99.1", "rustc"], { encoding: "utf8" });
  if (compiler.status !== 0 || !compiler.stdout.trim()) throw new Error("Install the pinned Rust1.99.1 toolchain and wasm32-unknown-unknown target");
  run("rustup", ["run", "1.99.1", "cargo", "build", "--locked", "-p", "ejm-relay-web", "--target", "wasm32-unknown-unknown", ...(fixture ? ["--features", "test-loopback"] : ["--release"])], root, { RUSTC: compiler.stdout.trim() });
}
await mkdir(assets, { recursive: true });
// Clean only this build's generated web output. The two build kinds stay separate.
for (const file of ["app.js", "protocol.js", "app.css", "ejm_relay_web.js", "ejm_relay_web_bg.wasm", "ejm_relay_web.d.ts", "ejm_relay_web_bg.wasm.d.ts"]) {
  if (sourceOnly && file.startsWith("ejm_relay_web")) continue;
  await rm(path.join(assets, file), { force: true });
}
run(process.execPath, [path.join(web, "../node_modules/typescript/bin/tsc"), "--project", path.join(web, "tsconfig.json"), "--noEmit", "false", "--outDir", assets, "--removeComments"]);
await copyFile(path.join(web, ".ui/index.html"), path.join(output, "index.html"));
for (const directory of ["common", "notices"]) {
  await rm(path.join(output, directory), { recursive: true, force: true });
  await cp(path.join(web, ".ui", directory), path.join(output, directory), { recursive: true });
}
if (!sourceOnly) {
  const target = path.resolve(root, process.env.CARGO_TARGET_DIR || "target");
  const wasm = path.join(target, "wasm32-unknown-unknown", fixture ? "debug" : "release", "ejm_relay_web.wasm");
  run("wasm-bindgen", ["--target", "web", "--out-dir", assets, wasm]);
  const wasmSize = (await stat(path.join(assets, "ejm_relay_web_bg.wasm"))).size;
  if (!fixture && wasmSize > 1572864) throw new Error("Controller WebAssembly exceeds its 1.5 MiB download budget");
  console.log(`Controller ${fixture ? "fixture" : "production"} build: ${wasmSize} bytes of WebAssembly`);
}
