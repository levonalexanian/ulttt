//! Depth-first proof-number search (df-pn, Nagai 2002).
//!
//! Proof-number search answers yes/no questions: "can the attacker force a
//! result worth at least `r`?" It grows the tree where a proof (or disproof)
//! looks cheapest, which suits the forcing play of Ultimate Tic-Tac-Toe. A
//! three-valued result needs at most two questions: can X win, and if not, can
//! X at least draw.
//!
//! Notation (phi/delta form): every node is seen from its player to move. `phi`
//! is the cost to prove that the mover reaches their goal, `delta` the cost to
//! disprove it. The attacker's goal is a result worth at least `r` to them; the
//! defender's goal is to keep it below `r`.

use ulttt_game::{Moves, Outcome, Player, Position, Variant};

use crate::order::winning_move;
use crate::{Solver, Stats, Value};

const INF: u32 = u32::MAX / 4;

#[derive(Clone, Copy, Default)]
struct Entry {
    /// 0 marks an empty slot.
    key: u64,
    phi: u32,
    delta: u32,
    /// Nodes searched to compute this entry: entries that took more work are
    /// kept in preference to cheap ones.
    work: u32,
}

/// A transposition table of `(phi, delta)` in buckets of two entries.
struct Table {
    entries: Vec<[Entry; 2]>,
    mask: u64,
}

impl Table {
    /// A table with `2^bits` entries.
    fn new(bits: u32) -> Table {
        Table {
            entries: vec![[Entry::default(); 2]; 1 << (bits - 1)],
            mask: (1 << (bits - 1)) - 1,
        }
    }

    #[inline]
    fn get(&self, key: u64) -> Option<(u32, u32)> {
        self.entries[(key & self.mask) as usize]
            .iter()
            .find(|e| e.key == key)
            .map(|e| (e.phi, e.delta))
    }

    #[inline]
    fn put(&mut self, key: u64, phi: u32, delta: u32, work: u32) {
        let bucket = &mut self.entries[(key & self.mask) as usize];
        let slot = match bucket.iter().position(|e| e.key == key) {
            Some(i) => i,
            None => (bucket[1].work < bucket[0].work) as usize,
        };
        let work = if bucket[slot].key == key {
            work.max(bucket[slot].work)
        } else {
            work
        };
        bucket[slot] = Entry {
            key,
            phi,
            delta,
            work,
        };
    }

    fn clear(&mut self) {
        self.entries.fill([Entry::default(); 2]);
    }
}

/// The question being searched: can `attacker` force a result worth at least
/// `goal` to them (1: a win, 0: at least a draw)?
#[derive(Clone, Copy)]
struct Question {
    variant: Variant,
    attacker: Player,
    goal: i8,
    /// Mixed into position hashes so answers to different questions never collide.
    salt: u64,
}

impl Question {
    fn score(&self, o: Outcome) -> i8 {
        if self.attacker == Player::X {
            o.score()
        } else {
            -o.score()
        }
    }
}

pub struct DfPn {
    tt: Table,
    /// Positions with at least this many empty cells are looked up by their
    /// symmetry class, so symmetric positions share table entries.
    sym_min_empties: u32,
    /// Positions with fewer empty cells are not stored in or looked up from the
    /// table: near the end of the game, a probe costs more than it saves.
    tt_min_empties: u32,
    stats: Stats,
    node_limit: u64,
    limit_now: u64,
    aborted: bool,
    /// Child buffers for `mid`, reused between calls.
    spare: Vec<Vec<(Position, u32, u32)>>,
}

impl DfPn {
    /// A solver with `2^tt_bits` table entries, using symmetry classes for
    /// positions with at least `sym_min_empties` empty cells (82: never).
    pub fn new(tt_bits: u32, sym_min_empties: u32) -> DfPn {
        DfPn {
            tt: Table::new(tt_bits),
            sym_min_empties,
            tt_min_empties: 0,
            stats: Stats::default(),
            node_limit: u64::MAX,
            limit_now: u64::MAX,
            aborted: false,
            spare: Vec::new(),
        }
    }

