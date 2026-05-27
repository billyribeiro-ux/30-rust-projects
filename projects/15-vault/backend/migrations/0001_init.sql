-- =========================================================================
-- Project 15 - File Vault: schema
-- =========================================================================
-- Auth tables (users / sessions) reuse the project 14 shape, minus
-- auth_tokens (no email verification in vault demo to keep scope tight).
--
-- Domain:
--   folders        - a folder tree per owner (parent_id NULL = root).
--   file_versions  - content-addressed blob catalogue (one row per unique
--                    SHA-256). Dedup happens here: two files with the
--                    same bytes share a single file_versions row.
--   files          - a named file inside a folder, pointing at a version.
--   upload_sessions- tus-style resumable uploads. Holds the temp path and
--                    the running offset. Reaped after expires_at.
-- =========================================================================

CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email         TEXT NOT NULL UNIQUE CHECK (email = lower(email)),
    password_hash TEXT NOT NULL,
    name          TEXT NOT NULL DEFAULT '',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE sessions (
    id           UUID PRIMARY KEY,
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash   BYTEA NOT NULL UNIQUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at   TIMESTAMPTZ NOT NULL,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ip           TEXT,
    user_agent   TEXT
);
CREATE INDEX idx_sessions_user_id    ON sessions (user_id);
CREATE INDEX idx_sessions_expires_at ON sessions (expires_at);

-- ---------- Domain ----------

CREATE TABLE folders (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_id  UUID REFERENCES folders(id) ON DELETE CASCADE,
    owner_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name       TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 200),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, parent_id, name)
);
CREATE INDEX idx_folders_owner_parent ON folders (owner_id, parent_id);

CREATE TABLE file_versions (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sha256      TEXT NOT NULL UNIQUE CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    size        BIGINT NOT NULL CHECK (size >= 0),
    mime        TEXT NOT NULL DEFAULT 'application/octet-stream',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_file_versions_sha ON file_versions (sha256);

CREATE TABLE files (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    folder_id  UUID REFERENCES folders(id) ON DELETE CASCADE,
    owner_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name       TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 255),
    version_id UUID NOT NULL REFERENCES file_versions(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_files_owner_folder ON files (owner_id, folder_id);
CREATE INDEX idx_files_version ON files (version_id);

CREATE TABLE upload_sessions (
    id            UUID PRIMARY KEY,
    owner_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    folder_id     UUID REFERENCES folders(id) ON DELETE CASCADE,
    filename      TEXT NOT NULL,
    content_type  TEXT NOT NULL DEFAULT 'application/octet-stream',
    total_size    BIGINT NOT NULL CHECK (total_size >= 0),
    received      BIGINT NOT NULL DEFAULT 0 CHECK (received >= 0),
    temp_path     TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at    TIMESTAMPTZ NOT NULL
);
CREATE INDEX idx_upload_sessions_owner ON upload_sessions (owner_id);
CREATE INDEX idx_upload_sessions_expires ON upload_sessions (expires_at);

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS TRIGGER AS $$
BEGIN NEW.updated_at = now(); RETURN NEW; END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER trg_folders_updated_at
    BEFORE UPDATE ON folders
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER trg_files_updated_at
    BEFORE UPDATE ON files
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
