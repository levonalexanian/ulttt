-- Matches may limit each move by time (milliseconds) instead of only nodes.
ALTER TABLE matches ADD COLUMN ms INTEGER;
