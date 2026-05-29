-- =========================================================================
-- Project 20 - Geo-aware Restaurant Finder: schema
-- =========================================================================
-- New here vs project 15:
--   * postgis extension + GEOGRAPHY(Point,4326) column on places
--   * GiST index on places.geom so `ST_DWithin` is fast
--   * oauth_accounts: links a user to one or more OAuth provider IDs.
--     A user may have NULL password_hash (signed up via OAuth only).
--   * reviews + friend_recs domain tables.
-- =========================================================================

CREATE EXTENSION IF NOT EXISTS postgis;

CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email         TEXT NOT NULL UNIQUE CHECK (email = lower(email)),
    -- NULL allowed: OAuth-only accounts have no local password.
    password_hash TEXT,
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

-- A given (provider, provider_user_id) maps to exactly one local user.
-- A given user can have at most one row per provider (UNIQUE on user_id+provider).
CREATE TABLE oauth_accounts (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id          UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider         TEXT NOT NULL CHECK (provider IN ('google','github')),
    provider_user_id TEXT NOT NULL,
    email            TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (provider, provider_user_id),
    UNIQUE (user_id, provider)
);
CREATE INDEX idx_oauth_accounts_user ON oauth_accounts (user_id);

-- ---------- Domain ----------

CREATE TABLE places (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name       TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 200),
    cuisine    TEXT NOT NULL DEFAULT '' CHECK (length(cuisine) <= 80),
    address    TEXT NOT NULL DEFAULT '',
    -- GEOGRAPHY stores lon/lat (in that order!) on a sphere; ST_Distance
    -- on geography returns metres, no projection needed. SRID 4326 is WGS84.
    geom       GEOGRAPHY(Point, 4326) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- GiST is the index that makes `ST_DWithin(geom, X, R)` fast. Without it,
-- a "near me" search has to scan every row.
CREATE INDEX idx_places_geom ON places USING GIST (geom);
CREATE INDEX idx_places_cuisine ON places (cuisine);

CREATE TABLE reviews (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    place_id   UUID NOT NULL REFERENCES places(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rating     SMALLINT NOT NULL CHECK (rating BETWEEN 1 AND 5),
    body       TEXT NOT NULL DEFAULT '' CHECK (length(body) <= 2000),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (place_id, user_id)
);
CREATE INDEX idx_reviews_place ON reviews (place_id);
CREATE INDEX idx_reviews_user  ON reviews (user_id);

-- "My friend Alex recommends Acme Diner". friend_id is also a user; the
-- semantics are user-asserted (no follow graph) — fine for the lesson.
CREATE TABLE friend_recs (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    friend_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    place_id   UUID NOT NULL REFERENCES places(id) ON DELETE CASCADE,
    note       TEXT NOT NULL DEFAULT '' CHECK (length(note) <= 500),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, friend_id, place_id)
);
CREATE INDEX idx_friend_recs_user ON friend_recs (user_id);
CREATE INDEX idx_friend_recs_place ON friend_recs (place_id);

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS TRIGGER AS $$
BEGIN NEW.updated_at = now(); RETURN NEW; END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER trg_places_updated_at
    BEFORE UPDATE ON places
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Seed a handful of places near a known coordinate so the "near me"
-- demo works without manual data entry. Lon, Lat order!
-- These are around the centre of London (51.5074N, -0.1278W). Coordinates
-- nudged slightly for separation so distance ordering is stable.
INSERT INTO places (name, cuisine, address, geom) VALUES
  ('Acme Diner',    'american', '1 Strand, London',          ST_SetSRID(ST_MakePoint(-0.1278, 51.5074), 4326)::geography),
  ('Bella Pasta',   'italian',  '12 Covent Garden, London',  ST_SetSRID(ST_MakePoint(-0.1240, 51.5118), 4326)::geography),
  ('Sushi Ten',     'japanese', '99 Soho Square, London',    ST_SetSRID(ST_MakePoint(-0.1320, 51.5150), 4326)::geography),
  ('Curry Bowl',    'indian',   '7 Brick Lane, London',      ST_SetSRID(ST_MakePoint(-0.0720, 51.5210), 4326)::geography),
  ('Taco Casa',     'mexican',  '3 Shoreditch, London',      ST_SetSRID(ST_MakePoint(-0.0780, 51.5260), 4326)::geography);
