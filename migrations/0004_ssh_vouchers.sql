
-- Mode SSH (root + password) menggantikan mode template UpCloud.
ALTER TABLE jobs ADD COLUMN ssh_port INTEGER NOT NULL DEFAULT 22;
ALTER TABLE jobs ADD COLUMN ssh_user TEXT NOT NULL DEFAULT 'root';
ALTER TABLE jobs ADD COLUMN ssh_pass_enc TEXT NOT NULL DEFAULT '';
ALTER TABLE jobs ADD COLUMN access_token TEXT NOT NULL DEFAULT '';
ALTER TABLE jobs ADD COLUMN guest INTEGER NOT NULL DEFAULT 0;
ALTER TABLE jobs ADD COLUMN voucher_code TEXT NOT NULL DEFAULT '';
ALTER TABLE jobs ADD COLUMN edition TEXT NOT NULL DEFAULT '';
ALTER TABLE jobs ADD COLUMN lang TEXT NOT NULL DEFAULT 'en-us';
CREATE UNIQUE INDEX IF NOT EXISTS idx_jobs_access_token ON jobs(access_token) WHERE access_token <> '';

-- Voucer: sekali pakai, satu instalasi, tanpa sisa saldo.
CREATE TABLE IF NOT EXISTS vouchers (
  code         TEXT PRIMARY KEY,
  created_by   INTEGER NOT NULL DEFAULT 0,
  created_at   INTEGER NOT NULL,
  used_at      INTEGER NOT NULL DEFAULT 0,
  used_job_id  INTEGER NOT NULL DEFAULT 0,
  used_host    TEXT NOT NULL DEFAULT '',
  revoked      INTEGER NOT NULL DEFAULT 0,
  note         TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_vouchers_used ON vouchers(used_at, revoked);

-- Baris penampung untuk job tamu (tanpa akun). FK jobs.user_id tetap terjaga.
INSERT OR IGNORE INTO users(id, tg_id, username, first_name, last_name, photo_url, banned, created_at, last_seen_at)
VALUES(0, NULL, '', 'Tamu', '', '', 0, 0, 0);
