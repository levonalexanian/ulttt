//! Monte Carlo tree search: an alternative to the alpha-beta engine.
//!
//! Each iteration walks down the tree by UCT (the upper confidence bound on a
//! move's win rate), adds one new move to the tree, plays the rest of the game
//! at random from there and counts the result for every move on the path. The
//! tree also keeps proven results (MCTS-solver): a move into a position the
//! mover has already won is a proven win, a position where the player to move
//! has a proven winning move is a proven loss for whoever moved into it, and
//! one where every move is a proven loss is a proven win.

use ulttt_game::random::Rng;
use ulttt_game::{Move, Outcome, Player, Position, Variant};

use crate::eval::{MAX_EVAL, WIN, wins_now};
use crate::search::Choice;

/// Settings of the MCTS engine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MctsOptions {
    /// The UCT exploration constant.
    pub c: f32,
    /// End playouts early when the player to move can win in one move.
    pub decisive: bool,
    /// Most tree nodes kept; past this, iterations stop growing the tree.
    pub max_nodes: usize,
    pub seed: u64,
}

impl Default for MctsOptions {
    fn default() -> Self {
        MctsOptions {
            c: 0.8,
            decisive: true,
            max_nodes: 1 << 21,
            seed: 1,
        }
    }
}

impl MctsOptions {
    /// Parses comma-separated `c`, `decisive` (0 or 1), `max` (tree nodes) and
    /// `seed` settings on top of the defaults.
    pub fn parse(spec: &str) -> Result<MctsOptions, String> {
        let mut o = MctsOptions::default();
        for part in spec.split(',').filter(|p| !p.is_empty()) {
            let (key, value) = part
                .split_once('=')
                .ok_or_else(|| format!("expected key=value, got {part:?}"))?;
            let bad = || format!("bad value in {part:?}");
            match key {
                "c" => o.c = value.parse().map_err(|_| bad())?,
                "decisive" => o.decisive = value.parse::<u8>().map_err(|_| bad())? != 0,
                "max" => o.max_nodes = value.parse().map_err(|_| bad())?,
                "seed" => o.seed = value.parse().map_err(|_| bad())?,
                _ => return Err(format!("unknown mcts option {key:?}")),
            }
        }
        Ok(o)
    }
}

const NONE: u32 = u32::MAX;

/// Proven results, for the player who moved into the node.
const WON: i8 = 1;
const LOST: i8 = -1;

#[derive(Clone, Copy)]
struct Node {
    /// Legal moves not yet added as children.
    untried: u128,
    first_child: u32,
    sibling: u32,
    visits: u32,
    /// Sum of results for the player who moved into this node (1 win, 0.5
    /// draw, 0 loss).
    value: f32,
    mv: Move,
    proven: i8,
    /// For a finished game: its result for the player who moved into it.
    terminal: Option<f32>,
}

pub struct Mcts {
    variant: Variant,
    opts: MctsOptions,
    nodes: Vec<Node>,
    rng: Rng,
    clock: Option<Box<dyn Fn() -> f32>>,
}

/// The `k`-th set bit (from 0) of `mask`, which has more than `k` bits set.
fn nth_bit(mask: u128, k: u32) -> Move {
    let lo = mask as u64;
    let n = lo.count_ones();
    let (mut x, mut k, base) = if k < n {
        (lo, k, 0)
    } else {
        ((mask >> 64) as u64, k - n, 64)
    };
    while k > 0 {
        x &= x - 1;
        k -= 1;
    }
    base + x.trailing_zeros() as Move
}

/// A result for `p`: 1 win, 0.5 draw, 0 loss.
fn result_for(o: Outcome, p: Player) -> f32 {
    match o {
        Outcome::Win(w) if w == p => 1.0,
        Outcome::Win(_) => 0.0,
        Outcome::Draw => 0.5,
    }
}

impl Mcts {
    pub fn new(variant: Variant, opts: MctsOptions) -> Mcts {
        Mcts {
            variant,
            opts,
            nodes: Vec::new(),
            rng: Rng::new(opts.seed),
            clock: None,
        }
    }

