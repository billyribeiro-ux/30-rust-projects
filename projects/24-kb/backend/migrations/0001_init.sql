-- =========================================================================
-- Project 24 — Hybrid Search Knowledge Base: schema + FTS + pg_trgm
-- =========================================================================

CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email         TEXT NOT NULL UNIQUE CHECK (email = lower(email)),
    password_hash TEXT NOT NULL,
    name          TEXT NOT NULL DEFAULT '',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE sessions (
    id           UUID PRIMARY KEY,
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash   BYTEA NOT NULL UNIQUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at   TIMESTAMPTZ NOT NULL
);
CREATE INDEX idx_sessions_expires_at ON sessions (expires_at);

CREATE TABLE categories (
    slug       TEXT PRIMARY KEY CHECK (slug ~ '^[a-z0-9-]{1,80}$'),
    title      TEXT NOT NULL,
    parent_slug TEXT REFERENCES categories(slug)
);

CREATE TABLE articles (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug          TEXT NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9-]{1,120}$'),
    title         TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 200),
    summary       TEXT NOT NULL DEFAULT '',
    body_md       TEXT NOT NULL,
    body_html     TEXT NOT NULL,
    category_slug TEXT REFERENCES categories(slug),
    locale        TEXT NOT NULL DEFAULT 'en',
    published_at  TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- The `tsv` column is the headline FTS column. Updated by trigger so
    -- the app can never forget. Weighted: title (A) > summary (B) > body (C).
    tsv           tsvector
);

-- GIN index on tsv → BM25 search.
CREATE INDEX idx_articles_tsv ON articles USING GIN (tsv);
-- pg_trgm index on title for typo-tolerant fuzzy match.
CREATE INDEX idx_articles_title_trgm ON articles USING GIN (title gin_trgm_ops);
CREATE INDEX idx_articles_published ON articles (published_at DESC NULLS LAST);

CREATE OR REPLACE FUNCTION articles_tsv_update() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.tsv :=
        setweight(to_tsvector('english', coalesce(NEW.title, '')),    'A') ||
        setweight(to_tsvector('english', coalesce(NEW.summary, '')),  'B') ||
        setweight(to_tsvector('english', coalesce(NEW.body_md, '')),  'C');
    RETURN NEW;
END
$$;

CREATE TRIGGER trg_articles_tsv
BEFORE INSERT OR UPDATE OF title, summary, body_md ON articles
FOR EACH ROW EXECUTE FUNCTION articles_tsv_update();

CREATE TABLE faqs (
    article_id UUID NOT NULL REFERENCES articles(id) ON DELETE CASCADE,
    position   INT  NOT NULL,
    q          TEXT NOT NULL,
    a          TEXT NOT NULL,
    PRIMARY KEY (article_id, position)
);
