-- The moves of each match game, space separated, so games can be replayed
-- (e.g. to tune the evaluation on their positions).
ALTER TABLE match_games ADD COLUMN moves TEXT NOT NULL DEFAULT '';
