-- Matches between two playing-engine configurations, to measure strength.
-- Each opening is played twice, once with each engine as X.
CREATE TABLE matches (
    id          INTEGER PRIMARY KEY,
    variant     TEXT NOT NULL,
    engine_a    TEXT NOT NULL,
    engine_b    TEXT NOT NULL,
    node_limit  INTEGER NOT NULL,
    git_rev     TEXT NOT NULL,
    notes       TEXT NOT NULL DEFAULT '',
    started_at  TEXT NOT NULL DEFAULT (datetime('now')),
    finished_at TEXT
);

-- `score` is the result for engine A: 1 win, 0 draw, -1 loss.
CREATE TABLE match_games (
    match_id INTEGER NOT NULL REFERENCES matches(id),
    idx      INTEGER NOT NULL,
    opening  TEXT NOT NULL,
    a_is_x   INTEGER NOT NULL,
    score    INTEGER NOT NULL,
    plies    INTEGER NOT NULL,
    micros   INTEGER NOT NULL,
    PRIMARY KEY (match_id, idx)
);