    /// Skips the table for positions with fewer than `n` empty cells.
    pub fn with_tt_min_empties(mut self, n: u32) -> DfPn {
        self.tt_min_empties = n;
        self
    }

    /// `(phi, delta)` of a position known without search, if any.
    fn evaluate(&self, q: &Question, pos: &Position) -> Option<(u32, u32)> {
        let mover_is_attacker = pos.to_move() == q.attacker;
        // `true`: the mover reaches their goal; `false`: they do not.
        let mover_wins = |attacker_succeeds: bool| attacker_succeeds == mover_is_attacker;
        let as_pd = |mover_succeeds: bool| if mover_succeeds { (0, INF) } else { (INF, 0) };

        if let Some(o) = pos.result(q.variant) {
            return Some(as_pd(mover_wins(q.score(o) >= q.goal)));
        }
        let defender = q.attacker.other();
        // The attacker cannot win: a win is out of reach, and a draw is the best
        // they can hope for.
        if q.goal == 1 && !pos.may_win(q.variant, q.attacker) {
            return Some(as_pd(mover_wins(false)));
        }
        // The defender cannot win: the attacker gets at least a draw.
        if q.goal == 0 && !pos.may_win(q.variant, defender) {
            return Some(as_pd(mover_wins(true)));
        }
        // A move that wins the game at once.
        if winning_move(pos, pos.legal_moves(q.variant)).is_some() {
            // Winning is always the mover's goal: the attacker wins (>= goal) or
            // the defender wins (< goal, as goal >= 0).
            return Some((0, INF));
        }
        None
    }

    fn key(&self, q: &Question, pos: &Position) -> u64 {
        if pos.empties() >= self.sym_min_empties {
            return pos.canonical().0.hash64() ^ q.salt;
        }
        pos.hash64() ^ q.salt
    }

    fn lookup(&self, q: &Question, pos: &Position) -> (u32, u32) {
        if let Some(pd) = self.evaluate(q, pos) {
            return pd;
        }
        if pos.empties() < self.tt_min_empties {
            return (1, 1);
        }
        self.tt.get(self.key(q, pos)).unwrap_or((1, 1))
    }

    /// Searches `pos` until its `phi >= th_phi` or `delta >= th_delta`.
    /// `pos` must be one that `evaluate` does not settle: the parent checked
    /// that when it looked the child up.
    fn mid(&mut self, q: &Question, pos: &Position, th_phi: u32, th_delta: u32) -> (u32, u32) {
        self.stats.nodes += 1;
        if self.stats.nodes > self.limit_now {
            self.aborted = true;
            return (1, 1);
        }
        // Reuse a child buffer from an earlier call: no allocation or
        // initialization per node once the search has warmed up.
        let mut children = self.spare.pop().unwrap_or_default();
        let result = self.expand(q, pos, th_phi, th_delta, &mut children);
        self.spare.push(children);
        result
    }

