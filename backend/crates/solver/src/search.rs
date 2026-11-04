//! The optimized solver: negamax alpha-beta plus a set of enhancements that can
//! each be switched off, so their effect can be measured with `ulttt bench`.

use ulttt_game::{Moves, Player, Position, Variant};

use crate::tt::Table;
use crate::{Solver, Stats, Value};

/// Which enhancements `Search` uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// Transposition table size as a power of two; `0` disables it.
    pub tt_bits: u32,
    /// Only use the transposition table in positions with at least this many
    /// empty cells. Near the leaves, probing costs more than it saves.
    pub tt_min_empties: u32,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            tt_bits: 24,
            tt_min_empties: 12,
        }
    }
}

impl Options {
    /// Parses `key=value` pairs separated by commas, e.g. `tt=20`, starting
    /// from the defaults.
    pub fn parse(spec: &str) -> Result<Options, String> {
        let mut opts = Options::default();
        for part in spec.split(',').filter(|p| !p.is_empty()) {
            let (key, value) = part
                .split_once('=')
                .ok_or_else(|| format!("expected key=value, got {part:?}"))?;
            let n: u32 = value
                .parse()
                .map_err(|_| format!("bad value in {part:?}"))?;
            match key {
                "tt" if n <= 32 => opts.tt_bits = n,
                "ttmin" => opts.tt_min_empties = n,
                _ => return Err(format!("unknown option {part:?}")),
            }
        }
        Ok(opts)
    }

    /// The options that differ from the defaults, in `parse` syntax.
    pub fn spec(&self) -> String {
        let default = Options::default();
        let mut parts = Vec::new();
        if self.tt_bits != default.tt_bits {
            parts.push(format!("tt={}", self.tt_bits));
        }
        if self.tt_min_empties != default.tt_min_empties {
            parts.push(format!("ttmin={}", self.tt_min_empties));
        }
        parts.join(",")
    }
}

pub struct Search {
    opts: Options,
    tt: Option<Table>,
    stats: Stats,
    node_limit: u64,
    aborted: bool,
}

impl Search {
    pub fn new(opts: Options) -> Search {
        let tt = (opts.tt_bits > 0).then(|| Table::new(opts.tt_bits));
        Search {
            opts,
            tt,
            stats: Stats::default(),
            node_limit: u64::MAX,
            aborted: false,
        }
    }

    /// The score of `pos` for the player to move, in `-1..=1`, searched in the
    /// window `(alpha, beta)`. Fails soft: a result `<= alpha` is an upper
    /// bound and a result `>= beta` a lower bound.
    fn search(&mut self, pos: &Position, v: Variant, mut alpha: i8, mut beta: i8) -> i8 {
        self.stats.nodes += 1;
        if self.stats.nodes > self.node_limit {
            self.aborted = true;
            return 0;
        }
        if let Some(outcome) = pos.result(v) {
            let x = outcome.score();
            return if pos.to_move() == Player::X { x } else { -x };
        }

        let use_tt = self.tt.is_some() && pos.empties() >= self.opts.tt_min_empties;
        let key = if use_tt { pos.hash64() } else { 0 };
        let entry = match &self.tt {
            Some(t) if use_tt => t.probe(key),
            _ => None,
        };
        if let Some((lower, upper)) = entry {
            if lower >= beta || lower == upper {
                return lower;
            }
            if upper <= alpha {
                return upper;
            }
            alpha = alpha.max(lower);
            beta = beta.min(upper);
        }
        let alpha0 = alpha;

        let mut best = -1;
        for m in Moves(pos.legal_moves(v)) {
            let score = -self.search(&pos.after(m), v, -beta, -alpha);
            if self.aborted {
                return 0;
            }
            if score > best {
                best = score;
                if best > alpha {
                    alpha = best;
                    if alpha >= beta {
                        break;
                    }
                }
            }
        }

        if let Some(t) = self.tt.as_mut().filter(|_| use_tt) {
            let (lower, upper) = if best <= alpha0 {
                (-1, best)
            } else if best >= beta {
                (best, 1)
            } else {
                (best, best)
            };
            t.store(key, lower, upper);
        }
        best
    }
}

impl Solver for Search {
    fn name(&self) -> String {
        match self.opts.spec() {
            s if s.is_empty() => "search".into(),
            s => format!("search:{s}"),
        }
    }

    fn set_node_limit(&mut self, limit: Option<u64>) {
        self.node_limit = limit.unwrap_or(u64::MAX);
    }

    fn solve(&mut self, pos: &Position, v: Variant) -> Option<Value> {
        self.aborted = false;
        let limit = self.node_limit;
        self.node_limit = self.stats.nodes.saturating_add(limit);
        let score = self.search(pos, v, -1, 1);
        self.node_limit = limit;
        (!self.aborted).then(|| Value::from_score(score, pos.to_move()))
    }

    fn stats(&self) -> Stats {
        self.stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AlphaBeta;
    use ulttt_game::random::Rng;

    /// Every configuration agrees with the baseline on random endgames.
    #[test]
    fn agrees_with_alphabeta() {
        let configs = ["", "tt=0", "tt=10", "ttmin=0"];
        let mut rng = Rng::new(42);
        for i in 0..60 {
            let v = Variant::ALL[i % 3];
            let Some(pos) = rng.position(v, 52) else {
                continue;
            };
            let want = AlphaBeta::new().solve(&pos, v);
            for c in configs {
                let mut s = Search::new(Options::parse(c).unwrap());
                assert_eq!(s.solve(&pos, v), want, "{c} on {pos}");
            }
        }
    }

    #[test]
    fn option_specs_round_trip() {
        assert_eq!(Options::parse("").unwrap(), Options::default());
        let o = Options::parse("tt=12").unwrap();
        assert_eq!(o.tt_bits, 12);
        assert_eq!(Options::parse(&o.spec()).unwrap(), o);
        assert!(Options::parse("bogus=1").is_err());
    }
}
