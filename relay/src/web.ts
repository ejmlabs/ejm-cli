import type { AccountSession, Env } from "./types.js";
import { failure } from "./http.js";
import { documentHtml, templates, withCurrentNav } from "./ui.js";
const common = new Set([
  "/common/relay.css", "/common/theme-boot.js", "/common/relay-logo-ink.svg", "/common/relay-logo-paper.svg",
  "/common/relay-mark-ink.svg", "/common/relay-mark-paper.svg", "/common/relay-favicon.svg",
  "/common/manrope-latin.woff2", "/common/ibm-plex-mono-500-latin.woff2",
]);
export async function webRoute(request: Request, env: Env, account: AccountSession | null): Promise<Response | null> {
  const url = new URL(request.url);
  const publicAsset = common.has(url.pathname) || url.pathname === "/account.css";
  if (!publicAsset && url.pathname.startsWith("/common/")) return failure(404, "not_found");
  if (!publicAsset && url.pathname !== "/control" && !url.pathname.startsWith("/control/assets/")) return null;
  if (!publicAsset && !account) {
    if (url.pathname === "/control" && request.method === "GET" && request.headers.get("accept")?.includes("text/html")) {
      return withCurrentNav(new Response(documentHtml("Sign in to open the controller", templates.controllerSignIn), { status: 401, headers: { "content-type": "text/html;charset=utf-8" } }), "/control");
    }
    return failure(401, "sign_in_required");
  }
  if (!["GET", "HEAD"].includes(request.method)) return failure(405, "method_not_allowed");
  if (!env.ASSETS) return failure(503, "controller_assets_unavailable");
  const asset = new URL(env.RELAY_ORIGIN);
  asset.pathname = url.pathname === "/account.css" ? "/common/relay.css" : publicAsset ? url.pathname : url.pathname === "/control" ? "/index.html" : url.pathname.slice("/control".length);
  // Only the fixed common allowlist is public. Every controller asset needs auth.
  return env.ASSETS.fetch(new Request(asset, { method: request.method }));
}
