//! Iterative-deepening alpha-beta over the heuristic evaluation.

use ulttt_game::{Move, Moves, Outcome, Player, Position, Variant, board_of, cell_of};
use ulttt_solver::{Options as ExactOptions, Search, Solver, Value};

use crate::eval::{Evaluator, WIN, wins_now};
use crate::options::Options;

/// Scores this close to [`WIN`] are forced results found by the search.
const WIN_THRESHOLD: i32 = WIN - 1000;

/// The move an engine picked and what it knows about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Choice {
    pub mv: Move,
    /// The heuristic score after the move, for the player who made it. Scores
    /// beyond `WIN - 1000` in absolute value are forced wins or losses.
    pub score: i32,
    /// Depth of the last completed search iteration.
    pub depth: u32,
    /// The exact value of the position before the move, if the solver found it.
    pub value: Option<Value>,
    /// Positions visited, by both searches.
    pub nodes: u64,
}

#[derive(Clone, Copy, Default)]
struct Entry {
    key: u64,
    score: i32,
    depth: u8,
    bound: u8,
    best: u8,
    /// The search (move) that stored the entry.
    age: u8,
}

const EXACT: u8 = 0;
const LOWER: u8 = 1;
const UPPER: u8 = 2;
const NO_MOVE: u8 = 255;

/// Moves in search order, on the stack.
/// Scored moves, handed out best first (ties: lowest move). Picking lazily
/// beats sorting, since most nodes stop after a move or two.
struct MoveList {
    scored: [(i32, Move); 81],
    len: usize,
    next: usize,
}

impl Iterator for MoveList {
    type Item = Move;

    fn next(&mut self) -> Option<Move> {
        if self.next >= self.len {
            return None;
        }
        let mut best = self.next;
        for i in self.next + 1..self.len {
            let (s, m) = self.scored[i];
            let (bs, bm) = self.scored[best];
            if s > bs || (s == bs && m < bm) {
                best = i;
            }
        }
        self.scored.swap(self.next, best);
        self.next += 1;
        Some(self.scored[self.next - 1].1)
    }
}

/// A playing engine for one variant. Keeps its tables between moves.
pub struct Engine {
    variant: Variant,
    table: Vec<Entry>,
    history: [[i32; 81]; 2],
    /// Two moves per ply that recently caused a cutoff there.
    killers: [[u8; 2]; 82],
    /// Counts searches, to tell old table entries from current ones.
    generation: u8,
    nodes: u64,
    limit: u64,
    aborted: bool,
    exact: Search,
    opts: Options,
    eval: Evaluator,
    clock: Option<Box<dyn Fn() -> f32>>,
}

impl Engine {
    pub fn new(variant: Variant, opts: Options) -> Engine {
        Engine {
            variant,
            table: vec![Entry::default(); 1 << opts.tt_bits],
            history: [[0; 81]; 2],
            killers: [[NO_MOVE; 2]; 82],
            generation: 0,
            nodes: 0,
            limit: 0,
            aborted: false,
            exact: Search::new(ExactOptions {
                tt_bits: opts.exact_tt_bits,
                ..ExactOptions::default()
            }),
            eval: Evaluator::new(opts.weights),
            opts,
            clock: None,
        }
    }

    /// A clock for each move: the fraction of the move's time used so far. The
    /// search ends when it reaches 1 (checked every 1024 positions), the exact
    /// solver gets its share of the time as of the node limit.
    pub fn set_clock(&mut self, clock: Option<Box<dyn Fn() -> f32>>) {
        self.clock = clock;
    }

    fn time_used(&self) -> f32 {
        self.clock.as_ref().map_or(0.0, |c| c())
    }

    /// Solves `pos` exactly within `nodes` positions and, with a clock, the
    /// fraction `time` of the move's time: in growing chunks, which the
    /// solver's table makes cheap to resume.
    fn solve_within(&mut self, pos: &Position, nodes: u64, time: f32) -> Option<Value> {
        let start = self.exact.stats().nodes;
        let mut chunk = if self.clock.is_some() {
            nodes.min(4096)
        } else {
            nodes
        };
        loop {
            self.exact.set_node_limit(Some(chunk.max(1)));
            if let Some(v) = self.exact.solve(pos, self.variant) {
                return Some(v);
            }
            let used = self.exact.stats().nodes - start;
            if used >= nodes || self.time_used() >= time {
                return None;
            }
            chunk = (2 * chunk).min(nodes - used);
        }
    }

