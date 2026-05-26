CREATE TABLE IF NOT EXISTS exercises (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL UNIQUE,
    muscle_group  TEXT NOT NULL,
    created_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS workouts (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL DEFAULT '',
    performed_at  TEXT NOT NULL,
    created_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sets (
    id            TEXT PRIMARY KEY,
    workout_id    TEXT NOT NULL,
    exercise_id   TEXT NOT NULL,
    weight_minor  INTEGER NOT NULL CHECK (weight_minor > 0),
    reps          INTEGER NOT NULL CHECK (reps > 0),
    rir           INTEGER NOT NULL DEFAULT 0 CHECK (rir BETWEEN 0 AND 10),
    set_order     INTEGER NOT NULL,
    FOREIGN KEY (workout_id) REFERENCES workouts(id) ON DELETE CASCADE,
    FOREIGN KEY (exercise_id) REFERENCES exercises(id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_sets_workout ON sets (workout_id);
CREATE INDEX IF NOT EXISTS idx_sets_exercise ON sets (exercise_id);
CREATE INDEX IF NOT EXISTS idx_workouts_performed_at ON workouts (performed_at DESC);

-- FTS5 virtual table mirrors exercises.name. content='exercises' makes it
-- an "external content" table — the FTS index stores only the tokenized
-- name; the canonical row lives in `exercises`. We drive it with triggers
-- so inserts/updates/deletes to `exercises` keep the index in sync.
CREATE VIRTUAL TABLE IF NOT EXISTS exercises_fts USING fts5(
    name,
    content='exercises',
    content_rowid='rowid'
);

CREATE TRIGGER IF NOT EXISTS exercises_ai AFTER INSERT ON exercises BEGIN
    INSERT INTO exercises_fts(rowid, name) VALUES (new.rowid, new.name);
END;

CREATE TRIGGER IF NOT EXISTS exercises_ad AFTER DELETE ON exercises BEGIN
    INSERT INTO exercises_fts(exercises_fts, rowid, name) VALUES('delete', old.rowid, old.name);
END;

CREATE TRIGGER IF NOT EXISTS exercises_au AFTER UPDATE ON exercises BEGIN
    INSERT INTO exercises_fts(exercises_fts, rowid, name) VALUES('delete', old.rowid, old.name);
    INSERT INTO exercises_fts(rowid, name) VALUES (new.rowid, new.name);
END;
