-- Timestamps stay RFC 3339 TEXT, as in SQLite: the sqlx Any driver used by the
-- repository has no date types.
CREATE TABLE news (
    id       BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    topic    TEXT   NOT NULL,
    used_at  TEXT   NOT NULL
);

CREATE TABLE stats (
    id          BIGINT PRIMARY KEY CHECK (id = 1),
    microsoft   BIGINT NOT NULL DEFAULT 0,
    quoi_feur   BIGINT NOT NULL DEFAULT 0
);

INSERT INTO stats (id, microsoft, quoi_feur) VALUES (1, 0, 0);

-- NULLS NOT DISTINCT: a roast without target increments one row per
-- (triggerer, type) instead of inserting a new one each time.
CREATE TABLE roast_stats (
    id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    triggerer_id TEXT   NOT NULL,
    target_id    TEXT,
    roast_type   TEXT   NOT NULL,
    count        BIGINT NOT NULL DEFAULT 1,
    UNIQUE NULLS NOT DISTINCT (triggerer_id, target_id, roast_type)
);

CREATE TABLE image_descriptions (
    canonical_key TEXT PRIMARY KEY,
    source_url    TEXT NOT NULL,
    description   TEXT NOT NULL,
    created_at    TEXT NOT NULL
);