    pub fn options(&self) -> &Options {
        &self.opts
    }

    pub fn variant(&self) -> Variant {
        self.variant
    }

    /// Picks a move for the player to move, visiting about `node_limit`
    /// positions. `None` if the game is over.
    pub fn best_move(&mut self, pos: &Position, node_limit: u64) -> Option<Choice> {
        let v = self.variant;
        let legal = pos.legal_moves(v);
        if legal == 0 {
            return None;
        }
        let me = pos.to_move();
        let mut candidates = legal;
        let mut value = None;
        let mut exact_nodes = 0;
        if pos.empties() <= self.opts.exact_max_empties {
            let before = self.exact.stats().nodes;
            let share = node_limit / self.opts.exact_share;
            let time_share = 1.0 / self.opts.exact_share as f32;
            value = self.solve_within(pos, share, time_share);
            if let Some(val) = value {
                let target = for_player(val, me);
                if target >= 0 {
                    // Keep only the moves that keep the value. Their solves are
                    // usually cheap now that the table holds the root's proof;
                    // all of them together get the same share again.
                    let mut keep = 0;
                    for m in Moves(legal) {
                        let used = self.exact.stats().nodes - before;
                        let Some(left) = (2 * share).checked_sub(used).filter(|&n| n > 0) else {
                            break;
                        };
                        if self
                            .solve_within(&pos.after(m), left, 2.0 * time_share)
                            .map(|c| for_player(c, me))
                            == Some(target)
                        {
                            keep |= 1 << m;
                        }
                    }
                    if keep != 0 {
                        candidates = keep;
                    }
                }
            }
            exact_nodes = self.exact.stats().nodes - before;
        }
        let budget = node_limit.saturating_sub(exact_nodes).max(1);
        let (mv, score, depth) = self.search_root(pos, candidates, budget);
        Some(Choice {
            mv,
            score,
            depth,
            value,
            nodes: self.nodes + exact_nodes,
        })
    }

    /// Iterative deepening over the root moves in `candidates`.
    fn search_root(&mut self, pos: &Position, candidates: u128, limit: u64) -> (Move, i32, u32) {
        self.nodes = 0;
        self.limit = limit;
        self.generation = self.generation.wrapping_add(1);
        self.aborted = false;
        for h in self.history.iter_mut().flatten() {
            *h /= 8;
        }
        let mut moves: Vec<Move> = self.ordered(pos, candidates, NO_MOVE, 0).collect();
        let mut best: (Move, i32, u32) = (moves[0], 0, 0);
        if moves.len() == 1 {
            // Still worth a score for the caller; a shallow search is enough.
            self.limit = limit.min(10_000);
        }
        for depth in 1..=pos.empties() {
            // Aspiration window: expect a score near the last iteration's and
            // search the full window again only if it falls outside.
            let window = self.opts.aspiration as i32;
            let narrow = window > 0 && depth >= 4 && best.1.abs() < WIN_THRESHOLD;
            let mut bounds = if narrow {
                (best.1 - window, best.1 + window)
            } else {
                (-WIN - 1, WIN + 1)
            };
            let mut iteration_best;
            loop {
                let (mut alpha, beta) = bounds;
                iteration_best = None;
                for (i, &m) in moves.iter().enumerate() {
                    let child = pos.after(m);
                    let s = -self.negamax(&child, depth - 1, -beta, -alpha, 1);
                    if self.aborted {
                        break;
                    }
                    if i == 0 || s > alpha {
                        alpha = s;
                        iteration_best = Some((m, s));
                    }
                }
                let failed = iteration_best.is_some_and(|(_, s)| s <= bounds.0 || s >= bounds.1);
                if self.aborted || !failed || bounds == (-WIN - 1, WIN + 1) {
                    break;
                }
                bounds = (-WIN - 1, WIN + 1);
            }
            if self.aborted {
                // A partial iteration still counts if its best move was
                // searched fully and beat the previous best, searched first.
                if let Some((m, s)) = iteration_best
                    && m != best.0
                {
                    best = (m, s, best.2);
                }
                break;
            }
            let (m, s) = iteration_best.expect("some move");
            best = (m, s, depth);
            // Search the best move first next time.
            moves.retain(|&x| x != m);
            moves.insert(0, m);
            if s.abs() >= WIN_THRESHOLD {
                break;
            }
        }
        best
    }

