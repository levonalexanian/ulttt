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
    "dfpn:tt=<bits>,sym=<empties>,ttmin=<empties>,progress=1",
    "search",
    "search:<options>",
];

/// Default df-pn table size (2^22 entries) and thresholds.
const DFPN_TT_BITS: u32 = 22;
const DFPN_SYM_MIN_EMPTIES: u32 = 60;
const DFPN_TT_MIN_EMPTIES: u32 = 8;

/// Parses df-pn options `tt=<bits>`, `sym=<empties>`, `ttmin=<empties>` and
/// `progress=1` (print the root's progress to stderr),
/// comma separated.
fn dfpn_from_spec(spec: &str) -> Option<DfPn> {
    let (mut tt, mut sym, mut ttmin) = (DFPN_TT_BITS, DFPN_SYM_MIN_EMPTIES, DFPN_TT_MIN_EMPTIES);
    let mut progress = false;
    for part in spec.split(',').filter(|p| !p.is_empty()) {
        let (key, value) = part.split_once('=')?;
        let n: u32 = value.parse().ok()?;
        match key {
            "tt" if (2..=32).contains(&n) => tt = n,
            "sym" => sym = n,
            "ttmin" => ttmin = n,
            "progress" if n <= 1 => progress = n == 1,
            _ => return None,
        }
    }
    Some(
        DfPn::new(tt, sym)
            .with_tt_min_empties(ttmin)
            .with_progress(progress),
    )
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

/// A line of optimal play from `pos`: at each turn, the first legal move that
/// keeps the position's value. `None` if a search hit the node limit.
pub fn principal_variation(
    solver: &mut dyn Solver,
    pos: &Position,
    v: Variant,
) -> Option<Vec<ulttt_game::Move>> {
    let value = solver.solve(pos, v)?;
    let mut line = Vec::new();
    let mut cur = *pos;
    while !cur.is_terminal(v) {
        let mut next = None;
        for m in ulttt_game::Moves(cur.legal_moves(v)) {
            if solver.solve(&cur.after(m), v)? == value {
                next = Some(m);
                break;
            }
        }
        let m = next.expect("some move keeps the value");
        line.push(m);
        cur.play(m);
    }
    Some(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ulttt_game::random::Rng;

    /// A principal variation is a legal game ending in the solved result.
    #[test]
    fn principal_variation_ends_in_the_value() {
        let mut rng = Rng::new(17);
        let mut solver = by_name("dfpn").unwrap();
        for i in 0..30 {
            let v = Variant::ALL[i % 3];
            let Some(pos) = rng.position(v, 50) else {
                continue;
            };
            let value = solver.solve(&pos, v).unwrap();
            let line = principal_variation(solver.as_mut(), &pos, v).unwrap();
            let mut cur = pos;
            for &m in &line {
                assert!(cur.is_legal(v, m));
                cur.play(m);
            }
            let outcome = cur.result(v).expect("the line ends the game");
            assert_eq!(outcome.score(), value as i8, "{v} {pos}");
        }
    }
}
