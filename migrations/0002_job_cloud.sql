
-- F3: job dibuat dari akun cloud (mode template), bukan dari VPS existing.
ALTER TABLE jobs ADD COLUMN cloud_account_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE jobs ADD COLUMN storage_size INTEGER NOT NULL DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_jobs_provider_ref ON jobs(provider_ref);