    /// `candidates` in the order to search them: the table's move, then local
    /// wins and blocks, avoiding sending the opponent where they want to go.
    fn ordered(&self, pos: &Position, candidates: u128, tt_move: u8, ply: usize) -> MoveList {
        let killers = if self.opts.killers {
            self.killers[ply.min(81)]
        } else {
            [NO_MOVE; 2]
        };
        let v = self.variant;
        let me = pos.to_move();
        let closed = pos.closed(v);
        let owned = pos.owned_by(Player::X) | pos.owned_by(Player::O);
        // Each board's completing cells for both players, computed once.
        let mut mine = [0u16; 9];
        let mut theirs = [0u16; 9];
        for b in 0..9u8 {
            if owned >> b & 1 == 0 {
                mine[b as usize] = pos.local_threats(me, b);
                theirs[b as usize] = pos.local_threats(me.other(), b);
            }
        }
        let history = &self.history[me.index()];
        let mut scored = [(0i32, 0u8); 81];
        let mut n = 0;
        for m in Moves(candidates) {
            let (b, c) = (board_of(m) as usize, cell_of(m));
            let mut s = history[m as usize];
            if m == tt_move {
                s += 1 << 30;
            } else if m == killers[0] || m == killers[1] {
                s += 1 << 20;
            }
            let wins_board = mine[b] >> c & 1 != 0;
            if wins_board {
                s += 1 << 24;
            } else if theirs[b] >> c & 1 != 0 {
                s += 1 << 22;
            }
            // Where the opponent is sent: anywhere if that board is closed (or
            // this move closes it), or a board they can win right away.
            let closes = c as usize == b && wins_board;
            if closed >> c & 1 != 0 || closes {
                s -= 1 << 23;
            } else if theirs[c as usize] != 0 {
                s -= 1 << 21;
            }
            scored[n] = (s, m);
            n += 1;
        }
        MoveList {
            scored,
            len: n,
            next: 0,
        }
    }

    /// Past the horizon, keep following moves that win a local board (up to
    /// `left` of them), so the evaluation does not stop in the middle of a
    /// capture sequence. The player to move may also stop and take the
    /// static evaluation.
    fn quiesce(&mut self, pos: &Position, mut alpha: i32, beta: i32, ply: i32, left: u32) -> i32 {
        let v = self.variant;
        let stand = self.eval.evaluate(pos, v);
        if left == 0 || stand >= beta {
            return stand;
        }
        alpha = alpha.max(stand);
        let me = pos.to_move();
        let owned = pos.owned_by(Player::X) | pos.owned_by(Player::O);
        let boards = pos.allowed_boards(v) & !owned;
        let mut captures = 0u128;
        for b in 0..9u8 {
            if boards >> b & 1 != 0 {
                captures |= u128::from(pos.local_threats(me, b)) << (9 * b);
            }
        }
        let mut best = stand;
        for m in Moves(captures) {
            self.nodes += 1;
            if self.nodes >= self.limit {
                self.aborted = true;
                return 0;
            }
            let child = pos.after(m);
            let s = if let Some(o) = child.result(v) {
                match o {
                    Outcome::Win(p) if p == me => WIN - ply - 1,
                    Outcome::Win(_) => -(WIN - ply - 1),
                    Outcome::Draw => 0,
                }
            } else if wins_now(&child, v) {
                -(WIN - ply - 2)
            } else {
                -self.quiesce(&child, -beta, -alpha, ply + 1, left - 1)
            };
            if self.aborted {
                return 0;
            }
            if s > best {
                best = s;
                if s > alpha {
                    alpha = s;
                }
                if alpha >= beta {
                    break;
                }
            }
        }
        best
    }

