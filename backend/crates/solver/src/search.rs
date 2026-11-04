//! The optimized solver: negamax alpha-beta plus a set of enhancements that can
//! each be switched off, so their effect can be measured with `ulttt bench`.

use ulttt_game::{Move, Moves, Player, Position, Variant};

use crate::order::{order, winning_move};
use crate::tt::{Entry, Table};
use crate::{Solver, Stats, Value};

/// Which enhancements `Search` uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// Transposition table size as a power of two; `0` disables it.
    pub tt_bits: u32,
    /// Only use the transposition table in positions with at least this many
    /// empty cells. Near the leaves, probing costs more than it saves.
    pub tt_min_empties: u32,
    /// Bound the value by a draw when a player can no longer win.
    pub draw_detection: bool,
    /// Return a win at once when a move completes a global line.
    pub immediate_wins: bool,
    /// Search the transposition table's best move first, then order the rest
    /// by a static heuristic.
    pub ordering: bool,
    /// Break ordering ties by how often each move caused a cutoff before.
    pub history: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            tt_bits: 24,
            tt_min_empties: 12,
            draw_detection: true,
            immediate_wins: true,
            ordering: true,
            history: true,
        }
    }
}

impl Options {
    /// Parses `key=value` pairs separated by commas, e.g. `tt=20,order=0`,
    /// starting from the defaults. Keys: `tt`, `ttmin`, `draw`, `win`, `order`,
    /// `hist`.
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
                "draw" if n <= 1 => opts.draw_detection = n == 1,
                "win" if n <= 1 => opts.immediate_wins = n == 1,
                "order" if n <= 1 => opts.ordering = n == 1,
                "hist" if n <= 1 => opts.history = n == 1,
                _ => return Err(format!("unknown option {part:?}")),
            }
        }
        Ok(opts)
    }

    /// The options that differ from the defaults, in `parse` syntax.
    pub fn spec(&self) -> String {
        let d = Options::default();
        let fields = [
            ("tt", self.tt_bits, d.tt_bits),
            ("ttmin", self.tt_min_empties, d.tt_min_empties),
            ("draw", self.draw_detection as u32, d.draw_detection as u32),
            ("win", self.immediate_wins as u32, d.immediate_wins as u32),
            ("order", self.ordering as u32, d.ordering as u32),
            ("hist", self.history as u32, d.history as u32),
        ];
        fields
            .iter()
            .filter(|(_, value, default)| value != default)
            .map(|(key, value, _)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(",")
    }
}

pub struct Search {
    opts: Options,
    tt: Option<Table>,
    stats: Stats,
    node_limit: u64,
    aborted: bool,
    /// Cutoff counts per move, for each player to move.
    history: [[u32; 81]; 2],
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
            history: [[0; 81]; 2],
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

        // The value is at least `floor` and at most `cap`.
        let (mut floor, mut cap) = (-1, 1);
        if self.opts.draw_detection {
            let me = pos.to_move();
            if !pos.may_win(v, me) {
                cap = 0;
            }
            if !pos.may_win(v, me.other()) {
                floor = 0;
            }
            if floor == cap {
                return 0;
            }
            if cap <= alpha {
                return cap;
            }
            if floor >= beta {
                return floor;
            }
            alpha = alpha.max(floor);
            beta = beta.min(cap);
        }

        let use_tt = self.tt.is_some() && pos.empties() >= self.opts.tt_min_empties;
        let key = if use_tt { pos.hash64() } else { 0 };
        let entry = match &self.tt {
            Some(t) if use_tt => t.probe(key),
            _ => None,
        };
        let mut tt_move = None;
        if let Some(e) = entry {
            if e.lower >= beta || e.lower == e.upper {
                return e.lower;
            }
            if e.upper <= alpha {
                return e.upper;
            }
            alpha = alpha.max(e.lower);
            beta = beta.min(e.upper);
            tt_move = e.best;
        }
        let alpha0 = alpha;

        let legal = pos.legal_moves(v);
        if self.opts.immediate_wins && winning_move(pos, legal).is_some() {
            return 1;
        }

        let mut best = -1;
        let mut best_move = None;
        let mut search_move = |this: &mut Self, m: Move, alpha: &mut i8| -> Option<bool> {
            let score = -this.search(&pos.after(m), v, -beta, -*alpha);
            if this.aborted {
                return None;
            }
            if score > best {
                best = score;
                best_move = Some(m);
                if best > *alpha {
                    *alpha = best;
                    if *alpha >= beta {
                        if this.opts.history {
                            let h = &mut this.history[pos.to_move().index()][m as usize];
                            *h = h.saturating_add(pos.empties() * pos.empties());
                        }
                        return Some(true);
                    }
                }
            }
            Some(false)
        };
        if self.opts.ordering {
            for &m in order(pos, v, legal, tt_move, &self.history[pos.to_move().index()]).as_slice()
            {
                match search_move(self, m, &mut alpha) {
                    None => return 0,
                    Some(true) => break,
                    Some(false) => {}
                }
            }
        } else {
            for m in Moves(legal) {
                match search_move(self, m, &mut alpha) {
                    None => return 0,
                    Some(true) => break,
                    Some(false) => {}
                }
            }
        }

        // Children's bounds can be looser than what is known here.
        best = best.clamp(floor, cap);

        if let Some(t) = self.tt.as_mut().filter(|_| use_tt) {
            let (lower, upper) = if best <= alpha0 {
                (-1, best)
            } else if best >= beta {
                (best, 1)
            } else {
                (best, best)
            };
            // After a fail-low every move failed, so none is known to be best.
            let best = if best > alpha0 { best_move } else { None };
            t.store(key, Entry { lower, upper, best });
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

    fn reset(&mut self) {
        self.stats = Stats::default();
        self.history = [[0; 81]; 2];
        if let Some(t) = self.tt.as_mut() {
            t.clear();
        }
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
        let configs = [
            "",
            "tt=0",
            "tt=10",
            "ttmin=0",
            "draw=0",
            "draw=0,tt=0",
            "win=0",
            "order=0",
            "order=0,tt=0,win=0,draw=0",
            "hist=0",
        ];
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
