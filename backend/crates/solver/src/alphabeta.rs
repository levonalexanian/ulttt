use ulttt_game::{Moves, Position, Variant};

use crate::{Solver, Stats, Value};

/// Plain negamax with alpha-beta pruning: the baseline every other solver is
/// compared against.
pub struct AlphaBeta {
    stats: Stats,
    node_limit: u64,
    aborted: bool,
}

impl Default for AlphaBeta {
    fn default() -> Self {
        AlphaBeta {
            stats: Stats::default(),
            node_limit: u64::MAX,
            aborted: false,
        }
    }
}

impl AlphaBeta {
    pub fn new() -> Self {
        Self::default()
    }

    /// The score of `pos` for the player to move, in `-1..=1`, searched in the
    /// window `(alpha, beta)`.
    fn search(&mut self, pos: &Position, v: Variant, mut alpha: i8, beta: i8) -> i8 {
        self.stats.nodes += 1;
        if self.stats.nodes > self.node_limit {
            self.aborted = true;
            return 0;
        }
        if let Some(outcome) = pos.result(v) {
            let x = outcome.score();
            return if pos.to_move() == ulttt_game::Player::X {
                x
            } else {
                -x
            };
        }
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
        best
    }
}

impl Solver for AlphaBeta {
    fn name(&self) -> String {
        "alphabeta".into()
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ulttt_game::mv;

    #[test]
    fn finished_game() {
        // X wins boards 0, 1, 2 along the top row of the global board.
        let pos: Position = format!("{}/{}/o/-", "x".repeat(27) + &".".repeat(54), "xxx......")
            .parse()
            .unwrap();
        assert_eq!(
            AlphaBeta::new().solve(&pos, Variant::Classic),
            Some(Value::XWins)
        );
    }

    #[test]
    fn win_in_one() {
        // X owns boards 0 and 1 and has two in a row on board 2, and is sent there.
        let mut cells = vec!['.'; 81];
        for m in [
            mv(0, 0),
            mv(0, 1),
            mv(0, 2),
            mv(1, 0),
            mv(1, 1),
            mv(1, 2),
            mv(2, 0),
            mv(2, 1),
        ] {
            cells[m as usize] = 'x';
        }
        for m in [
            mv(5, 0),
            mv(5, 1),
            mv(5, 2),
            mv(6, 0),
            mv(6, 1),
            mv(6, 2),
            mv(7, 0),
            mv(7, 1),
        ] {
            cells[m as usize] = 'o';
        }
        let s: String = cells.into_iter().collect();
        let pos: Position = format!("{s}/xx...oo../x/2").parse().unwrap();
        for v in Variant::ALL {
            assert_eq!(AlphaBeta::new().solve(&pos, v), Some(Value::XWins), "{v}");
        }
    }
}
