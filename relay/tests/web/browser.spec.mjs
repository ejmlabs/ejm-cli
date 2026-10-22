import { test, expect } from "@playwright/test";
import { mkdtemp, chmod, readFile, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";
import { createLocalFixture } from "../../scripts/local-fixture.ts";
import { findNativeTestBinary, nativeFixtureEnvironment } from "../../scripts/native-test-binary.ts";

const relayRoot = fileURLToPath(new URL("../..", import.meta.url));
async function readJson(file) { try { return JSON.parse(await readFile(file, "utf8")); } catch { return null; } }
async function hostFixture(role) {
  const binary = await findNativeTestBinary("browser_worker_native_host_fixture");
  const relay = await createLocalFixture({ assetsDirectory: "web/.fixture-dist" });
  const temporary = await mkdtemp(path.join(tmpdir(), "ejm-browser-native-")); await chmod(temporary, 0o700);
  const configPath = path.join(temporary, "config.json"), readyPath = path.join(temporary, "ready.json"), stopPath = path.join(temporary, "stop"), statusPath = path.join(temporary, "status.json");
  await writeFile(configPath, JSON.stringify({ origin: relay.origin, accountId: relay.accountId, token: relay.token, deviceId: relay.deviceId, role, readyPath, stopPath, statusPath }), { mode: 0o600 });
  const env = nativeFixtureEnvironment();
  const child = spawn(binary.path, ["--exact", binary.testName, "--ignored", "--nocapture"], { cwd: path.resolve(relayRoot, ".."), env: { ...env, EJM_REMOTE_FIXTURE_CONFIG: configPath }, stdio: ["ignore", "pipe", "pipe"] });
  // Keep child diagnostics private and bounded. Never include its config or invitation in reports.
  let diagnostics = "", ended = false;
  child.stdout.on("data", data => { diagnostics = (diagnostics + data.toString()).slice(-16384); });
  child.stderr.on("data", data => { diagnostics = (diagnostics + data.toString()).slice(-16384); });
  child.on("exit", () => { ended = true; });
  child.on("error", () => { ended = true; });
  const dispose = async () => {
    await writeFile(stopPath, "stop", { mode: 0o600 });
    for (let i = 0; i < 200 && !ended; i++) await new Promise(resolve => setTimeout(resolve, 50));
    if (!ended) {
      child.kill("SIGTERM");
      for (let i = 0; i < 40 && !ended; i++) await new Promise(resolve => setTimeout(resolve, 50));
      if (!ended) child.kill("SIGKILL");
    }
    diagnostics = "";
    await relay.dispose(); await rm(temporary, { recursive: true, force: true });
  };
  try {
    await expect.poll(async () => ended ? "failed" : (await readJson(readyPath)) ? "ready" : "waiting", { timeout: 30000, message: "Offline native host fixture starts" }).toBe("ready");
    const ready = await readJson(readyPath);
    // Readiness is written just before host WSS admission. Observe its room reservation.
    await expect.poll(async () => {
      const response = await fetch(`${relay.origin}/v1/rooms`, { headers: { authorization: `Bearer ${relay.token}` } });
      const body = await response.json(); return body.rooms?.length ?? 0;
    }, { timeout: 10000 }).toBe(1);
    return { ...relay, ready, status: () => readJson(statusPath), stop: () => writeFile(stopPath, "stop", { mode: 0o600 }), dispose };
  } catch (error) { await dispose(); throw error; }
}

async function signIn(context, fixture) {
  await context.addCookies([{ name: fixture.cookieName, value: fixture.cookieValue, domain: new URL(fixture.origin).hostname, path: "/", httpOnly: true, secure: true, sameSite: "Lax" }]);
}
async function pair(page, fixture) {
  await page.goto(`${fixture.origin}/control`);
  await expect(page.locator("#pair")).toBeEnabled();
  await page.locator("#trust-accepted").check();
  await page.locator("#invitation").fill(fixture.ready.invitation);
  await page.locator("#pair").click();
  await expect(page.locator("#connection-state")).toHaveText("Connected");
}
async function browserStorage(page) {
  return page.evaluate(async () => ({ local: Object.entries(localStorage), session: Object.entries(sessionStorage), databases: await indexedDB.databases(), caches: await caches.keys(), workers: (await navigator.serviceWorker.getRegistrations()).length }));
}

test("signed-in browser pairs native host, renders literal text, reconnects without replay, and loses pairing on reload", async ({ browser }) => {
  const fixture = await hostFixture("control");
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  try {
    await signIn(context, fixture);
    const page = await context.newPage(), requests = [], sockets = [], logs = [];
    await page.addInitScript(() => {
      const NativeWebSocket = globalThis.WebSocket;
      globalThis.WebSocket = class extends NativeWebSocket {
        constructor(...args) { super(...args); globalThis.__fixtureSocket = this; }
      };
    });
    page.on("request", request => requests.push({ url: request.url(), body: request.postData() ?? "" }));
    page.on("console", message => logs.push(message.text()));
    page.on("websocket", socket => { const captured = { url: socket.url(), frames: [], sent: 0 }; sockets.push(captured); socket.on("framesent", event => { captured.sent++; captured.frames.push(event.payload); }); socket.on("framereceived", event => captured.frames.push(event.payload)); });
    await pair(page, fixture);
    const controllerFingerprint = await page.locator("#controller-fingerprint").textContent();
    expect(controllerFingerprint).toBe((await fixture.status()).controllerFingerprint);
    expect((await fixture.status()).locallyConfirmed).toBe(true);
    expect(await page.locator("#invitation").inputValue()).toBe("");
    const oversized = "🦀".repeat(4096) + "x", sentBefore = sockets[0].sent;
    await page.locator("#prompt").fill(oversized); await page.locator("#send").click();
    await expect(page.locator("#action-status")).toContainText("16 KiB");
    await expect(page.locator("#prompt")).toHaveValue(oversized);
    await expect(page.locator("#connection-state")).toHaveText("Connected");
    await expect(page.locator("#send")).toBeEnabled();
    expect(sockets[0].sent).toBe(sentBefore);
    expect((await fixture.status()).runsStarted).toBe(0);
    const sentinel = 'browser-private-sentinel <img src=x onerror="globalThis.pwned=true">\u001b[31m';
    await page.locator("#prompt").fill(sentinel); await page.locator("#send").click();
    await expect(page.locator("#transcript")).toContainText("Demo: browser-private-sentinel");
    await expect(page.locator("#transcript img")).toHaveCount(0);
    expect(await page.evaluate(() => globalThis.pwned === true)).toBe(false);
    await expect.poll(async () => (await fixture.status())?.runsStarted).toBe(1);
    await expect(page.locator("#pending-panel")).toBeHidden();
    await expect(page.locator("#prompt")).toBeEnabled();
    // Cut only this transport. Reconnect retains identity and uses a new handshake.
    await page.evaluate(() => globalThis.__fixtureSocket.close(1000, "Fixture transport loss"));
    await expect(page.locator("#connection-state")).toHaveText("Disconnected");
    await expect.poll(async () => (await fixture.status())?.disconnections ?? 0).toBeGreaterThanOrEqual(1);
    // The host applies a 500ms first reconnect backoff after this short transport.
    await page.waitForTimeout(750);
    await page.locator("#connection-details > summary").click();
    await page.locator("#reconnect").click();
    await expect(page.locator("#connection-state")).toHaveText("Connected");
    expect(await page.locator("#controller-fingerprint").textContent()).toBe(controllerFingerprint);
    expect((await fixture.status()).confirmations).toBe(1);
    expect((await fixture.status()).runsStarted).toBe(1);
    expect(sockets.length).toBe(2);
    expect(requests.filter(request => new URL(request.url).pathname === "/v1/devices")).toHaveLength(1);
    expect(Buffer.from(sockets[0].frames[0]).equals(Buffer.from(sockets[1].frames[0]))).toBe(false);
    const invitationFields = JSON.parse(Buffer.from(fixture.ready.invitation.slice("ejm-remote:v1:".length), "base64url").toString("utf8"));
    const forbidden = [fixture.ready.invitation, invitationFields.psk, invitationFields.host_public_key, "browser-private-sentinel", fixture.ready.sessionId];
    for (const request of requests) for (const secret of forbidden) expect(request.url.includes(secret) || request.body.includes(secret)).toBe(false);
    for (const socket of sockets) {
      expect(new URL(socket.url).search).toBe("");
      for (const frame of socket.frames) expect(Buffer.from(frame).includes(Buffer.from("browser-private-sentinel"))).toBe(false);
    }
    expect(logs.some(log => forbidden.some(secret => log.includes(secret)))).toBe(false);
    expect(await browserStorage(page)).toEqual({ local: [], session: [], databases: [], caches: [], workers: 0 });
    await page.reload();
    await expect(page.locator("#connection-state")).toHaveText("Not paired");
    await expect(page.locator("#pair-panel")).toBeVisible();
    expect(await page.locator("#controller-fingerprint").textContent()).toBe("");
    expect(await page.locator("#transcript").textContent()).not.toContain("browser-private-sentinel");
    await page.locator("#trust-accepted").check(); await page.locator("#invitation").fill(fixture.ready.invitation); await page.locator("#pair").click();
    await expect(page.locator("#controller-fingerprint")).not.toHaveText(controllerFingerprint);
    await expect(page.locator("#connection-state")).toHaveText("Disconnected");
    expect((await fixture.status()).confirmations).toBe(1);
    await fixture.stop();
  } finally { await context.close(); await fixture.dispose(); }
});

test("tampered host ciphertext closes the WASM channel before exposing session output", async ({ browser }) => {
  const fixture = await hostFixture("control"), context = await browser.newContext();
  try {
    await signIn(context, fixture); const page = await context.newPage();
    await page.routeWebSocket(/\/controller$/u, route => {
      const server = route.connectToServer(); let incoming = 0;
      server.onMessage(message => {
        incoming++;
        if (incoming === 2 && typeof message !== "string") { const corrupted = Buffer.from(message); corrupted[0] ^= 1; route.send(corrupted); }
        else route.send(message);
      });
    });
    await page.goto(`${fixture.origin}/control`); await expect(page.locator("#pair")).toBeEnabled();
    await page.locator("#trust-accepted").check(); await page.locator("#invitation").fill(fixture.ready.invitation); await page.locator("#pair").click();
    await expect(page.locator("#connection-state")).toHaveText("Disconnected");
    await expect(page.locator("#status")).toContainText("authentication failed");
    await expect(page.locator("#send")).toBeDisabled();
    expect(await page.locator("#transcript").textContent()).not.toContain("private offline browser fixture");
  } finally { await context.close(); await fixture.dispose(); }
});

test("view invitation cannot send actions and host stop disables the browser", async ({ browser }) => {
  const fixture = await hostFixture("view"), context = await browser.newContext({ viewport: { width: 390, height: 844 } });
  try {
    await signIn(context, fixture); const page = await context.newPage(); await pair(page, fixture);
    await expect(page.locator("#access")).toHaveText("View only");
    for (const name of ["prompt", "action", "send", "cancel"]) await expect(page.locator(`#${name}`)).toBeDisabled();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await fixture.stop();
    await expect(page.locator("#connection-state")).toHaveText(/Sharing stopped|Disconnected/u);
    await expect(page.locator("#send")).toBeDisabled();
    await page.locator("#connection-details > summary").click();
    await page.locator("#disconnect").click();
    await expect(page.locator("#controller-fingerprint")).toHaveText("");
    await expect(page.locator("#transcript")).toHaveText("");
  } finally { await context.close(); await fixture.dispose(); }
});

test("signed-out page and every asset reject access without disclosing content", async ({ browser }) => {
  const fixture = await createLocalFixture({ assetsDirectory: "web/.fixture-dist" }), context = await browser.newContext();
  try {
    for (const route of ["/control", "/control/assets/app.js", "/control/assets/protocol.js", "/control/assets/ejm_relay_web.js", "/control/assets/ejm_relay_web_bg.wasm", "/index.html", "/assets/ejm_relay_web_bg.wasm"]) {
      const response = await context.request.get(fixture.origin + route); expect(response.status()).toBe(401);
      expect((await response.body()).length).toBeLessThan(1024);
    }
    for (const [route, type] of [["relay.css", "text/css"], ["theme-boot.js", "javascript"], ["manrope-latin.woff2", "font/woff2"], ["ibm-plex-mono-500-latin.woff2", "font/woff2"], ["relay-logo-ink.svg", "image/svg+xml"], ["relay-logo-paper.svg", "image/svg+xml"], ["relay-mark-ink.svg", "image/svg+xml"], ["relay-mark-paper.svg", "image/svg+xml"], ["relay-favicon.svg", "image/svg+xml"]]) {
      const asset = await context.request.get(fixture.origin + "/common/" + route);
      expect(asset.status()).toBe(200); expect(asset.headers()["content-type"]).toContain(type);
      const bytes = await asset.body(); expect(bytes.length).toBeGreaterThan(0);
      if (route.startsWith("relay-") && route.endsWith(".svg")) expect(bytes).toEqual(await readFile(fileURLToPath(import.meta.resolve("@ejm/tokens/assets/projects/relay/" + route.slice("relay-".length)))));
    }
    const page = await context.newPage();
    const privacy = await page.goto(fixture.origin + "/privacy");
    expect(privacy.status()).toBe(401); await expect(page.getByRole("link", { name: "Go to sign-in" })).toBeVisible();
    const notice = 'https://operator.example.invalid/notice?version=" data-injected="true&revision=2&literal=&quot;&numeric=&#34;';
    const approvedBy = 'Operator </p><script>alert("fixture")</script>';
    await fixture.server.update(options => ({ ...options, workers: options.workers.map((worker, index) => index === 0 ? { ...worker, vars: { ...worker.vars, ADMISSION_ENABLED: "false", RETENTION_CONFIG: JSON.stringify({ ...JSON.parse(worker.vars.RETENTION_CONFIG), approvedBy, externalNotice: notice }) } } : worker) }));
    await page.goto(fixture.origin + "/terms");
    await expect(page).toHaveTitle(/ \| Relay by ejmlabs$/u);
    await expect(page.locator('link[rel="icon"]')).toHaveAttribute("href", "/common/relay-favicon.svg");
    await expect(page.getByRole("link", { name: "Relay by ejmlabs, account", exact: true })).toBeVisible();
    const noticeLink = page.locator('a[data-field="noticeLink"]');
    await expect(noticeLink).toHaveCount(1); await expect(noticeLink).toHaveAttribute("href", notice);
    expect(await noticeLink.evaluate(element => element.href)).toBe(new URL(notice).href);
    await expect(page.getByText(approvedBy, { exact: true })).toBeVisible();
    await expect(page.locator("[data-injected]")).toHaveCount(0);
    expect(await page.locator("script").evaluateAll(elements => elements.map(element => element.getAttribute("src")))).toEqual(["/common/theme-boot.js", "/account.js"]);
    await signIn(context, fixture); const response = await context.request.get(fixture.origin + "/control");
    expect(response.status()).toBe(200); expect(response.headers()["content-security-policy"]).toContain("script-src 'self' 'wasm-unsafe-eval'");
    expect(response.headers()["cache-control"]).toContain("no-store");
  } finally { await context.close(); await fixture.dispose(); }
});

test("account closure confirms inline and keeps its private receipt without JavaScript", async ({ browser }) => {
  const fixture = await createLocalFixture({ assetsDirectory: "web/.fixture-dist" }), context = await browser.newContext({ javaScriptEnabled: false });
  try {
    await signIn(context, fixture);
    const page = await context.newPage(), closes = [], dialogs = [];
    page.on("request", request => { if (new URL(request.url()).pathname === "/v1/account/close") closes.push(Object.fromEntries(new URLSearchParams(request.postData()))); });
    page.on("dialog", dialog => { dialogs.push(dialog.type()); void dialog.dismiss(); });
    await page.goto(fixture.origin + "/account");
    await page.locator("summary").filter({ hasText: /^Close account$/u }).click();
    const confirmation = page.getByRole("checkbox", { name: "I want to close my relay account and revoke its devices." });
    const close = page.getByRole("button", { name: "Close account", exact: true });
    await expect(confirmation).not.toBeChecked();
    await close.click();
    expect(closes).toEqual([]);
    await confirmation.check();
    const completed = page.waitForResponse(response => new URL(response.url()).pathname === "/v1/account/close" && response.request().method() === "POST");
    await close.click();
    expect((await completed).status()).toBe(200);
    expect(closes).toEqual([{ confirm: "close" }]);
    expect(dialogs).toEqual([]);
    await expect(page.getByRole("heading", { name: "Data choice saved", exact: true })).toBeVisible();
    const receipt = JSON.parse(await page.locator("pre").textContent());
    expect(typeof receipt.requestId).toBe("string"); expect(receipt.statusKey).toMatch(/^[a-f0-9]{64}$/u);
    expect(page.url()).toBe(fixture.origin + "/v1/account/close");
    expect(page.url()).not.toContain(receipt.statusKey);
    const status = await context.request.post(fixture.origin + "/api/privacy/rights/status", { headers: { origin: fixture.origin }, data: { requestId: receipt.requestId, statusKey: receipt.statusKey } });
    expect(status.status()).toBe(200); expect(await status.json()).toMatchObject({ kind: "close", operational_state: "pending" });
  } finally { await context.close(); await fixture.dispose(); }
});

test("privacy rights and opt-out native forms work without JavaScript on a narrow screen", async ({ browser }) => {
  const fixture = await createLocalFixture({ assetsDirectory: "web/.fixture-dist" });
  const context = await browser.newContext({ viewport: { width: 390, height: 844 }, javaScriptEnabled: false, colorScheme: "dark" });
  try {
    const page = await context.newPage();
    await page.goto(fixture.origin + "/do-not-sell-or-share");
    await page.getByRole("button", { name: "Save my opt-out" }).click();
    await expect(page.getByRole("status")).toContainText("preference is saved");
    await signIn(context, fixture);
    await page.goto(fixture.origin + "/privacy");
    await expect(page.locator('form[action="/api/privacy/profile"]')).toHaveCount(0);
    await expect(page.locator('form[action="/api/privacy/adult"]')).toHaveCount(0);
    await page.getByLabel("Request kind").selectOption("access");
    await page.getByLabel("Jurisdiction").fill("fixture");
    await page.getByRole("button", { name: "Submit rights request" }).click();
    await expect(page.getByRole("heading", { name: "Data choice saved", exact: true })).toBeVisible();
    const record = JSON.parse(await page.locator("pre").textContent());
    expect(typeof record.requestId).toBe("string"); expect(typeof record.statusKey).toBe("string");
    await page.goto(fixture.origin + "/privacy/rights");
    await page.getByLabel("Request ID", { exact: true }).fill(record.requestId);
    await page.getByLabel("Private status key").fill(record.statusKey);
    await page.getByRole("button", { name: "View this request" }).click();
    await expect(page.getByRole("heading", { name: "Data choice saved", exact: true })).toBeVisible();
    const tracked = JSON.parse(await page.locator("pre").textContent());
    expect(tracked.kind).toBe("access"); expect(tracked.state).toBe("acknowledged");
  } finally { await context.close(); await fixture.dispose(); }
});