    /// The body of `mid`, with `children` as scratch space for the children
    /// and their numbers. The numbers are kept here rather than re-read from
    /// the table, where they may be overwritten.
    fn expand(
        &mut self,
        q: &Question,
        pos: &Position,
        th_phi: u32,
        th_delta: u32,
        children: &mut Vec<(Position, u32, u32)>,
    ) -> (u32, u32) {
        let key = self.key(q, pos);
        let start = self.stats.nodes;
        children.clear();
        for m in Moves(pos.legal_moves(q.variant)) {
            let child = pos.after(m);
            let (phi, delta) = self.lookup(q, &child);
            children.push((child, phi, delta));
        }

        loop {
            // phi(n) = min delta(c); delta(n) = sum phi(c).
            let (mut best, mut best_delta, mut second_delta) = (0usize, INF, INF);
            let (mut sum_phi, mut best_phi) = (0u32, 0u32);
            for (i, &(_, cphi, cdelta)) in children.iter().enumerate() {
                sum_phi = sum_phi.saturating_add(cphi).min(INF);
                if cdelta < best_delta {
                    second_delta = best_delta;
                    best_delta = cdelta;
                    best = i;
                    best_phi = cphi;
                } else if cdelta < second_delta {
                    second_delta = cdelta;
                }
            }
            let (phi, delta) = (best_delta, sum_phi);
            if phi >= th_phi || delta >= th_delta || self.aborted {
                if !self.aborted {
                    let work = (self.stats.nodes - start).min(u32::MAX as u64) as u32;
                    if pos.empties() >= self.tt_min_empties {
                        self.tt.put(key, phi, delta, work);
                    }
                }
                return (phi, delta);
            }
            // Thresholds for the most promising child.
            let child_th_phi = th_delta
                .saturating_sub(delta)
                .saturating_add(best_phi)
                .min(INF);
            // The 1 + epsilon trick (Pawlewicz & Lew): a looser bound lets the
            // child search longer before control returns here.
            let child_th_delta = th_phi
                .min(
                    second_delta
                        .saturating_add(second_delta / 4)
                        .saturating_add(1),
                )
                .min(INF);
            let child = children[best].0;
            let (cphi, cdelta) = self.mid(q, &child, child_th_phi, child_th_delta);
            children[best].1 = cphi;
            children[best].2 = cdelta;
            if self.aborted {
                return (1, 1);
            }
        }
    }

    /// Answers the question for the root; `None` if the node limit was hit.
    fn prove(&mut self, q: &Question, pos: &Position) -> Option<bool> {
        let (phi, _) = match self.evaluate(q, pos) {
            Some(pd) => pd,
            None => self.mid(q, pos, INF - 1, INF - 1),
        };
        if self.aborted {
            return None;
        }
        let mover_is_attacker = pos.to_move() == q.attacker;
        // phi == 0: the mover reaches their goal.
        Some((phi == 0) == mover_is_attacker)
    }
}

impl Solver for DfPn {
    fn name(&self) -> String {
        "dfpn".into()
    }

    fn set_node_limit(&mut self, limit: Option<u64>) {
        self.node_limit = limit.unwrap_or(u64::MAX);
    }

    fn solve(&mut self, pos: &Position, v: Variant) -> Option<Value> {
        self.aborted = false;
        self.limit_now = self.stats.nodes.saturating_add(self.node_limit);
        let ask = |goal: i8, salt: u64| Question {
            variant: v,
            attacker: Player::X,
            goal,
            salt,
        };
        if self.prove(&ask(1, 0x51ED_2701), pos)? {
            return Some(Value::XWins);
        }
        if self.prove(&ask(0, 0xA076_1D64), pos)? {
            Some(Value::Draw)
        } else {
            Some(Value::OWins)
        }
    }

    fn stats(&self) -> Stats {
        self.stats
    }

    fn reset(&mut self) {
        self.stats = Stats::default();
        self.tt.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AlphaBeta;
    use ulttt_game::random::Rng;

    #[test]
    fn agrees_with_alphabeta() {
        let mut rng = Rng::new(9);
        let mut dfpn = DfPn::new(18, 82);
        let mut dfpn_sym = DfPn::new(18, 0).with_tt_min_empties(8);
        for i in 0..90 {
            let v = Variant::ALL[i % 3];
            let Some(pos) = rng.position(v, 52) else {
                continue;
            };
            let want = AlphaBeta::new().solve(&pos, v);
            dfpn.reset();
            assert_eq!(dfpn.solve(&pos, v), want, "{v} {pos}");
            dfpn_sym.reset();
            assert_eq!(dfpn_sym.solve(&pos, v), want, "symmetry keys: {v} {pos}");
        }
    }
}
