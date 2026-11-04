-- Benchmark suites: fixed lists of positions to solve.
CREATE TABLE suites (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    variant     TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE suite_positions (
    suite_id INTEGER NOT NULL REFERENCES suites(id) ON DELETE CASCADE,
    idx      INTEGER NOT NULL,
    position TEXT NOT NULL,
    PRIMARY KEY (suite_id, idx)
);

-- One run of a solver over a suite.
CREATE TABLE runs (
    id          INTEGER PRIMARY KEY,
    suite_id    INTEGER NOT NULL REFERENCES suites(id) ON DELETE CASCADE,
    solver      TEXT NOT NULL,
    git_rev     TEXT NOT NULL DEFAULT '',
    node_limit  INTEGER,
    notes       TEXT NOT NULL DEFAULT '',
    started_at  TEXT NOT NULL DEFAULT (datetime('now')),
    finished_at TEXT
);

-- The result of one position in a run. `value` is NULL when the node limit was hit.
CREATE TABLE run_results (
    run_id INTEGER NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    idx    INTEGER NOT NULL,
    value  INTEGER,
    nodes  INTEGER NOT NULL,
    micros INTEGER NOT NULL,
    PRIMARY KEY (run_id, idx)
);

-- Exact values of positions, from X's point of view (-1, 0, 1).
CREATE TABLE solved (
    variant   TEXT NOT NULL,
    position  TEXT NOT NULL,
    value     INTEGER NOT NULL,
    nodes     INTEGER NOT NULL,
    solver    TEXT NOT NULL,
    solved_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (variant, position)
);
