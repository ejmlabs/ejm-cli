import { execFile } from "node:child_process";
import { constants } from "node:fs";
import { access } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, URL } from "node:url";
import { promisify } from "node:util";

const execute = promisify(execFile);
const repository = fileURLToPath(new URL("../..", import.meta.url));

// Test executables receive only platform runtime settings, never provider credentials.
export function nativeFixtureEnvironment(): NodeJS.ProcessEnv {
  const allowed = ["PATH", "HOME", "TMPDIR", "TEMP", "TMP", "SYSTEMROOT", "WINDIR", "DYLD_FALLBACK_LIBRARY_PATH"];
  return Object.fromEntries(allowed.flatMap(key => process.env[key] === undefined ? [] : [[key, process.env[key]]]));
}

export async function findNativeTestBinary(requiredTest: string): Promise<{ path: string; testName: string }> {
  const messages = process.env.EJM_NATIVE_TEST_ARTIFACTS;
  if (!messages) throw new Error("native_fixture_requires_current_cargo_artifacts_run_root_scripts_check");
  const { stdout: selected } = await execute(process.env.EJM_FIXTURE_PYTHON ?? "python3", [path.join(repository, "scripts/check_terminal.py"), "--native-fixture", path.resolve(repository, messages)], {
    env: nativeFixtureEnvironment(), timeout: 10000, maxBuffer: 1024 * 1024,
  });
  const binary = selected.trim();
  if (!binary || /[\r\n]/u.test(binary)) throw new Error("native_fixture_artifact_path_invalid");
  await access(binary, constants.X_OK);
  const { stdout } = await execute(binary, ["--list", "--format", "terse"], { env: nativeFixtureEnvironment(), timeout: 10000, maxBuffer: 1024 * 1024 });
  const matches = stdout.split(/\r?\n/u).filter(line => line.endsWith(": test")).map(line => line.slice(0, -6)).filter(name => name === requiredTest || name.endsWith(`::${requiredTest}`));
  if (matches.length > 1) throw new Error("native_fixture_test_name_is_ambiguous");
  if (!matches[0]) throw new Error("native_fixture_test_missing_from_current_artifact");
  return { path: binary, testName: matches[0] };
}