    pub fn variant(&self) -> Variant {
        self.variant
    }

    /// See [`crate::Engine::set_clock`].
    pub fn set_clock(&mut self, clock: Option<Box<dyn Fn() -> f32>>) {
        self.clock = clock;
    }

    fn new_node(&mut self, pos: &Position, mv: Move) -> u32 {
        let v = self.variant;
        let terminal = pos.result(v).map(|o| result_for(o, pos.to_move().other()));
        self.nodes.push(Node {
            untried: pos.legal_moves(v),
            first_child: NONE,
            sibling: NONE,
            visits: 0,
            value: 0.0,
            mv,
            proven: if terminal == Some(1.0) { WON } else { 0 },
            terminal,
        });
        (self.nodes.len() - 1) as u32
    }

    /// Plays at random to the end; the result for the player who moved last
    /// before `pos`.
    fn playout(&mut self, mut pos: Position, steps: &mut u64) -> f32 {
        let v = self.variant;
        let mover = pos.to_move().other();
        loop {
            if let Some(o) = pos.result(v) {
                return result_for(o, mover);
            }
            if self.opts.decisive && wins_now(&pos, v) {
                return if pos.to_move() == mover { 1.0 } else { 0.0 };
            }
            let legal = pos.legal_moves(v);
            let k = self.rng.below(legal.count_ones() as usize) as u32;
            pos.play(nth_bit(legal, k));
            *steps += 1;
        }
    }

    /// The child of `node` to explore by UCT, skipping proven losses for the
    /// mover unless nothing else is left.
    fn select(&self, node: u32) -> u32 {
        let n = &self.nodes[node as usize];
        let log_n = (n.visits.max(1) as f32).ln();
        let mut best = (f32::NEG_INFINITY, NONE);
        let mut child = n.first_child;
        while child != NONE {
            let c = &self.nodes[child as usize];
            let score = if c.proven == WON {
                f32::INFINITY
            } else if c.proven == LOST {
                -1.0
            } else {
                let visits = c.visits.max(1) as f32;
                c.value / visits + self.opts.c * (log_n / visits).sqrt()
            };
            if score > best.0 {
                best = (score, child);
            }
            child = c.sibling;
        }
        best.1
    }

    /// Updates `node`'s proven result from its children.
    fn prove(&mut self, node: u32) -> bool {
        let n = self.nodes[node as usize];
        if n.proven != 0 {
            return false;
        }
        let mut all_lost = n.untried == 0;
        let mut child = n.first_child;
        while child != NONE {
            let c = &self.nodes[child as usize];
            if c.proven == WON {
                self.nodes[node as usize].proven = LOST;
                return true;
            }
            all_lost &= c.proven == LOST;
            child = c.sibling;
        }
        if all_lost && n.first_child != NONE {
            self.nodes[node as usize].proven = WON;
            return true;
        }
        false
    }

