export type DiagnosticCode = "account_unavailable" | "quota_unavailable" | "invalid_request" | "billing_unavailable" | "cleanup_complete";
export function diagnostic(code: DiagnosticCode, status: number, count?: number): void {
  // Never pass a Request, an exception, an identifier, or a transport buffer here.
  console.info(JSON.stringify({ code, status, ...(count === undefined ? {} : { count: Math.max(0, Math.min(count, 10000)) }) }));
}
