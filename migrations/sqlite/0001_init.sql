-- Same schema the bot created at startup before sqlx: IF NOT EXISTS lets an
-- existing memory.db adopt the migration history without changes.
CREATE TABLE IF NOT EXISTS news (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    topic    TEXT    NOT NULL,
    used_at  TEXT    NOT NULL
);

CREATE TABLE IF NOT EXISTS stats (
    id          INTEGER PRIMARY KEY CHECK (id = 1),
    microsoft   INTEGER NOT NULL DEFAULT 0,
    quoi_feur   INTEGER NOT NULL DEFAULT 0
);

INSERT OR IGNORE INTO stats (id, microsoft, quoi_feur) VALUES (1, 0, 0);

CREATE TABLE IF NOT EXISTS roast_stats (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    triggerer_id TEXT NOT NULL,
    target_id    TEXT,
    roast_type   TEXT NOT NULL,
    count        INTEGER NOT NULL DEFAULT 1,
    UNIQUE(triggerer_id, target_id, roast_type)
);

CREATE TABLE IF NOT EXISTS image_descriptions (
    canonical_key TEXT PRIMARY KEY,
    source_url    TEXT NOT NULL,
    description   TEXT NOT NULL,
    created_at    TEXT NOT NULL
);
