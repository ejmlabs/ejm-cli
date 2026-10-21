import { createTestHarness } from "wrangler";
import { mkdtemp, readFile, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath, URL } from "node:url";
import { serializeSignedCookie } from "better-call";
import { createAuth } from "../src/auth.js";
import type { Env } from "../src/types.js";

// This module runs only in Node test tooling. It is never part of the Worker bundle.
export async function createLocalFixture(options: { assetsDirectory?: string } = {}): Promise<{
  origin: string; cookie: string; cookieName: string; cookieValue: string;
  accountId: string; token: string; deviceId: string; sessionId: string;
  env: Env; server: ReturnType<typeof createTestHarness>; dispose: () => Promise<void>;
}> {
  const vars = {
    TERMS_VERSION: "local-fixture-v1", ADMISSION_ENABLED: "true", PRIVACY_COOKIE_SECURE: "false",
    RETENTION_CONFIG: JSON.stringify({ approved: true, version: "fixture-v1", approvedBy: "fixture-operator", accountActivePurpose: "Authenticate active relay accounts and enforce access, billing and user rights.", unadmittedDays: 7, accountClosedDays: 30, providerIdentityDays: 30, paymentDays: 365, externalNotice: "https://example.invalid/terms", legalHoldPolicy: "Fixture only: reason, evidence and expiry are required." }),
    RELAY_ORIGIN: "https://uninitialized.invalid", COMMERCIAL_CONFIG: "{}",
  };
  const secrets = { BETTER_AUTH_SECRET: "local-fixture-only-at-least-thirty-two-characters-long", GITHUB_CLIENT_ID: "local-fixture", GITHUB_CLIENT_SECRET: "local-fixture" };
  const root = fileURLToPath(new URL("..", import.meta.url));
  const temporary = await mkdtemp(path.join(tmpdir(), "ejm-relay-fixture-"));
  const config = JSON.parse(await readFile(path.join(root, "wrangler.jsonc"), "utf8")) as Record<string, unknown>;
  config.main = path.join(root, "src/index.ts");
  config.d1_databases = [{ binding: "DB", database_name: "fixture-only", database_id: "00000000-0000-0000-0000-000000000003", migrations_dir: path.join(root, "migrations") }];
  if (options.assetsDirectory) config.assets = { directory: path.resolve(root, options.assetsDirectory), binding: "ASSETS", run_worker_first: true };
  else if (config.assets && typeof config.assets === "object" && "directory" in config.assets) config.assets = { ...config.assets, directory: path.resolve(root, String(config.assets.directory)) };
  const configPath = path.join(temporary, "wrangler.json");
  await writeFile(configPath, JSON.stringify(config), { mode: 0o600 });
  const server = createTestHarness({ root, workers: [{ configPath, vars, secrets }] });
  const dispose = async (): Promise<void> => { try { await server.close(); } finally { await rm(temporary, { recursive: true, force: true }); } };
  try {
    const { url } = await server.listen();
    const origin = url.origin;
    if (url.protocol !== "http:" || url.hostname !== "127.0.0.1") throw new Error("fixture_requires_http_loopback");
    await server.update({ root, workers: [{ configPath, vars: { ...vars, RELAY_ORIGIN: origin }, secrets }] });
    const worker = server.getWorker<Env>();
    await worker.applyD1Migrations("DB");
    const env = await worker.getEnv();
    const context = await createAuth(env).$context;
    const user = await context.internalAdapter.createUser({ email: "local-fixture@example.invalid", name: "Offline fixture", emailVerified: true }, { method: "api" });
    const session = await context.internalAdapter.createSession(user.id);
    const call = async (path: string, body: Record<string, unknown>): Promise<Response> => fetch(origin + path, { method: "POST", headers: { authorization: `Bearer ${session.token}`, "content-type": "application/json" }, body: JSON.stringify(body) });
    const terms = await call("/v1/account/terms", { version: env.TERMS_VERSION });
    await terms.text();
    if (!terms.ok) throw new Error("fixture_terms_rejected");
    const registration = await call("/v1/devices", { label: "Offline fixture host", osFamily: "other", cliMajor: 1 });
    const device = await registration.json() as { id?: string };
    if (!registration.ok || !device.id) throw new Error("fixture_device_rejected");
    const signed = (await serializeSignedCookie(context.authCookies.sessionToken.name, session.token, context.secret)).split(";")[0]!;
    const equal = signed.indexOf("=");
    return { origin, cookie: signed, cookieName: signed.slice(0, equal), cookieValue: signed.slice(equal + 1), accountId: user.id, token: session.token, sessionId: session.id, deviceId: device.id, env, server, dispose };
  } catch (error) { await dispose(); throw error; }
}
