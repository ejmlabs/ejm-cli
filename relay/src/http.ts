import { LIMITS } from "./types.js";

export function failure(status: number, code: string): Response {
  return Response.json({ error: { code } }, { status });
}
export function integer(value: string | undefined, fallback: number, maximum = Number.MAX_SAFE_INTEGER): number {
  if (value === undefined) return fallback;
  return /^\d+$/.test(value) ? Math.min(Number(value), maximum) : 0;
}
export function validId(value: unknown): value is string {
  return typeof value === "string" && /^[a-zA-Z0-9_-]{16,128}$/.test(value);
}
export function credentialHeaders(headers: Headers): Headers | null {
  const result = new Headers();
  const authorization = headers.get("authorization");
  if (authorization !== null) {
    if (!/^Bearer [!-~]{1,4096}$/.test(authorization)) return null;
    result.set("authorization", authorization);
  } else {
    const cookie = headers.get("cookie");
    if (!cookie || cookie.length > 8192) return null;
    result.set("cookie", cookie);
  }
  return result;
}
export function mutationAllowed(request: Request, origin: string): boolean {
  const supplied = request.headers.get("origin");
  if (supplied !== null) return supplied === origin;
  return request.headers.has("authorization") && credentialHeaders(request.headers) !== null;
}
export async function boundedBytes(request: Pick<Request, "headers" | "body">, maximum = LIMITS.maxBody): Promise<Uint8Array | null> {
  const length = request.headers.get("content-length");
  if (length !== null && (!/^\d+$/.test(length) || Number(length) > maximum)) return null;
  if (!request.body) return new Uint8Array();
  const reader = request.body.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  for (;;) {
    const result = await reader.read();
    if (result.done) break;
    size += result.value.byteLength;
    if (size > maximum) { await reader.cancel(); return null; }
    chunks.push(result.value);
  }
  const bytes = new Uint8Array(size);
  let offset = 0;
  for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.byteLength; }
  return bytes;
}
export async function jsonObject(request: Request): Promise<Record<string, unknown> | null> {
  const bytes = await boundedBytes(request);
  if (bytes === null) return null;
  try {
    const data: unknown = JSON.parse(new TextDecoder().decode(bytes));
    return data !== null && typeof data === "object" && !Array.isArray(data)
      ? data as Record<string, unknown> : null;
  } catch { return null; }
}
export function onlyKeys(data: Record<string, unknown>, allowed: string[]): boolean {
  return Object.keys(data).every(key => allowed.includes(key));
}
export function randomToken(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(32));
  return btoa(String.fromCharCode(...bytes)).replaceAll("+", "-").replaceAll("/", "_").replaceAll("=", "");
}
export async function hashToken(value: string): Promise<string> {
  const hash = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
  return Array.from(new Uint8Array(hash), byte => byte.toString(16).padStart(2, "0")).join("");
}
export function secure(response: Response, origin?: string, browserControl = false): Response {
  if (response.status === 101) return response;
  const result = new Response(response.body, response);
  result.headers.set("cache-control", "no-store");
  result.headers.set("referrer-policy", "same-origin");
  result.headers.set("x-content-type-options", "nosniff");
  const websocket = origin?.startsWith("https://") ? origin.replace("https://", "wss://") : "";
  result.headers.set("content-security-policy", `default-src 'self'; script-src 'self'${browserControl ? " 'wasm-unsafe-eval'" : ""}; connect-src 'self' ${websocket}; style-src 'self'; object-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'`);
  return result;
}
export function htmlEscape(value: string): string {
  return value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;").replaceAll("'", "&#39;");
}