    fn negamax(
        &mut self,
        pos: &Position,
        depth: u32,
        mut alpha: i32,
        mut beta: i32,
        ply: i32,
    ) -> i32 {
        self.nodes += 1;
        if self.nodes >= self.limit || (self.nodes.is_multiple_of(1024) && self.time_used() >= 1.0)
        {
            self.aborted = true;
            return 0;
        }
        let v = self.variant;
        let me = pos.to_move();
        if let Some(o) = pos.result(v) {
            return match o {
                Outcome::Win(p) if p == me => WIN - ply,
                Outcome::Win(_) => -(WIN - ply),
                Outcome::Draw => 0,
            };
        }
        // A player who can no longer win does no better than a draw.
        let (i_may, they_may) = (pos.may_win(v, me), pos.may_win(v, me.other()));
        if !i_may && !they_may {
            return 0;
        }
        if !i_may && alpha >= 0 {
            return 0;
        }
        if !they_may && beta <= 0 {
            return 0;
        }
        if depth == 0 {
            if self.opts.leaf_wins && wins_now(pos, v) {
                return WIN - ply - 1;
            }
            if self.opts.quiescence > 0 {
                return self.quiesce(pos, alpha, beta, ply, self.opts.quiescence);
            }
            return self.eval.evaluate(pos, v);
        }
        let key = pos.hash64();
        let mut slot = (key as usize) & (self.table.len() - 1);
        if self.opts.tt_buckets && self.table[slot].key != key && self.table[slot ^ 1].key == key {
            slot ^= 1;
        }
        let e = self.table[slot];
        let mut tt_move = NO_MOVE;
        if e.key == key {
            tt_move = e.best;
            if u32::from(e.depth) >= depth {
                let s = from_table(e.score, ply);
                match e.bound {
                    EXACT => return s,
                    LOWER => alpha = alpha.max(s),
                    _ => beta = beta.min(s),
                }
                if alpha >= beta {
                    return s;
                }
            }
        }
        let original_alpha = alpha;
        let mut best = -WIN - 1;
        let mut best_move = NO_MOVE;
        let list = self.ordered(pos, pos.legal_moves(v), tt_move, ply as usize);
        for (i, m) in list.enumerate() {
            let child = pos.after(m);
            let s = if i == 0 || !self.opts.pvs {
                -self.negamax(&child, depth - 1, -beta, -alpha, ply + 1)
            } else {
                // Late move reductions: a move this far down the order is
                // probably bad, so first check that at reduced depth.
                let (b, c) = (board_of(m), cell_of(m));
                let reduce = self.opts.lmr > 0
                    && i >= self.opts.lmr as usize
                    && depth >= 3
                    && m != tt_move
                    && !(pos.owner(b).is_none() && pos.local_threats(me, b) >> c & 1 != 0);
                if reduce {
                    let s = -self.negamax(&child, depth - 2, -alpha - 1, -alpha, ply + 1);
                    if self.aborted {
                        return 0;
                    }
                    if s <= alpha {
                        if s > best {
                            best = s;
                            best_move = m;
                        }
                        continue;
                    }
                }
                // Principal variation search: prove the move no better than
                // the best so far with a null window, re-search if it is.
                let s = -self.negamax(&child, depth - 1, -alpha - 1, -alpha, ply + 1);
                if s > alpha && s < beta && !self.aborted {
                    -self.negamax(&child, depth - 1, -beta, -alpha, ply + 1)
                } else {
                    s
                }
            };
            if self.aborted {
                return 0;
            }
            if s > best {
                best = s;
                best_move = m;
            }
            if s > alpha {
                alpha = s;
            }
            if alpha >= beta {
                self.history[me.index()][m as usize] += (depth * depth) as i32;
                let k = &mut self.killers[(ply as usize).min(81)];
                if k[0] != m {
                    k[1] = k[0];
                    k[0] = m;
                }
                break;
            }
        }
        let bound = if best <= original_alpha {
            UPPER
        } else if best >= beta {
            LOWER
        } else {
            EXACT
        };
        if self.opts.tt_buckets && e.key != key {
            // Two entries per bucket: the first keeps the deepest current
            // search, the second takes whatever it displaces.
            let first = slot & !1;
            let f = self.table[first];
            slot = if f.key == key || f.age != self.generation || u32::from(f.depth) <= depth {
                first
            } else {
                first | 1
            };
        }
        self.table[slot] = Entry {
            key,
            age: self.generation,
            score: to_table(best, ply),
            depth: depth.min(255) as u8,
            bound,
            best: best_move,
        };
        best
    }
}

