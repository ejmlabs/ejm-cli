/// <reference types="@cloudflare/vitest-plugin/types" />
import { env } from "cloudflare:workers";
import { describe, expect, it } from "vitest";
import { privacyRoute, privacySignal } from "../../src/privacy/index.js";
import type { AccountSession, Env } from "../../src/types.js";

function fixture(): Env {
  const bindings = env as unknown as Env; const namespace = bindings.PRIVACY_AUTHORITY;
  const unique = namespace.idFromName(crypto.randomUUID());
  return { ...bindings, PRIVACY_COOKIE_SECURE: "false", PRIVACY_AUTHORITY: { idFromName: () => unique, get: (id: DurableObjectId) => namespace.get(id) } as unknown as DurableObjectNamespace };
}
function account(id = "current-account"): AccountSession { return { accountId: id, sessionId: "fixture-session", deviceId: null, email: "never-export@example.invalid", termsVersion: "fixture-v1" }; }
function request(path: string, init?: RequestInit): Request { return new Request(`https://relay.example.invalid${path}`, init); }

describe("public privacy routes and account ownership", () => {
  it("keeps browser privacy navigation branded at 401 and preserves API JSON authentication errors", async () => {
    const config = fixture();
    const navigation = await privacyRoute(request("/privacy", { headers: { accept: "text/html,application/xhtml+xml" } }), config, null);
    expect(navigation?.status).toBe(401); expect(navigation?.headers.get("content-type")).toContain("text/html");
    const html = await navigation?.text(); expect(html).toContain('href="/account"'); expect(html).toContain("Sign in to view data choices");
    expect(navigation?.headers.get("content-security-policy")).not.toContain("unsafe-inline");
    for (const path of ["/privacy", "/api/privacy/access"]) {
      const api = await privacyRoute(request(path, { headers: { accept: "application/json" } }), config, null);
      expect(api?.status).toBe(401); expect(await api?.json()).toEqual({ error: "authentication_required" });
    }
  });
  it("keeps rights and withdrawal forms usable while disabled commercial collection offers no participation forms", async () => {
    const config = fixture();
    const response = await privacyRoute(request("/privacy"), config, account());
    const actions: (string | null)[] = [];
    const parsed = new HTMLRewriter().on("form", { element: element => { actions.push(element.getAttribute("action")); } }).transform(response!);
    const html = await parsed.text();
    expect(actions).toEqual(["/api/privacy/withdraw", "/api/privacy/rights/request"]);
    expect(html).not.toContain('name="country"'); expect(html).not.toContain('name="adult"');
    const withdrawal = await privacyRoute(request("/api/privacy/withdraw", { method: "POST", headers: { origin: "https://relay.example.invalid", "content-type": "application/x-www-form-urlencoded" }, body: "category=profile" }), config, account());
    expect(withdrawal?.status).toBe(200); expect(withdrawal?.headers.get("content-type")).toContain("text/html");
    expect(await withdrawal?.text()).toContain("Data choice saved");
  });
  it("keeps failed native forms in HTML with the same denial status and safe support label", async () => {
    const response = await privacyRoute(request("/api/privacy/rights/request", { method: "POST", headers: { origin: "https://attacker.invalid", "content-type": "application/x-www-form-urlencoded" }, body: "kind=access&jurisdiction=fixture" }), fixture(), account());
    expect(response?.status).toBe(403); expect(response?.headers.get("content-type")).toContain("text/html");
    const html = await response?.text(); expect(html).toContain("same_origin_required"); expect(html).toContain("Data choice unavailable"); expect(html).not.toContain("<style");
  });
  it("saves a public optout without login and without asking for identity", async () => {
    const response = await privacyRoute(request("/do-not-sell-or-share", { method: "POST" }), { ...fixture(), PRIVACY_COOKIE_SECURE: "true" }, null);
    expect(response?.status).toBe(200);
    const preference = response?.headers.get("set-cookie") ?? "";
    expect(preference).toMatch(/^__Host-ejm_privacy=[a-f0-9-]+;/u);
    expect(preference.split("; ").slice(1).sort()).toEqual(["HttpOnly", "Max-Age=34560000", "Path=/", "SameSite=Lax", "Secure"]);
    expect(response?.headers.get("cache-control")).toBe("no-store");
    const html = await response?.text(); expect(html).toContain("preference is saved"); expect(html).not.toContain('name="email"');
  });
  it("applies GPC to an anonymous browser and later known account after the header disappears", async () => {
    const config = fixture();
    const headers = await privacySignal(request("/account", { headers: { "Sec-GPC": "1" } }), config, null);
    const saved = headers.get("set-cookie")!.split(";")[0]!;
    const competing = `ejm_privacy_local=${crypto.randomUUID()}`;
    const renewed = await privacySignal(request("/account", { headers: { cookie: `${saved}; ${competing}` } }), config, account());
    expect(renewed.get("set-cookie")?.split(";")[0]).toBe(saved);
    const status = await privacyRoute(request("/api/privacy"), config, account());
    expect(await status?.json()).toMatchObject({ optedOut: true, maySolicit: false, serviceAccessChanged: false });
  });
  it("persists a known account signal without requiring the browser cookie to return", async () => {
    const config = fixture(); await privacyRoute(request("/api/privacy", { headers: { "Sec-GPC": "1" } }), config, account());
    const later = await privacyRoute(request("/api/privacy"), config, account());
    expect(await later?.json()).toMatchObject({ optedOut: true });
  });
  it("requires authentication for private records and same-origin CSRF protection for mutations", async () => {
    const config = fixture();
    expect((await privacyRoute(request("/api/privacy/access"), config, null))?.status).toBe(401);
    for (const origin of [undefined, "null", "https://attacker.invalid"]) {
      const headers = new Headers({ "content-type": "application/json" });
      if (origin !== undefined) headers.set("origin", origin);
      const csrf = await privacyRoute(request("/api/privacy/adult", { method: "POST", headers, body: '{"adult":true}' }), config, account());
      expect(csrf?.status).toBe(403); expect(await csrf?.json()).toEqual({ error: "same_origin_required" });
    }
  });
  it("does not select another account from query parameters or leak operational identity to buyers", async () => {
    const response = await privacyRoute(request("/api/privacy/access?accountId=another-account"), fixture(), account());
    const data = await response?.json(); expect(data).toMatchObject({ accountId: "current-account", classification: "personal-data" });
    expect(JSON.stringify(data)).not.toContain("never-export@example.invalid");
  });
  it("refuses a user-supplied operator flag and returns no download authority", async () => {
    const response = await privacyRoute(request("/api/privacy/operator/prepare", { method: "POST", headers: { origin: "https://relay.example.invalid", "content-type": "application/json" }, body: JSON.stringify({ operator: true, buyerId: "invented-buyer", category: "profile", fields: ["role"], accountIds: ["current-account"], approved: true }) }), fixture(), account());
    expect(response?.status).toBe(403); expect(await response?.json()).toEqual({ error: "operator_required" }); expect(response?.headers.has("location")).toBe(false);
  });
  it("bounds bodies before forwarding them to the privacy authority", async () => {
    const response = await privacyRoute(request("/api/privacy/profile", { method: "POST", headers: { origin: "https://relay.example.invalid", "content-type": "application/json" }, body: JSON.stringify({ secret: "x".repeat(65_536) }) }), fixture(), account());
    expect(response?.status).toBe(413); expect(await response?.json()).toEqual({ error: "body_too_large" });
  });
  it("keeps a tracked rights request readable after the session is gone without restoring account authority", async () => {
    const config = fixture(); const headers = { origin: "https://relay.example.invalid", "content-type": "application/json" };
    const created = await privacyRoute(request("/api/privacy/rights/request", { method: "POST", headers, body: JSON.stringify({ kind: "access", jurisdiction: "fixture" }) }), config, account());
    expect(created?.status).toBe(200); const record = await created?.json() as { requestId: string; statusKey: string };
    const tracked = await privacyRoute(request("/api/privacy/rights/status", { method: "POST", headers, body: JSON.stringify({ requestId: record.requestId, statusKey: record.statusKey }) }), config, null);
    expect(tracked?.status).toBe(200); expect(await tracked?.json()).toMatchObject({ id: record.requestId, kind: "access", state: "acknowledged" });
    const access = await privacyRoute(request("/api/privacy/access", { headers: { "x-rights-status-key": record.statusKey } }), config, null);
    expect(access?.status).toBe(401); await access?.arrayBuffer();
  });
});
