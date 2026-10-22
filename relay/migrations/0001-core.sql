-- Operational identity, access and payment state. Never a commercial export source.
CREATE TABLE relay_accounts (id TEXT PRIMARY KEY REFERENCES "user"(id),status TEXT NOT NULL CHECK(status IN ('active','suspended','closed')),terms_version TEXT,terms_at INTEGER,email_provenance TEXT NOT NULL,created_at INTEGER NOT NULL,closed_at INTEGER,cleanup_after INTEGER,quota_cleanup_at INTEGER);
CREATE INDEX relay_accounts_status ON relay_accounts(status);
CREATE TABLE relay_devices (id TEXT PRIMARY KEY,account_id TEXT NOT NULL REFERENCES relay_accounts(id),session_id TEXT NOT NULL UNIQUE,label TEXT NOT NULL,created_at INTEGER NOT NULL,last_used_at INTEGER NOT NULL,revoked_at INTEGER,os_family TEXT,cli_major INTEGER);
CREATE INDEX relay_devices_account ON relay_devices(account_id,created_at);
CREATE INDEX relay_devices_revoked ON relay_devices(revoked_at);
CREATE TABLE connection_tickets (hash TEXT PRIMARY KEY,account_id TEXT NOT NULL,room TEXT NOT NULL,role TEXT NOT NULL,expires_at INTEGER NOT NULL);
CREATE INDEX connection_tickets_expiry ON connection_tickets(expires_at);
CREATE TABLE relay_billing (account_id TEXT PRIMARY KEY REFERENCES relay_accounts(id),customer_id TEXT NOT NULL UNIQUE,subscription_id TEXT,plan TEXT NOT NULL,status TEXT NOT NULL,paid_until INTEGER NOT NULL,cancel_at_period_end INTEGER NOT NULL,updated_at INTEGER NOT NULL,checkout_key TEXT,checkout_session_id TEXT,checkout_plan TEXT,checkout_expires_at INTEGER);
CREATE TABLE relay_billing_events (id TEXT PRIMARY KEY,account_id TEXT NOT NULL,status TEXT NOT NULL,created_at INTEGER NOT NULL,processed_at INTEGER,expires_at INTEGER NOT NULL);
CREATE INDEX relay_billing_events_expiry ON relay_billing_events(expires_at);
CREATE TABLE relay_payment_holds (account_id TEXT NOT NULL,reference_id TEXT PRIMARY KEY,invoice_id TEXT,kind TEXT NOT NULL,active INTEGER NOT NULL,updated_at INTEGER NOT NULL);
CREATE INDEX relay_payment_holds_account ON relay_payment_holds(account_id,active);

CREATE TABLE operational_legal_holds(id TEXT PRIMARY KEY,account_id TEXT NOT NULL,category TEXT NOT NULL,reason TEXT NOT NULL,evidence TEXT NOT NULL,expires_at INTEGER NOT NULL);
CREATE INDEX operational_legal_holds_expiry ON operational_legal_holds(expires_at);
CREATE INDEX operational_legal_holds_subject ON operational_legal_holds(account_id,expires_at);
CREATE INDEX session_expiry_idx ON "session"("expiresAt");
CREATE INDEX verification_expiry_idx ON "verification"("expiresAt");
CREATE INDEX deviceCode_expiry_idx ON "deviceCode"("expiresAt");
CREATE INDEX rateLimit_expiry_idx ON "rateLimit"("lastRequest");

CREATE INDEX relay_accounts_cleanup ON relay_accounts(status,cleanup_after);
CREATE TABLE relay_billing_cancellations(account_id TEXT PRIMARY KEY,state TEXT NOT NULL,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL);
CREATE INDEX relay_billing_cancellations_pending ON relay_billing_cancellations(state,created_at);

CREATE INDEX user_created_idx ON "user"("createdAt");
CREATE INDEX relay_accounts_quota_cleanup ON relay_accounts(quota_cleanup_at,created_at);
