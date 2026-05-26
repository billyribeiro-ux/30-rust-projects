-- A small double-entry accounting schema.
-- Every transaction has 2+ postings; the sum of postings.amount_minor
-- across a single transaction MUST equal zero (validated in code).
-- We store amounts as INTEGER minor units (cents). The frontend can
-- display in any currency the user picks per account.

CREATE TABLE IF NOT EXISTS accounts (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL UNIQUE,
    kind         TEXT NOT NULL CHECK (kind IN ('asset', 'liability', 'income', 'expense', 'equity')),
    color        TEXT NOT NULL,
    created_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS transactions (
    id           TEXT PRIMARY KEY,
    description  TEXT NOT NULL DEFAULT '',
    occurred_at  TEXT NOT NULL,
    created_at   TEXT NOT NULL
);

-- A posting is one entry in the double-entry ledger.
-- Positive amount_minor = debit. Negative = credit.
-- Sum of amount_minor across all postings of a transaction == 0.
CREATE TABLE IF NOT EXISTS postings (
    id             TEXT PRIMARY KEY,
    transaction_id TEXT NOT NULL,
    account_id     TEXT NOT NULL,
    amount_minor   INTEGER NOT NULL,
    FOREIGN KEY (transaction_id) REFERENCES transactions(id) ON DELETE CASCADE,
    FOREIGN KEY (account_id)     REFERENCES accounts(id)     ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_postings_tx ON postings (transaction_id);
CREATE INDEX IF NOT EXISTS idx_postings_account ON postings (account_id);
CREATE INDEX IF NOT EXISTS idx_transactions_occurred ON transactions (occurred_at DESC);
