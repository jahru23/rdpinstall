
-- F4: topup QRIS via PayGate. `amount` = nominal yang dikreditkan (base_amount),
-- `total_amount` = yang benar-benar dibayar pelanggan (base + fee gateway).
ALTER TABLE payments ADD COLUMN total_amount INTEGER NOT NULL DEFAULT 0;
ALTER TABLE payments ADD COLUMN txn_id TEXT NOT NULL DEFAULT '';
ALTER TABLE payments ADD COLUMN expires_at INTEGER NOT NULL DEFAULT 0;
CREATE UNIQUE INDEX IF NOT EXISTS idx_payments_txn ON payments(txn_id) WHERE txn_id <> '';
CREATE INDEX IF NOT EXISTS idx_payments_status ON payments(status);
