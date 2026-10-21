import { test, expect } from "@playwright/test";
import { mkdtemp, readFile, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createLocalFixture } from "../../scripts/local-fixture.ts";
import { createAuth } from "../../src/auth.ts";

const fixtureEntry = fileURLToPath(new URL("../fixtures/oauth-worker.ts", import.meta.url));

for (const scenario of ["success", "profile-unavailable", "storage-unavailable", "provider-denied", "missing-state"]) test(`closed OAuth callback finishes on an HTML page without a browser download: ${scenario}`, async ({ browser }) => {
  const relay = await createLocalFixture();
  const temporary = await mkdtemp(path.join(tmpdir(), "ejm-browser-oauth-"));
  let context;
  try {
    let current;
    await relay.server.update(options => { current = options; return options; });
    const original = current.workers[0];
    const config = JSON.parse(await readFile(original.configPath, "utf8"));
    const configPath = path.join(temporary, "wrangler.json");
    await writeFile(configPath, JSON.stringify({ ...config, main: fixtureEntry }), { mode: 0o600 });
    await relay.server.update({ ...current, workers: [{ ...original, configPath, vars: { ...original.vars, ADMISSION_ENABLED: "false", RETENTION_CONFIG: "{}", OAUTH_FIXTURE_SCENARIO: scenario } }] });
    const worker = relay.server.getWorker(); await worker.applyD1Migrations("DB");
    const env = await worker.getEnv();
    const count = () => env.DB.prepare('SELECT (SELECT COUNT(*) FROM "user") AS users,(SELECT COUNT(*) FROM "account") AS accounts,(SELECT COUNT(*) FROM "session") AS sessions').first();
    if (scenario === "storage-unavailable") {
      const auth = await createAuth(env).$context;
      const user = await auth.internalAdapter.createUser({ email: "oauth-54321@example.invalid", name: "Offline linked account", emailVerified: true }, { method: "api" });
      await auth.internalAdapter.createAccount({ userId: user.id, providerId: "github", accountId: "54321" });
      await env.DB.prepare(`CREATE TRIGGER fixture_callback_storage_failure BEFORE UPDATE ON "account" BEGIN SELECT RAISE(ABORT,'Offline fixture storage failure'); END`).run();
    }
    const before = await count();
    context = await browser.newContext();
    await context.route("**/*", async route => {
      const request = route.request(); const url = new URL(request.url());
      if (url.origin === relay.origin) return route.continue();
      if (url.origin === "https://github.com" && url.pathname === "/login/oauth/authorize" && request.method() === "GET") {
        expect(url.searchParams.get("redirect_uri")).toBe(relay.origin + "/api/auth/callback/github");
        const query = scenario === "provider-denied"
          ? new URLSearchParams({ error: "access_denied", error_description: "SENTINEL_PROVIDER_DETAIL_<script>window.oauthDetailExecuted=true</script>", state: url.searchParams.get("state") })
          : new URLSearchParams({ code: "fixture-provider-code", state: url.searchParams.get("state") });
        const callback = relay.origin + "/api/auth/callback/github";
        return route.fulfill({ status: 302, headers: { location: scenario === "missing-state" ? callback : callback + "?" + query } });
      }
      await route.abort("blockedbyclient");
      throw new Error("Unexpected browser OAuth fixture request");
    });
    const page = await context.newPage(); const downloads = []; const responses = [];
    page.on("download", () => downloads.push(true));
    page.on("response", response => {
      const url = new URL(response.url());
      if (url.origin === relay.origin && response.request().isNavigationRequest()) responses.push({ path: url.pathname, status: response.status(), type: response.headers()["content-type"] ?? null, attachment: response.headers()["content-disposition"] ?? null });
    });
    await page.goto(relay.origin + "/account");
    const outcome = Promise.race([
      page.waitForURL(url => url.origin === relay.origin && ((url.pathname === "/account" && url.searchParams.has("error")) || (scenario === "storage-unavailable" && url.pathname === "/api/auth/callback/github")), { waitUntil: "domcontentloaded" }).then(() => "html").catch(() => "navigation-failure"),
      page.waitForEvent("download").then(() => "download"),
    ]);
    await page.getByRole("button", { name: "Sign in with GitHub" }).click();
    expect(await outcome, JSON.stringify(responses)).toBe("html");
    expect(downloads, JSON.stringify(responses)).toEqual([]);
    await expect(page.locator("h1"), JSON.stringify(responses)).toBeVisible();
    expect(responses.at(-1)?.type, JSON.stringify(responses)).toMatch(/^text\/html(?:;|$)/);
    expect(responses.every(response => response.attachment === null)).toBe(true);
    if (scenario !== "storage-unavailable") {
      await expect(page.getByRole("status")).not.toBeEmpty();
      expect(new URL(page.url()).searchParams.has("error_description")).toBe(false);
      expect(await page.locator("body").textContent()).not.toContain("SENTINEL_PROVIDER_DETAIL");
      expect(await page.evaluate(() => window.oauthDetailExecuted)).toBeUndefined();
    }
    if (scenario === "storage-unavailable") expect(responses.at(-1)?.status).toBe(500);
    expect(await count()).toEqual(before);
  } finally { await context?.close(); await relay.dispose(); await rm(temporary, { recursive: true, force: true }); }
});
