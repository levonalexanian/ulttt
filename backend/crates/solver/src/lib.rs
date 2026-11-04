//! Exact solvers: compute the value of a position under perfect play, as defined
//! by `value` in `proofs/Ulttt/Value.lean`.

mod alphabeta;

pub use alphabeta::AlphaBeta;

/// Names accepted by [`by_name`].
pub const SOLVERS: &[&str] = &["alphabeta"];

/// A fresh solver by name.
pub fn by_name(name: &str) -> Option<Box<dyn Solver>> {
    match name {
        "alphabeta" => Some(Box::new(AlphaBeta::new())),
        _ => None,
    }
}

use ulttt_game::{Player, Position, Variant};

/// The value of a position under perfect play, from X's point of view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Value {
    OWins = -1,
    Draw = 0,
    XWins = 1,
}

impl Value {
    /// From a score in `-1..=1` for the player `p`.
    pub fn from_score(score: i8, p: Player) -> Value {
        let x = if p == Player::X { score } else { -score };
        match x.signum() {
            1 => Value::XWins,
            -1 => Value::OWins,
            _ => Value::Draw,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Value::XWins => "x",
            Value::OWins => "o",
            Value::Draw => "draw",
        }
    }
}

/// Counters collected during a search.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// Positions visited.
    pub nodes: u64,
}

/// A solver computes exact values.
pub trait Solver {
    /// A short name identifying the solver and its configuration.
    fn name(&self) -> String;

    /// Stop searches after visiting this many positions. `None` means no limit.
    fn set_node_limit(&mut self, limit: Option<u64>);

    /// The value of `pos` under perfect play in variant `v`, or `None` if the
    /// node limit was reached first.
    fn solve(&mut self, pos: &Position, v: Variant) -> Option<Value>;

    /// Counters for the searches run so far.
    fn stats(&self) -> Stats;
}
