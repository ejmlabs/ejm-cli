import { APIError, betterAuth, type BetterAuthOptions, type BetterAuthPlugin, type DBAdapter } from "better-auth";
import type { GithubProfile } from "better-auth/social-providers";
import { bearer, deviceAuthorization } from "better-auth/plugins";
import type { DatabaseSync } from "node:sqlite";
import { LIMITS, type Env, type AccountSession } from "./types.js";
import { boundedBytes, credentialHeaders, failure, integer, mutationAllowed, onlyKeys } from "./http.js";
import { retentionPolicy } from "./retention.js";
import { authFailurePage } from "./pages.js";

// Guard the maintained plugin's terminal consent update against concurrent approval/denial.
const atomicDeviceConsent: BetterAuthPlugin = {
  id: "ejm-relay-atomic-device-consent",
  init(context) {
    const adapter = context.adapter;
    const update: DBAdapter["update"] = async <T>(data: Parameters<DBAdapter["update"]>[0]): Promise<T | null> => {
      if (data.model !== "deviceCode" || (data.update.status !== "approved" && data.update.status !== "denied")) return adapter.update<T>(data);
      const userId: unknown = data.update.userId;
      if (typeof userId !== "string") throw new APIError("BAD_REQUEST", { error: "invalid_request" });
      const row = await adapter.incrementOne<T>({ model: data.model, where: [...data.where, { field: "status", value: "pending" }, { field: "userId", value: userId }], increment: {}, set: data.update });
      if (row === null) throw new APIError("BAD_REQUEST", { error: "invalid_request" });
      return row;
    };
    return { context: { adapter: { ...adapter, update } } };
  },
};
// Better Auth owns OAuth state, PKCE, exchange and sessions. Its supported profile hook
// selects the verified primary email; the default provider may prefer a public secondary.
export async function verifiedGithubIdentity(accessToken: string | undefined): Promise<{ user: { name: string; email: string; emailVerified: boolean; image: string }; data: GithubProfile } | null> {
  if (!accessToken) return null;
  try {
    const headers = { authorization: `Bearer ${accessToken}`, "user-agent": "ejm-cli-relay", accept: "application/vnd.github+json" };
    const responses = await Promise.all(["https://api.github.com/user", "https://api.github.com/user/emails"].map(url => fetch(url, { headers, redirect: "manual" })));
    const bodies = await Promise.all(responses.map(response => boundedBytes(response)));
    if (responses.some(response => !response.ok) || bodies.some(bytes => bytes === null)) return null;
    const parsed: unknown[] = [];
    for (const bytes of bodies) {
      if (!bytes) return null;
      try { parsed.push(JSON.parse(new TextDecoder().decode(bytes))); } catch { return null; }
    }
    const profile = parsed[0]; const emails = parsed[1];
    if (!profile || typeof profile !== "object" || !("id" in profile) || !Array.isArray(emails)) return null;
    const primary: unknown = emails.find((entry: unknown) => entry !== null && typeof entry === "object" && "primary" in entry && entry.primary === true && "verified" in entry && entry.verified === true);
    if (!primary || typeof primary !== "object" || !("email" in primary) || typeof primary.email !== "string" || primary.email.length > 320) return null;
    const subject = String(profile.id);
    if (!/^\d{1,32}$/.test(subject)) return null;
    return { user: { name: "Relay account", email: primary.email, emailVerified: true, image: "" }, data: { ...profile, id: subject } as GithubProfile };
  } catch { return null; }
}
export function authOptions(env: Pick<Env, "RELAY_ORIGIN" | "BETTER_AUTH_SECRET" | "GITHUB_CLIENT_ID" | "GITHUB_CLIENT_SECRET" | "ADMISSION_ENABLED" | "ADMISSION_LIMIT" | "RETENTION_CONFIG">, database: D1Database | DatabaseSync): BetterAuthOptions {
  return {
    database,
    appName: "cli by ejmlabs relay",
    baseURL: env.RELAY_ORIGIN,
    basePath: "/api/auth",
    secret: env.BETTER_AUTH_SECRET ?? "",
    trustedOrigins: [env.RELAY_ORIGIN],
    onAPIError: { errorURL: `${env.RELAY_ORIGIN}/account` },
    // Recomputed for each callback; linked accounts can still sign in when new signup is closed.
    socialProviders: { github: { clientId: env.GITHUB_CLIENT_ID ?? "", clientSecret: env.GITHUB_CLIENT_SECRET ?? "", disableSignUp: env.ADMISSION_ENABLED !== "true" || integer(env.ADMISSION_LIMIT, LIMITS.accounts, LIMITS.accounts) === 0 || !retentionPolicy(env), disableDefaultScope: true, scope: ["user:email"], getUserInfo: async (tokens) => verifiedGithubIdentity(tokens.accessToken) } },
    account: { accountLinking: { enabled: false }, storeAccountCookie: false },
    session: { expiresIn: 60 * 60 * 24 * 7, updateAge: 60 * 60 * 24, cookieCache: { enabled: false } },
    databaseHooks: {
      session: { create: { before: async (session) => ({ data: { ...session, ipAddress: null, userAgent: null } }) } },
      account: {
        create: { before: async (account) => ({ data: { ...account, accessToken: null, refreshToken: null, idToken: null, accessTokenExpiresAt: null, refreshTokenExpiresAt: null } }) },
        update: { before: async (account) => ({ data: { ...account, accessToken: null, refreshToken: null, idToken: null, accessTokenExpiresAt: null, refreshTokenExpiresAt: null } }) },
      },
    },
    // No client-IP header is trusted. The maintained limiter uses a shared per-path
    // bucket; disableIpTracking would disable that limiter entirely in Better Auth.
    advanced: { useSecureCookies: true, ipAddress: { ipAddressHeaders: [] }, defaultCookieAttributes: { httpOnly: true, secure: true, sameSite: "lax" } },
    rateLimit: { enabled: true, storage: "database", window: 60, max: 60, customRules: { "/device/code": { window: 60, max: 10 }, "/device/token": { window: 60, max: 30 }, "/sign-in/social": { window: 60, max: 10 } } },
    logger: { disabled: true },
    plugins: [bearer(), deviceAuthorization({ expiresIn: "10m", interval: "5s", verificationUri: `${env.RELAY_ORIGIN}/device`, validateClient: (id: string) => id === "ejm-cli-relay" }), atomicDeviceConsent],
  } satisfies BetterAuthOptions;
}
export function createAuth(env: Env): ReturnType<typeof betterAuth<BetterAuthOptions>> { return betterAuth(authOptions(env, env.DB)); }
export function authConfigured(env: Env): boolean {
  return Boolean(env.BETTER_AUTH_SECRET && env.BETTER_AUTH_SECRET.length >= 32 && env.GITHUB_CLIENT_ID && env.GITHUB_CLIENT_SECRET);
}
export async function authenticate(request: Request, env: Env): Promise<AccountSession | null> {
  if (!authConfigured(env)) return null;
  const headers = credentialHeaders(request.headers);
  if (!headers) return null;
  const result = await createAuth(env).api.getSession({ headers });
  if (!result || !result.user.emailVerified) return null;
  const account = await env.DB.prepare("SELECT terms_version,status FROM relay_accounts WHERE id=?").bind(result.user.id).first<{ terms_version: string | null; status: string }>();
  if (account && account.status !== "active") return null;
  const device = await env.DB.prepare("SELECT id FROM relay_devices WHERE session_id=? AND account_id=? AND revoked_at IS NULL").bind(result.session.id, result.user.id).first<{ id: string }>();
  return { accountId: result.user.id, sessionId: result.session.id, deviceId: device?.id ?? null, email: result.user.email, termsVersion: account?.terms_version ?? null };
}
const browserPosts = new Set(["/api/auth/sign-in/social", "/api/auth/device/approve", "/api/auth/device/deny"]);
const posts = new Set([...browserPosts, "/api/auth/sign-out", "/api/auth/device/code", "/api/auth/device/token"]);
const gets = new Set(["/api/auth/get-session", "/api/auth/device", "/api/auth/callback/github", "/api/auth/error"]);
export async function authRoute(request: Request, env: Env): Promise<Response> {
  if (!authConfigured(env)) return failure(503, "authentication_not_configured");
  const url = new URL(request.url);
  const origin = request.headers.get("origin");
  if (origin !== null && origin !== env.RELAY_ORIGIN) return failure(403, "origin_not_allowed");
  if ((request.method === "GET" && !gets.has(url.pathname)) || (request.method === "POST" && !posts.has(url.pathname)) || !["GET", "POST"].includes(request.method)) return failure(404, "not_found");
  if (browserPosts.has(url.pathname) && origin !== env.RELAY_ORIGIN) return failure(403, "origin_required");
  if (url.pathname === "/api/auth/sign-out" && !mutationAllowed(request, env.RELAY_ORIGIN)) return failure(403, "origin_required");
  const headers = new Headers(request.headers);
  // The auth framework must not retain raw IPs, arbitrary user agents or mixed credentials.
  headers.delete("cf-connecting-ip"); headers.delete("x-forwarded-for"); headers.delete("user-agent");
  if (headers.has("authorization")) {
    const credentials = credentialHeaders(headers);
    if (!credentials) return failure(401, "sign_in_required");
    headers.delete("cookie");
  }
  if (request.method === "GET") {
    const forwarded = new Request(request, { headers });
    if (url.pathname !== "/api/auth/callback/github") return createAuth(env).handler(forwarded);
    // Ask the maintained router to rethrow unexpected callback failures instead
    // of returning an empty 500 or logging provider/storage exception details.
    const options = authOptions(env, env.DB);
    try { return await betterAuth({ ...options, onAPIError: { ...options.onAPIError, throw: true } }).handler(forwarded); }
    catch { return authFailurePage(); }
  }
  const bytes = await boundedBytes(request);
  if (bytes === null) return failure(413, "body_too_large");
  let data: Record<string, unknown>;
  try {
    const text = new TextDecoder().decode(bytes);
    data = headers.get("content-type")?.includes("application/x-www-form-urlencoded")
      ? Object.fromEntries(new URLSearchParams(text)) : JSON.parse(text) as Record<string, unknown>;
    if (!data || typeof data !== "object" || Array.isArray(data)) return failure(400, "invalid_request");
  } catch { return failure(400, "invalid_request"); }
  if (url.pathname === "/api/auth/device/code" && (data.client_id !== "ejm-cli-relay" || (data.scope !== undefined && data.scope !== "relay") || !onlyKeys(data, ["client_id", "scope"]))) return failure(400, "invalid_device_request");
  if (url.pathname === "/api/auth/device/token" && (data.client_id !== "ejm-cli-relay" || !onlyKeys(data, ["client_id", "device_code", "grant_type"]))) return failure(400, "invalid_device_request");
  if (url.pathname === "/api/auth/sign-in/social") {
    if (data.provider !== "github" || (data.callbackURL !== undefined && data.callbackURL !== `${env.RELAY_ORIGIN}/account`) || !onlyKeys(data, ["provider", "callbackURL"])) return failure(400, "invalid_sign_in");
  }
  headers.set("content-type", "application/json");
  return createAuth(env).handler(new Request(request.url, { method: "POST", headers, body: JSON.stringify(data) }));
}
