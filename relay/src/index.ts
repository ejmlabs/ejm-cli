import type { Env } from "./types.js";
import { accountRoute } from "./accounts.js";
import { accountPage, accountScript, devicePage, termsPage } from "./pages.js";
import { authenticate, authRoute } from "./auth.js";
import { billingRoute } from "./billing.js";
import { operationalCleanup } from "./cleanup.js";
import { diagnostic } from "./diagnostics.js";
import { failure, mutationAllowed, secure } from "./http.js";
import { webRoute } from "./web.js";
import { roomRoute } from "./rooms.js";
import { privacyRoute, privacySignal, privacyCleanup } from "./privacy/index.js";
export { Room } from "./rooms.js";
export { AccountQuota } from "./quota.js";
export { PrivacyAuthority } from "./privacy/index.js";

async function handle(request: Request, env: Env): Promise<Response> {
  const url = new URL(request.url);
  if (url.origin !== env.RELAY_ORIGIN) return failure(421, "configured_origin_required");
  const path = url.pathname;
  if (path.startsWith("/internal/")) return failure(404, "not_found");
  const account = await authenticate(request, env);
  const signalHeaders = await privacySignal(request, env, account);
  let response: Response | null = null;
  if (path === "/health" && request.method === "GET") response = Response.json({ service: "ejm-cli-relay", protocol: 1 });
  else if (path === "/account.js" && request.method === "GET") response = new Response(accountScript, { headers: { "content-type": "text/javascript;charset=utf-8" } });
  else if (path === "/account.css" || path.startsWith("/common/")) response = await webRoute(request, env, account) ?? failure(404, "not_found");
  else if (/^\/v1\/rooms\/[a-zA-Z0-9_-]{16,128}\/(host|controller)$/.test(path)) response = await roomRoute(request, env, account) ?? failure(404, "not_found");
  else if (path === "/v1/billing/webhook") response = await billingRoute(request, env, null) ?? failure(404, "not_found");
  else response = await privacyRoute(request, env, account);
  if (!response) {
    if (path.startsWith("/api/auth/")) response = await authRoute(request, env);
    else if (path === "/account" && request.method === "GET") response = accountPage(account, env);
    else if (path === "/device" && request.method === "GET") response = devicePage(account);
    else if (path === "/terms" && request.method === "GET") response = termsPage(env);
    else if (path === "/control" || path.startsWith("/control/assets/")) response = await webRoute(request, env, account) ?? failure(404, "not_found");
    else if (!account) response = failure(401, "sign_in_required");
    else if (!["GET", "HEAD"].includes(request.method) && !mutationAllowed(request, env.RELAY_ORIGIN)) response = failure(403, "origin_required");
    else response = await accountRoute(request, env, account) ?? await billingRoute(request, env, account) ?? await roomRoute(request, env, account) ?? failure(404, "not_found");
  }
  if ([...signalHeaders].length === 0) return response;
  const headers = new Headers(response.headers);
  for (const [key, value] of signalHeaders) {
    if (key.toLowerCase() === "set-cookie" && headers.has("set-cookie")) continue;
    headers.append(key, value);
  }
  return new Response(response.body, { status: response.status, statusText: response.statusText, headers, ...(response.webSocket ? { webSocket: response.webSocket } : {}) });
}
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    try { return secure(await handle(request, env), env.RELAY_ORIGIN, new URL(request.url).pathname.startsWith("/control")); }
    catch { diagnostic("account_unavailable", 503); return secure(failure(503, "service_temporarily_unavailable")); }
  },
  async scheduled(_controller: ScheduledController, env: Env): Promise<void> {
    await operationalCleanup(env);
    await privacyCleanup(env);
  },
} satisfies ExportedHandler<Env>;