/// Forced-result scores count plies from the root; the table stores them
/// counted from the position instead, so they stay right from other paths.
fn to_table(s: i32, ply: i32) -> i32 {
    if s >= WIN_THRESHOLD {
        s + ply
    } else if s <= -WIN_THRESHOLD {
        s - ply
    } else {
        s
    }
}

fn from_table(s: i32, ply: i32) -> i32 {
    if s >= WIN_THRESHOLD {
        s - ply
    } else if s <= -WIN_THRESHOLD {
        s + ply
    } else {
        s
    }
}

/// A value from player `p`'s point of view: 1 win, 0 draw, -1 loss.
fn for_player(v: Value, p: Player) -> i8 {
    let x = v as i8;
    if p == Player::X { x } else { -x }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ulttt_game::random::Rng;

    fn small(bits: u32) -> Options {
        Options {
            tt_bits: bits,
            exact_tt_bits: bits,
            ..Options::default()
        }
    }

    fn pos(x: &[usize], o: &[usize], rest: &str) -> Position {
        let mut cells = ['.'; 81];
        for &m in x {
            cells[m] = 'x';
        }
        for &m in o {
            cells[m] = 'o';
        }
        let cells: String = cells.iter().collect();
        format!("{cells}/{rest}").parse().unwrap()
    }

    #[test]
    fn takes_a_global_win() {
        // X owns boards 0 and 1, has two in a row on board 2 and is sent there.
        let p = pos(
            &[0, 1, 2, 9, 10, 11, 18, 19],
            &[45, 46, 47, 54, 55, 56, 63, 64],
            "xx...oo../x/2",
        );
        let mut e = Engine::new(Variant::Classic, small(12));
        let c = e.best_move(&p, 20_000).unwrap();
        assert_eq!(c.mv, 20);
        assert!(c.score >= WIN_THRESHOLD);
    }

    #[test]
    fn heuristic_alone_takes_a_global_win() {
        // Same, too early for the exact solver: the search finds it at depth 1.
        let p = pos(
            &[0, 1, 2, 9, 10, 11, 18, 19],
            &[45, 46, 47, 54, 55, 56, 63, 64],
            "xx...oo../x/2",
        );
        let mut e = Engine::new(Variant::Classic, small(12));
        let (m, s, _) = e.search_root(&p, p.legal_moves(Variant::Classic), 20_000);
        assert_eq!(m, 20);
        assert!(s >= WIN_THRESHOLD);
    }

    /// Plays a game between the engine (as `engine_side`) and random moves.
    fn game_vs_random(v: Variant, engine_side: Player, seed: u64, nodes: u64) -> Option<Outcome> {
        let mut rng = Rng::new(seed);
        let mut e = Engine::new(v, small(14));
        let mut p = Position::new();
        while !p.is_terminal(v) {
            let m = if p.to_move() == engine_side {
                e.best_move(&p, nodes).unwrap().mv
            } else {
                let moves: Vec<Move> = Moves(p.legal_moves(v)).collect();
                moves[rng.below(moves.len())]
            };
            p.play(m);
        }
        p.result(v)
    }

    #[test]
    fn beats_random_play() {
        for v in Variant::ALL {
            for seed in 0..4 {
                for side in [Player::X, Player::O] {
                    let r = game_vs_random(v, side, seed, 3_000);
                    assert_eq!(r, Some(Outcome::Win(side)), "{v:?} seed {seed} as {side:?}");
                }
            }
        }
    }

    #[test]
    fn keeps_proven_wins() {
        // In a solved position the engine only plays value-keeping moves.
        let mut rng = Rng::new(9);
        let mut solver = Search::new(ExactOptions {
            tt_bits: 16,
            ..ExactOptions::default()
        });
        let mut tested = 0;
        while tested < 8 {
            let v = Variant::ALL[tested % 3];
            let Some(p) = rng.position(v, 56) else {
                continue;
            };
            let value = solver.solve(&p, v).unwrap();
            let mut e = Engine::new(
                v,
                Options {
                    exact_tt_bits: 16,
                    ..small(14)
                },
            );
            let c = e.best_move(&p, 2_000_000).unwrap();
            assert_eq!(c.value, Some(value));
            let after = solver.solve(&p.after(c.mv), v).unwrap();
            if for_player(value, p.to_move()) >= 0 {
                assert_eq!(after, value);
            }
            tested += 1;
        }
    }
}
