-- Positions whose value was certified and the certificates accepted by the
-- Lean checker (`ulttt-check`). `value` is from X's point of view (-1, 0, 1).
CREATE TABLE verified (
    variant    TEXT NOT NULL,
    position   TEXT NOT NULL,
    value      INTEGER NOT NULL,
    cert_moves INTEGER NOT NULL,
    solver     TEXT NOT NULL,
    checked_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (variant, position)
);
