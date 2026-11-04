//! SQLite storage for benchmark suites, benchmark runs and solved positions.
//!
//! The database lives in `data/` by default (gitignored). Its schema is in
//! `schema.sql`; the schema version is tracked with `PRAGMA user_version`.

use std::path::Path;

use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, params};
use ulttt_game::{Position, Variant};
use ulttt_solver::Value;

pub use rusqlite::Error;
pub type Result<T> = rusqlite::Result<T>;

/// Schema migrations, applied in order. Never edit a released entry; append.
const MIGRATIONS: &[&str] = &[
    include_str!("schema.sql"),
    include_str!("verified.sql"),
    include_str!("matches.sql"),
    include_str!("match_time.sql"),
];

/// What to record about a new match.
#[derive(Clone, Debug)]
pub struct NewMatch<'a> {
    pub variant: Variant,
    pub engine_a: &'a str,
    pub engine_b: &'a str,
    pub node_limit: u64,
    /// Milliseconds per move, if moves are limited by time.
    pub ms: Option<u64>,
    pub git_rev: &'a str,
    pub notes: &'a str,
}

/// One finished game of a match.
#[derive(Clone, Debug, PartialEq)]
pub struct MatchGame {
    pub idx: usize,
    pub opening: Position,
    pub a_is_x: bool,
    /// For engine A: 1 win, 0 draw, -1 loss.
    pub score: i8,
    pub plies: u32,
    pub micros: u64,
}

/// Totals for one match.
#[derive(Clone, Debug, PartialEq)]
pub struct MatchSummary {
    pub id: i64,
    pub variant: String,
    pub engine_a: String,
    pub engine_b: String,
    pub node_limit: u64,
    pub ms: Option<u64>,
    pub git_rev: String,
    pub started_at: String,
    pub a_wins: u64,
    pub draws: u64,
    pub b_wins: u64,
}

/// A benchmark suite.
#[derive(Clone, Debug, PartialEq)]
pub struct Suite {
    pub id: i64,
    pub name: String,
    pub variant: Variant,
    pub description: String,
    pub positions: Vec<Position>,
}

/// What to record about a new run.
#[derive(Clone, Debug, Default)]
pub struct NewRun<'a> {
    pub suite_id: i64,
    pub solver: &'a str,
    pub git_rev: &'a str,
    pub node_limit: Option<u64>,
    pub notes: &'a str,
}

/// Totals for one run.
#[derive(Clone, Debug, PartialEq)]
pub struct RunSummary {
    pub id: i64,
    pub suite: String,
    pub solver: String,
    pub git_rev: String,
    pub started_at: String,
    pub positions: u64,
    pub solved: u64,
    pub nodes: u64,
    pub micros: u64,
}

/// The result of one position in a run.
#[derive(Clone, Debug, PartialEq)]
pub struct RunResult {
    pub idx: usize,
    /// `None` when the node limit was hit.
    pub value: Option<Value>,
    pub nodes: u64,
    pub micros: u64,
}

/// What the database holds; see [`Db::summary`].
#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    pub suites: u64,
    pub runs: u64,
    /// Solved positions per variant.
    pub solved: Vec<(String, u64)>,
    /// Lean-verified positions per variant.
    pub verified: Vec<(String, u64)>,
}

pub struct Db {
    conn: Connection,
}

fn value_to_sql(v: Value) -> i64 {
    v as i64
}

fn value_from_sql(x: i64) -> Value {
    match x.signum() {
        1 => Value::XWins,
        -1 => Value::OWins,
        _ => Value::Draw,
    }
}

fn parse_error(col: usize, e: String) -> Error {
    Error::FromSqlConversionFailure(col, Type::Text, e.into())
}

impl Db {
    /// Opens (creating if needed) the database at `path` and migrates it.
    pub fn open(path: impl AsRef<Path>) -> Result<Db> {
        Self::init(Connection::open(path)?)
    }

