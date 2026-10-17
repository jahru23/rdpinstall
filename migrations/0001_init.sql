
-- users / sessions
CREATE TABLE IF NOT EXISTS users (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  tg_id        INTEGER UNIQUE,
  username     TEXT NOT NULL DEFAULT '',
  first_name   TEXT NOT NULL DEFAULT '',
  last_name    TEXT NOT NULL DEFAULT '',
  photo_url    TEXT NOT NULL DEFAULT '',
  banned       INTEGER NOT NULL DEFAULT 0,
  created_at   INTEGER NOT NULL,
  last_seen_at INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS sessions (
  token      TEXT PRIMARY KEY,
  user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  created_at INTEGER NOT NULL,
  expires_at INTEGER NOT NULL,
  ip         TEXT NOT NULL DEFAULT '',
  ua         TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);

-- balance + ledger (IDR, integer rupiah)
CREATE TABLE IF NOT EXISTS balances (
  user_id INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  amount  INTEGER NOT NULL DEFAULT 0,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS ledger (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  delta      INTEGER NOT NULL,
  kind       TEXT NOT NULL,              -- topup | charge | refund | adjust
  ref        TEXT NOT NULL DEFAULT '',   -- payment ref / job id
  note       TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_ledger_user ON ledger(user_id, created_at);

CREATE TABLE IF NOT EXISTS payments (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  gateway      TEXT NOT NULL,            -- paygate | cryptogate
  ref          TEXT NOT NULL,            -- our reference id sent to gateway
  gateway_id   TEXT NOT NULL DEFAULT '',
  amount       INTEGER NOT NULL,
  status       TEXT NOT NULL DEFAULT 'pending',  -- pending | paid | expired | failed
  qr_string    TEXT NOT NULL DEFAULT '',
  pay_url      TEXT NOT NULL DEFAULT '',
  raw_json     TEXT NOT NULL DEFAULT '{}',
  created_at   INTEGER NOT NULL,
  paid_at      INTEGER NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_payments_ref ON payments(ref);
CREATE INDEX IF NOT EXISTS idx_payments_user ON payments(user_id, created_at);

-- cloud provider accounts (user-owned API tokens, encrypted at rest)
CREATE TABLE IF NOT EXISTS cloud_accounts (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  user_id       INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  provider      TEXT NOT NULL,           -- upcloud
  label         TEXT NOT NULL DEFAULT '',
  token_enc     TEXT NOT NULL,
  status        TEXT NOT NULL DEFAULT 'unchecked',
  credit        TEXT NOT NULL DEFAULT '',
  last_checked  INTEGER NOT NULL DEFAULT 0,
  last_error    TEXT NOT NULL DEFAULT '',
  created_at    INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_cloud_accounts_user ON cloud_accounts(user_id);

-- jobs
CREATE TABLE IF NOT EXISTS jobs (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  user_id       INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  mode          TEXT NOT NULL,           -- template | reinstall
  provider      TEXT NOT NULL DEFAULT 'upcloud',
  status        TEXT NOT NULL,           -- queued | running | done | failed | cancelled
  phase         TEXT NOT NULL DEFAULT '',
  progress      INTEGER NOT NULL DEFAULT 0,
  os_label      TEXT NOT NULL DEFAULT '',
  zone          TEXT NOT NULL DEFAULT '',
  plan          TEXT NOT NULL DEFAULT '',
  host          TEXT NOT NULL DEFAULT '',
  rdp_port      INTEGER NOT NULL DEFAULT 3389,
  rdp_user      TEXT NOT NULL DEFAULT 'Administrator',
  rdp_pass_enc  TEXT NOT NULL DEFAULT '',
  provider_ref  TEXT NOT NULL DEFAULT '',  -- server uuid
  storage_ref   TEXT NOT NULL DEFAULT '',
  error         TEXT NOT NULL DEFAULT '',
  created_at    INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL,
  done_at       INTEGER NOT NULL DEFAULT 0,
  expires_at    INTEGER NOT NULL DEFAULT 0,
  wiped         INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_jobs_user ON jobs(user_id, id DESC);
CREATE INDEX IF NOT EXISTS idx_jobs_active ON jobs(status);

CREATE TABLE IF NOT EXISTS job_events (
  id      INTEGER PRIMARY KEY AUTOINCREMENT,
  job_id  INTEGER NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
  ts      INTEGER NOT NULL,
  phase   TEXT NOT NULL DEFAULT '',
  message TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_job_events ON job_events(job_id, id);

CREATE TABLE IF NOT EXISTS audit_log (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  user_id    INTEGER NOT NULL DEFAULT 0,
  action     TEXT NOT NULL,
  detail     TEXT NOT NULL DEFAULT '',
  ip         TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL
);
