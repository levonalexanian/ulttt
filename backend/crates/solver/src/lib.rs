//! Exact solvers: compute the value of a position under perfect play, as defined
//! by `value` in `proofs/Ulttt/Value.lean`.

mod alphabeta;
mod dfpn;
mod order;
mod search;
mod tt;

pub use alphabeta::AlphaBeta;
pub use dfpn::DfPn;
pub use search::{Options, Search};

/// Names accepted by [`by_name`].
pub const SOLVERS: &[&str] = &[
    "alphabeta",
    "dfpn",
    "dfpn:tt=<bits>,sym=<empties>,ttmin=<empties>",
    "search",
    "search:<options>",
];

/// Default df-pn table size (2^22 entries) and thresholds.
const DFPN_TT_BITS: u32 = 22;
const DFPN_SYM_MIN_EMPTIES: u32 = 60;
const DFPN_TT_MIN_EMPTIES: u32 = 8;

/// Parses df-pn options `tt=<bits>`, `sym=<empties>` and `ttmin=<empties>`,
/// comma separated.
fn dfpn_from_spec(spec: &str) -> Option<DfPn> {
    let (mut tt, mut sym, mut ttmin) = (DFPN_TT_BITS, DFPN_SYM_MIN_EMPTIES, DFPN_TT_MIN_EMPTIES);
    for part in spec.split(',').filter(|p| !p.is_empty()) {
        let (key, value) = part.split_once('=')?;
        let n: u32 = value.parse().ok()?;
        match key {
            "tt" if (2..=32).contains(&n) => tt = n,
            "sym" => sym = n,
            "ttmin" => ttmin = n,
            _ => return None,
        }
    }
    Some(DfPn::new(tt, sym).with_tt_min_empties(ttmin))
}

/// A fresh solver by name: `alphabeta`, `dfpn`, `dfpn:<options>`, `search`, or
/// `search:<options>` with options as in [`Options::parse`].
pub fn by_name(name: &str) -> Option<Box<dyn Solver>> {
    match name.split_once(':') {
        _ if name == "alphabeta" => Some(Box::new(AlphaBeta::new())),
        _ if name == "dfpn" => dfpn_from_spec("").map(|s| Box::new(s) as Box<dyn Solver>),
        Some(("dfpn", spec)) => dfpn_from_spec(spec).map(|s| Box::new(s) as Box<dyn Solver>),
        _ if name == "search" => Some(Box::new(Search::new(Options::default()))),
        Some(("search", spec)) => Options::parse(spec)
            .ok()
            .map(|o| Box::new(Search::new(o)) as Box<dyn Solver>),
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

    /// Forgets everything learned and resets the counters, so the next search
    /// runs as on a fresh solver, but reuses allocated memory.
    fn reset(&mut self);
}
