-- Privacy state and the release ledger live in the single SQLite PrivacyAuthority.
-- This queue coordinates rights requests with the separate operational D1 store.
CREATE TABLE privacy_operational_tasks (
  request_id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  action TEXT NOT NULL CHECK (action IN ('access','export','correction','delete','close')),
  state TEXT NOT NULL DEFAULT 'pending' CHECK (state IN ('pending','completed','exception')),
  requested_at INTEGER NOT NULL,
  due_at INTEGER NOT NULL,
  expires_at INTEGER NOT NULL,
  result TEXT
);
CREATE INDEX privacy_operational_task_state_due ON privacy_operational_tasks(state,due_at);
CREATE INDEX privacy_operational_task_expiry ON privacy_operational_tasks(expires_at);
