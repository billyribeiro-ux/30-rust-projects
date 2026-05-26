CREATE TABLE IF NOT EXISTS books (
    id            TEXT PRIMARY KEY,
    isbn          TEXT,
    title         TEXT NOT NULL,
    author        TEXT NOT NULL DEFAULT '',
    cover_url     TEXT,
    pages         INTEGER,
    status        TEXT NOT NULL DEFAULT 'want_to_read'
                  CHECK (status IN ('want_to_read', 'reading', 'finished')),
    current_page  INTEGER NOT NULL DEFAULT 0 CHECK (current_page >= 0),
    started_at    TEXT,
    finished_at   TEXT,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS reading_sessions (
    id              TEXT PRIMARY KEY,
    book_id         TEXT NOT NULL,
    pages_read      INTEGER NOT NULL CHECK (pages_read > 0),
    duration_minutes INTEGER NOT NULL CHECK (duration_minutes > 0),
    occurred_at     TEXT NOT NULL,
    FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS highlights (
    id           TEXT PRIMARY KEY,
    book_id      TEXT NOT NULL,
    quote        TEXT NOT NULL,
    note         TEXT NOT NULL DEFAULT '',
    page         INTEGER,
    created_at   TEXT NOT NULL,
    FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_books_status ON books (status);
CREATE INDEX IF NOT EXISTS idx_books_updated_at ON books (updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_sessions_book ON reading_sessions (book_id, occurred_at DESC);
CREATE INDEX IF NOT EXISTS idx_highlights_book ON highlights (book_id, created_at DESC);
