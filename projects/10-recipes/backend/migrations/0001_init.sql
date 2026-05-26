CREATE TABLE IF NOT EXISTS recipes (
    id                TEXT PRIMARY KEY,
    slug              TEXT NOT NULL UNIQUE,
    title             TEXT NOT NULL,
    description       TEXT NOT NULL DEFAULT '',
    ingredients_json  TEXT NOT NULL DEFAULT '[]',
    instructions_json TEXT NOT NULL DEFAULT '[]',
    prep_minutes      INTEGER,
    cook_minutes      INTEGER,
    servings          INTEGER,
    cover_image_id    TEXT,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL,
    FOREIGN KEY (cover_image_id) REFERENCES recipe_images(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS recipe_images (
    id          TEXT PRIMARY KEY,
    recipe_id   TEXT NOT NULL,
    mime_type   TEXT NOT NULL,
    width       INTEGER NOT NULL,
    height      INTEGER NOT NULL,
    bytes       INTEGER NOT NULL,
    path        TEXT NOT NULL,
    thumb_path  TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    FOREIGN KEY (recipe_id) REFERENCES recipes(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS ratings (
    id          TEXT PRIMARY KEY,
    recipe_id   TEXT NOT NULL,
    stars       INTEGER NOT NULL CHECK (stars BETWEEN 1 AND 5),
    comment     TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL,
    FOREIGN KEY (recipe_id) REFERENCES recipes(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_recipes_updated_at ON recipes (updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_recipe_images_recipe ON recipe_images (recipe_id);
CREATE INDEX IF NOT EXISTS idx_ratings_recipe ON ratings (recipe_id, created_at DESC);
