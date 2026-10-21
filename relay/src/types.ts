export interface Env {
  DB: D1Database;
  ASSETS?: Fetcher;
  ROOM: DurableObjectNamespace;
  ACCOUNT_QUOTA: DurableObjectNamespace;
  PRIVACY_AUTHORITY: DurableObjectNamespace;
  RELAY_ORIGIN: string;
  BETTER_AUTH_SECRET?: string;
  GITHUB_CLIENT_ID?: string;
  GITHUB_CLIENT_SECRET?: string;
  TERMS_VERSION: string;
  RETENTION_CONFIG?: string;
  ADMISSION_LIMIT?: string;
  ADMISSION_ENABLED?: string;
  STRIPE_SECRET_KEY?: string;
  STRIPE_WEBHOOK_SECRET?: string;
  STRIPE_MONTHLY_PRICE?: string;
  STRIPE_YEARLY_PRICE?: string;
  PAID_DAILY_FRAMES?: string;
  PAID_DAILY_BYTES?: string;
  COMMERCIAL_CONFIG?: string;
  PRIVACY_OPERATOR_ACCOUNT_IDS?: string;
  PRIVACY_COOKIE_SECURE?: string;
}

export interface AccountSession {
  accountId: string;
  sessionId: string;
  deviceId: string | null;
  email: string;
  termsVersion: string | null;
}

export type PeerRole = "host" | "controller";
export interface Lease {
  accountId: string;
  deviceId: string;
  sessionId: string;
  room: string;
  role: PeerRole;
  expiresAt: number;
  frames: number;
  bytes: number;
}
export interface TicketScope {
  accountId: string;
  deviceId: string;
  sessionId: string;
  room: string;
  role: PeerRole;
  expiresAt: number;
}
export const LIMITS = Object.freeze({
  accounts: 30, dailyFrames: 50_000, dailyBytes: 64 * 1024 * 1024,
  blockFrames: 512, blockBytes: 1024 * 1024, frameBytes: 16 * 1024,
  burstFrames: 100, dailyControls: 300, leaseMs: 15 * 60 * 1000, ticketMs: 60 * 1000,
  shareMs: 8 * 60 * 60 * 1000, maxBody: 16 * 1024,
});