    /// Runs iterations from `pos` for about `node_limit` positions and picks the
    /// most visited move (or a proven win). `None` if the game is over.
    pub fn best_move(&mut self, pos: &Position, node_limit: u64) -> Option<Choice> {
        let v = self.variant;
        if pos.is_terminal(v) {
            return None;
        }
        self.nodes.clear();
        self.new_node(pos, 0);
        let mut steps = 0u64;
        let mut path = Vec::with_capacity(81);
        let mut iterations = 0u64;
        while steps < node_limit && self.nodes[0].proven == 0 {
            iterations += 1;
            if iterations.is_multiple_of(256) && self.clock.as_ref().is_some_and(|c| c() >= 1.0) {
                break;
            }
            path.clear();
            path.push(0u32);
            let mut cur = *pos;
            let mut node = 0u32;
            let result = loop {
                steps += 1;
                let n = self.nodes[node as usize];
                if let Some(r) = n.terminal {
                    break r;
                }
                if n.proven != 0 {
                    break if n.proven == WON { 1.0 } else { 0.0 };
                }
                if n.untried != 0 && self.nodes.len() < self.opts.max_nodes {
                    let k = self.rng.below(n.untried.count_ones() as usize) as u32;
                    let m = nth_bit(n.untried, k);
                    cur.play(m);
                    let child = self.new_node(&cur, m);
                    let parent = &mut self.nodes[node as usize];
                    parent.untried &= !(1 << m);
                    self.nodes[child as usize].sibling = parent.first_child;
                    self.nodes[node as usize].first_child = child;
                    path.push(child);
                    break match self.nodes[child as usize].terminal {
                        Some(r) => r,
                        None => self.playout(cur, &mut steps),
                    };
                }
                if n.first_child == NONE {
                    // The tree is full: just play out from here.
                    break self.playout(cur, &mut steps);
                }
                node = self.select(node);
                cur.play(self.nodes[node as usize].mv);
                path.push(node);
            };
            // `result` is for the player who moved into the last node; flip it
            // at each step up.
            let mut r = result;
            let mut proving = true;
            for &id in path.iter().rev() {
                let n = &mut self.nodes[id as usize];
                n.visits += 1;
                n.value += r;
                r = 1.0 - r;
                if proving && id != *path.last().expect("path") {
                    proving = self.prove(id);
                }
            }
        }
        let root = self.nodes[0];
        let (mut best, mut best_key) = (NONE, (i8::MIN, 0u32));
        let mut child = root.first_child;
        while child != NONE {
            let c = &self.nodes[child as usize];
            let key = (c.proven, c.visits);
            // Prefer proven wins, then avoid proven losses, then visits.
            let key = (
                match key.0 {
                    WON => 2,
                    LOST => 0,
                    _ => 1,
                },
                key.1,
            );
            if key > best_key {
                best_key = key;
                best = child;
            }
            child = c.sibling;
        }
        let c = self.nodes[best as usize];
        let score = match c.proven {
            WON => WIN - 1,
            LOST => -(WIN - 1),
            _ => ((2.0 * c.value / c.visits.max(1) as f32 - 1.0) * MAX_EVAL as f32) as i32,
        };
        Some(Choice {
            mv: c.mv,
            score,
            depth: 0,
            value: None,
            nodes: steps,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nth_bit_finds_bits() {
        let mask: u128 = (1 << 3) | (1 << 70) | (1 << 80);
        assert_eq!(nth_bit(mask, 0), 3);
        assert_eq!(nth_bit(mask, 1), 70);
        assert_eq!(nth_bit(mask, 2), 80);
    }

    #[test]
    fn takes_a_global_win() {
        let mut cells = ['.'; 81];
        for m in [0, 1, 2, 9, 10, 11, 18, 19] {
            cells[m] = 'x';
        }
        for m in [45, 46, 47, 54, 55, 56, 63, 64] {
            cells[m] = 'o';
        }
        let cells: String = cells.iter().collect();
        let pos: Position = format!("{cells}/xx...oo../x/2").parse().unwrap();
        let mut m = Mcts::new(Variant::Classic, MctsOptions::default());
        let c = m.best_move(&pos, 50_000).unwrap();
        assert_eq!(c.mv, 20);
        assert!(c.score >= WIN - 1);
    }

    #[test]
    fn beats_random_play() {
        for v in Variant::ALL {
            for seed in 0..3 {
                let mut rng = Rng::new(seed);
                let mut m = Mcts::new(v, MctsOptions::default());
                let mut pos = Position::new();
                while !pos.is_terminal(v) {
                    let mv = if pos.to_move() == Player::X {
                        m.best_move(&pos, 20_000).unwrap().mv
                    } else {
                        rng.legal_move(&pos, v).unwrap()
                    };
                    pos.play(mv);
                }
                assert_eq!(pos.result(v), Some(Outcome::Win(Player::X)), "{v:?} {seed}");
            }
        }
    }
}
