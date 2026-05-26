CREATE TABLE IF NOT EXISTS members (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    color       TEXT NOT NULL,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS expenses (
    id            TEXT PRIMARY KEY,
    payer_id      TEXT NOT NULL,
    amount_cents  INTEGER NOT NULL CHECK (amount_cents > 0),
    description   TEXT NOT NULL DEFAULT '',
    paid_at       TEXT NOT NULL,
    split_kind    TEXT NOT NULL CHECK (split_kind IN ('equal', 'exact', 'percent')),
    created_at    TEXT NOT NULL,
    FOREIGN KEY (payer_id) REFERENCES members(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS expense_shares (
    expense_id   TEXT NOT NULL,
    member_id    TEXT NOT NULL,
    share_cents  INTEGER NOT NULL CHECK (share_cents >= 0),
    PRIMARY KEY (expense_id, member_id),
    FOREIGN KEY (expense_id) REFERENCES expenses(id) ON DELETE CASCADE,
    FOREIGN KEY (member_id)  REFERENCES members(id)  ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_expenses_paid_at ON expenses (paid_at DESC);
CREATE INDEX IF NOT EXISTS idx_shares_member ON expense_shares (member_id);