    /// A fresh in-memory database, for tests.
    pub fn open_in_memory() -> Result<Db> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Db> {
        conn.pragma_update(None, "foreign_keys", true)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            conn.execute_batch(sql)?;
            conn.pragma_update(None, "user_version", i as i64 + 1)?;
        }
        Ok(Db { conn })
    }

    /// Creates a suite with the given positions and returns its id.
    pub fn create_suite(
        &mut self,
        name: &str,
        variant: Variant,
        description: &str,
        positions: &[Position],
    ) -> Result<i64> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO suites (name, variant, description) VALUES (?1, ?2, ?3)",
            params![name, variant.name(), description],
        )?;
        let id = tx.last_insert_rowid();
        {
            let mut stmt = tx.prepare(
                "INSERT INTO suite_positions (suite_id, idx, position) VALUES (?1, ?2, ?3)",
            )?;
            for (i, pos) in positions.iter().enumerate() {
                stmt.execute(params![id, i as i64, pos.to_string()])?;
            }
        }
        tx.commit()?;
        Ok(id)
    }

    /// The suite with this name, if any.
    pub fn suite(&self, name: &str) -> Result<Option<Suite>> {
        let row = self
            .conn
            .query_row(
                "SELECT id, variant, description FROM suites WHERE name = ?1",
                params![name],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get(2)?)),
            )
            .optional()?;
        let Some((id, variant, description)) = row else {
            return Ok(None);
        };
        let variant = variant.parse::<Variant>().map_err(|e| parse_error(1, e))?;
        let mut stmt = self
            .conn
            .prepare("SELECT position FROM suite_positions WHERE suite_id = ?1 ORDER BY idx")?;
        let positions = stmt
            .query_map(params![id], |r| r.get::<_, String>(0))?
            .map(|s| s?.parse::<Position>().map_err(|e| parse_error(0, e)))
            .collect::<Result<Vec<_>>>()?;
        Ok(Some(Suite {
            id,
            name: name.to_string(),
            variant,
            description,
            positions,
        }))
    }

    /// Names of all suites, oldest first.
    pub fn suite_names(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT name FROM suites ORDER BY id")?;
        stmt.query_map([], |r| r.get(0))?.collect()
    }

    /// Records the start of a match and returns its id.
    pub fn start_match(&self, m: &NewMatch) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO matches (variant, engine_a, engine_b, node_limit, git_rev, notes, ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                m.variant.name(),
                m.engine_a,
                m.engine_b,
                m.node_limit as i64,
                m.git_rev,
                m.notes,
                m.ms.map(|ms| ms as i64)
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Records one game of a match.
    pub fn record_game(&self, match_id: i64, g: &MatchGame) -> Result<()> {
        self.conn.execute(
            "INSERT INTO match_games (match_id, idx, opening, a_is_x, score, plies, micros)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                match_id,
                g.idx as i64,
                g.opening.to_string(),
                g.a_is_x,
                g.score,
                g.plies,
                g.micros as i64
            ],
        )?;
        Ok(())
    }

    /// Marks a match as finished.
    pub fn finish_match(&self, match_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE matches SET finished_at = datetime('now') WHERE id = ?1",
            params![match_id],
        )?;
        Ok(())
    }

    /// Totals of the most recent matches, newest first.
    pub fn match_summaries(&self, limit: usize) -> Result<Vec<MatchSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.variant, m.engine_a, m.engine_b, m.node_limit, m.git_rev,
                    m.started_at,
                    COALESCE(SUM(g.score = 1), 0), COALESCE(SUM(g.score = 0), 0),
                    COALESCE(SUM(g.score = -1), 0), m.ms
             FROM matches m LEFT JOIN match_games g ON g.match_id = m.id
             GROUP BY m.id ORDER BY m.id DESC LIMIT ?1",
        )?;
        stmt.query_map(params![limit as i64], |r| {
            Ok(MatchSummary {
                id: r.get(0)?,
                variant: r.get(1)?,
                engine_a: r.get(2)?,
                engine_b: r.get(3)?,
                node_limit: r.get::<_, i64>(4)? as u64,
                git_rev: r.get(5)?,
                started_at: r.get(6)?,
                a_wins: r.get::<_, i64>(7)? as u64,
                draws: r.get::<_, i64>(8)? as u64,
                b_wins: r.get::<_, i64>(9)? as u64,
                ms: r.get::<_, Option<i64>>(10)?.map(|ms| ms as u64),
            })
        })?
        .collect()
    }

    /// Records the start of a run and returns its id.
    pub fn start_run(&self, run: &NewRun) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO runs (suite_id, solver, git_rev, node_limit, notes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                run.suite_id,
                run.solver,
                run.git_rev,
                run.node_limit.map(|n| n as i64),
                run.notes
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Records the result of position `idx` in a run.
    pub fn record_result(
        &self,
        run_id: i64,
        idx: usize,
        value: Option<Value>,
        nodes: u64,
        micros: u64,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO run_results (run_id, idx, value, nodes, micros)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                run_id,
                idx as i64,
                value.map(value_to_sql),
                nodes as i64,
                micros as i64
            ],
        )?;
        Ok(())
    }

    /// Marks a run as finished.
    pub fn finish_run(&self, run_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE runs SET finished_at = datetime('now') WHERE id = ?1",
            params![run_id],
        )?;
        Ok(())
    }

    /// Totals of the most recent runs, newest first, optionally for one suite.
    pub fn run_summaries(&self, suite: Option<&str>, limit: usize) -> Result<Vec<RunSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT r.id, s.name, r.solver, r.git_rev, r.started_at,
                    COUNT(x.idx), COUNT(x.value), COALESCE(SUM(x.nodes), 0),
                    COALESCE(SUM(x.micros), 0)
             FROM runs r
             JOIN suites s ON s.id = r.suite_id
             LEFT JOIN run_results x ON x.run_id = r.id
             WHERE ?1 IS NULL OR s.name = ?1
             GROUP BY r.id
             ORDER BY r.id DESC
             LIMIT ?2",
        )?;
        stmt.query_map(params![suite, limit as i64], |r| {
            Ok(RunSummary {
                id: r.get(0)?,
                suite: r.get(1)?,
                solver: r.get(2)?,
                git_rev: r.get(3)?,
                started_at: r.get(4)?,
                positions: r.get::<_, i64>(5)? as u64,
                solved: r.get::<_, i64>(6)? as u64,
                nodes: r.get::<_, i64>(7)? as u64,
                micros: r.get::<_, i64>(8)? as u64,
            })
        })?
        .collect()
    }

    /// Every result of a run, by position index.
    pub fn run_results(&self, run_id: i64) -> Result<Vec<RunResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT idx, value, nodes, micros FROM run_results WHERE run_id = ?1 ORDER BY idx",
        )?;
        stmt.query_map(params![run_id], |r| {
            Ok(RunResult {
                idx: r.get::<_, i64>(0)? as usize,
                value: r.get::<_, Option<i64>>(1)?.map(value_from_sql),
                nodes: r.get::<_, i64>(2)? as u64,
                micros: r.get::<_, i64>(3)? as u64,
            })
        })?
        .collect()
    }

    /// The values a run found, by position index (`None` where unsolved).
    pub fn run_values(&self, run_id: i64) -> Result<Vec<(usize, Option<Value>)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT idx, value FROM run_results WHERE run_id = ?1 ORDER BY idx")?;
        stmt.query_map(params![run_id], |r| {
            Ok((
                r.get::<_, i64>(0)? as usize,
                r.get::<_, Option<i64>>(1)?.map(value_from_sql),
            ))
        })?
        .collect()
    }

    /// Stores the exact value of a position, replacing any earlier entry.
    pub fn store_solved(
        &self,
        variant: Variant,
        pos: &Position,
        value: Value,
        nodes: u64,
        solver: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO solved (variant, position, value, nodes, solver)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                variant.name(),
                pos.to_string(),
                value_to_sql(value),
                nodes as i64,
                solver
            ],
        )?;
        Ok(())
    }

    /// Records a position whose certificates the Lean checker accepted.
    pub fn store_verified(
        &self,
        variant: Variant,
        pos: &Position,
        value: Value,
        cert_moves: u64,
        solver: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO verified (variant, position, value, cert_moves, solver)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                variant.name(),
                pos.to_string(),
                value_to_sql(value),
                cert_moves as i64,
                solver
            ],
        )?;
        Ok(())
    }

    /// The verified value of a position, if any.
    pub fn lookup_verified(&self, variant: Variant, pos: &Position) -> Result<Option<Value>> {
        self.conn
            .query_row(
                "SELECT value FROM verified WHERE variant = ?1 AND position = ?2",
                params![variant.name(), pos.to_string()],
                |r| r.get::<_, i64>(0),
            )
            .optional()
            .map(|v| v.map(value_from_sql))
    }

    /// Counts of what the database holds: suites, runs, then solved positions
    /// and Lean-verified positions per variant.
    pub fn summary(&self) -> Result<Summary> {
        let count = |sql: &str| -> Result<u64> {
            self.conn
                .query_row(sql, [], |r| r.get::<_, i64>(0).map(|n| n as u64))
        };
        let mut stmt = self
            .conn
            .prepare("SELECT variant, COUNT(*) FROM solved GROUP BY variant ORDER BY variant")?;
        let solved = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get::<_, i64>(1)? as u64)))?
            .collect::<Result<_>>()?;
        Ok(Summary {
            suites: count("SELECT COUNT(*) FROM suites")?,
            runs: count("SELECT COUNT(*) FROM runs")?,
            solved,
            verified: self.verified_counts()?,
        })
    }

    /// Number of verified positions per variant.
    pub fn verified_counts(&self) -> Result<Vec<(String, u64)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT variant, COUNT(*) FROM verified GROUP BY variant ORDER BY variant")?;
        stmt.query_map([], |r| Ok((r.get(0)?, r.get::<_, i64>(1)? as u64)))?
            .collect()
    }

    /// The stored exact value of a position, if known.
    pub fn lookup_solved(&self, variant: Variant, pos: &Position) -> Result<Option<Value>> {
        self.conn
            .query_row(
                "SELECT value FROM solved WHERE variant = ?1 AND position = ?2",
                params![variant.name(), pos.to_string()],
                |r| r.get::<_, i64>(0),
            )
            .optional()
            .map(|v| v.map(value_from_sql))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ulttt_game::random::Rng;

    #[test]
    fn suites_runs_and_solved_positions() {
        let mut db = Db::open_in_memory().unwrap();
        let mut rng = Rng::new(1);
        let positions: Vec<Position> = (0..3)
            .map(|_| rng.position(Variant::Open, 10).unwrap())
            .collect();
        let id = db
            .create_suite("test", Variant::Open, "three positions", &positions)
            .unwrap();
        let suite = db.suite("test").unwrap().unwrap();
        assert_eq!(suite.id, id);
        assert_eq!(suite.variant, Variant::Open);
        assert_eq!(suite.positions, positions);
        assert_eq!(db.suite("missing").unwrap(), None);
        assert_eq!(db.suite_names().unwrap(), ["test"]);

        let run = db
            .start_run(&NewRun {
                suite_id: id,
                solver: "alphabeta",
                ..Default::default()
            })
            .unwrap();
        db.record_result(run, 0, Some(Value::XWins), 10, 5).unwrap();
        db.record_result(run, 1, None, 20, 7).unwrap();
        db.record_result(run, 2, Some(Value::Draw), 30, 9).unwrap();
        db.finish_run(run).unwrap();
        let summary = &db.run_summaries(Some("test"), 10).unwrap()[0];
        assert_eq!((summary.positions, summary.solved), (3, 2));
        assert_eq!((summary.nodes, summary.micros), (60, 21));
        assert_eq!(
            db.run_values(run).unwrap(),
            [(0, Some(Value::XWins)), (1, None), (2, Some(Value::Draw))]
        );

        db.store_solved(Variant::Open, &positions[0], Value::OWins, 5, "alphabeta")
            .unwrap();
        assert_eq!(
            db.lookup_solved(Variant::Open, &positions[0]).unwrap(),
            Some(Value::OWins)
        );
        assert_eq!(
            db.lookup_solved(Variant::Classic, &positions[0]).unwrap(),
            None
        );

        db.store_verified(Variant::Open, &positions[1], Value::Draw, 42, "dfpn")
            .unwrap();
        assert_eq!(
            db.lookup_verified(Variant::Open, &positions[1]).unwrap(),
            Some(Value::Draw)
        );
        assert_eq!(db.verified_counts().unwrap(), [("open".to_string(), 1)]);
        let summary = db.summary().unwrap();
        assert_eq!((summary.suites, summary.runs), (1, 1));
        assert_eq!(summary.solved, [("open".to_string(), 1)]);
    }

    #[test]
    fn matches() {
        let db = Db::open_in_memory().unwrap();
        let id = db
            .start_match(&NewMatch {
                variant: Variant::Classic,
                engine_a: "engine",
                engine_b: "engine:tempo=0",
                node_limit: 1000,
                ms: Some(5),
                git_rev: "abc",
                notes: "",
            })
            .unwrap();
        for (idx, score) in [1, 1, 0, -1].into_iter().enumerate() {
            let g = MatchGame {
                idx,
                opening: Position::new(),
                a_is_x: idx % 2 == 0,
                score,
                plies: 40,
                micros: 10,
            };
            db.record_game(id, &g).unwrap();
        }
        db.finish_match(id).unwrap();
        let m = &db.match_summaries(10).unwrap()[0];
        assert_eq!((m.a_wins, m.draws, m.b_wins), (2, 1, 1));
        assert_eq!(m.engine_b, "engine:tempo=0");
    }
}
