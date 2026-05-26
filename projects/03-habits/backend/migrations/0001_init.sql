CREATE TABLE IF NOT EXISTS habits (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    color       TEXT NOT NULL,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS habit_completions (
    habit_id        TEXT NOT NULL,
    completion_date TEXT NOT NULL,
    PRIMARY KEY (habit_id, completion_date),
    FOREIGN KEY (habit_id) REFERENCES habits(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_completions_date ON habit_completions (completion_date);
