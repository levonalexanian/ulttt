//! A heuristic evaluation of positions the search cannot see to the end.
//!
//! Each local board gets, for each player, a rough chance of being won by that
//! player: 1 if they own it, 0 if they cannot win it any more, and otherwise a
//! share based on the open lines on it and how far along they are. The global
//! picture combines these the same way: every global line is worth the product
//! of its three boards' chances, so two owned boards in a line with a likely
//! third are worth almost a win, and a line with a board the player cannot win
//! is worth nothing.

use ulttt_game::lines::{FULL, LINES, has_line};
use ulttt_game::{Player, Position, Variant};

use crate::options::Weights;

/// Scores at or above this (in absolute value) are forced results, not guesses.
pub const WIN: i32 = 1_000_000;

/// The largest heuristic score; far below [`WIN`].
pub const MAX_EVAL: i32 = 100_000;

const fn mask(line: [u8; 3]) -> u16 {
    (1 << line[0]) | (1 << line[1]) | (1 << line[2])
}

const LINE_MASKS: [u16; 8] = {
    let mut m = [0; 8];
    let mut i = 0;
    while i < 8 {
        m[i] = mask(LINES[i]);
        i += 1;
    }
    m
};

/// The potential of `own` on a local board where the opponent holds `opp`: the
/// open lines (no opponent mark), weighted by how many marks they already have.
fn potential(own: u16, opp: u16, w: &Weights) -> f32 {
    let open_line = [1.0, w.line1, w.line2];
    LINE_MASKS
        .iter()
        .filter(|&&l| l & opp == 0)
        .map(|&l| open_line[(l & own).count_ones() as usize])
        .sum()
}

/// The chances of X and O on an unowned local board holding `x` and `o`.
fn unowned_chances(x: u16, o: u16, w: &Weights) -> (f32, f32) {
    // Overlapping or completed patterns never occur on an unowned board.
    if x & o != 0 || x | o == FULL || has_line(x) || has_line(o) {
        return (0.0, 0.0);
    }
    let (px, po) = (potential(x, o, w), potential(o, x, w));
    // Even an uncontested board is not won yet, and it gets harder to win as
    // it fills up.
    let free = (FULL & !(x | o)).count_ones() as f32 / 9.0;
    let total = px + po + w.prior;
    let scale = w.base + w.fill * (1.0 - free);
    (scale * px / total, scale * po / total)
}

/// Whether the player to move can win the game with their next move, by
/// taking a local board that completes a global line.
pub fn wins_now(pos: &Position, v: Variant) -> bool {
    let me = pos.to_move();
    let owned = pos.owned_by(me);
    let boards = pos.allowed_boards(v) & !(pos.owned_by(Player::X) | pos.owned_by(Player::O));
    (0..9u8)
        .any(|b| boards >> b & 1 != 0 && has_line(owned | 1 << b) && pos.local_threats(me, b) != 0)
}

/// The heuristic evaluation with its weights, and every unowned local board's
/// chances precomputed (indexed by X's cells, then O's shifted by 9).
pub struct Evaluator {
    w: Weights,
    /// How much each board matters on its own (center, corners, edges).
    board_weight: [f32; 9],
    chances: Vec<(f32, f32)>,
}

impl Evaluator {
    pub fn new(w: Weights) -> Evaluator {
        let chances = (0..1u32 << 18)
            .map(|i| unowned_chances(i as u16 & FULL, (i >> 9) as u16 & FULL, &w))
            .collect();
        let (c, k, e) = (w.corner, w.center, w.edge);
        let board_weight = [c, e, c, e, k, e, c, e, c];
        Evaluator {
            w,
            board_weight,
            chances,
        }
    }

    /// Each player's chance (X, O) of winning board `b`, in `0..=1`.
    fn board_chances(&self, pos: &Position, b: u8) -> (f32, f32) {
        match pos.owner(b) {
            Some(Player::X) => (1.0, 0.0),
            Some(Player::O) => (0.0, 1.0),
            None => {
                let x = pos.local_cells(Player::X, b) as usize;
                let o = pos.local_cells(Player::O, b) as usize;
                self.chances[x | o << 9]
            }
        }
    }

    /// The heuristic value of a position that is not over, for the player to
    /// move.
    pub fn evaluate(&self, pos: &Position, v: Variant) -> i32 {
        let w = &self.w;
        let mut cx = [0.0f32; 9];
        let mut co = [0.0f32; 9];
        let mut local = 0.0;
        for b in 0..9u8 {
            let (x, o) = self.board_chances(pos, b);
            cx[b as usize] = x;
            co[b as usize] = o;
            local += self.board_weight[b as usize] * (x - o);
        }
        let global = if w.union > 0.0 {
            // The chance that at least one line comes through, treating
            // lines as independent.
            let (mut miss_x, mut miss_o) = (1.0, 1.0);
            for line in LINES {
                let [a, b, c] = line.map(usize::from);
                miss_x *= 1.0 - cx[a] * cx[b] * cx[c];
                miss_o *= 1.0 - co[a] * co[b] * co[c];
            }
            miss_o - miss_x
        } else {
            let mut sum = 0.0;
            for line in LINES {
                let [a, b, c] = line.map(usize::from);
                sum += cx[a] * cx[b] * cx[c] - co[a] * co[b] * co[c];
            }
            sum
        };
        let mut score = w.global * global + w.local * local;
        if w.sent != 0.0
            && let Some(b) = pos.sent().filter(|&b| pos.allowed_boards(v) == 1 << b)
            && pos.owner(b).is_none()
            && pos.local_threats(pos.to_move(), b) != 0
        {
            // The player to move is sent to a board they can take at once.
            score += if pos.to_move() == Player::X {
                w.sent
            } else {
                -w.sent
            };
        }
        if v == Variant::Majority {
            // Boards count even without a line.
            score += w.majority * (cx.iter().sum::<f32>() - co.iter().sum::<f32>());
        }
        // Choosing any board is worth something.
        let free_choice = pos.allowed_boards(v).count_ones() > 1;
        let tempo = if free_choice { w.tempo } else { 0.0 };
        let for_x = (score as i32).clamp(-MAX_EVAL, MAX_EVAL);
        let s = if pos.to_move() == Player::X {
            for_x
        } else {
            -for_x
        };
        s + tempo as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_is_balanced() {
        let pos = Position::new();
        for v in Variant::ALL {
            // Only the tempo of a free choice.
            assert_eq!(Evaluator::new(Weights::default()).evaluate(&pos, v), 300);
        }
    }

    #[test]
    fn owning_boards_is_good() {
        // X has won board 4 (cells 36, 40, 44); O has scattered marks.
        let mut cells = ['.'; 81];
        for m in [36, 40, 44] {
            cells[m] = 'x';
        }
        for m in [0, 9] {
            cells[m] = 'o';
        }
        let cells: String = cells.iter().collect();
        let pos: Position = format!("{cells}/....x..../o/8").parse().unwrap();
        assert_eq!(pos.owner(4), Some(Player::X));
        // O to move: bad for O.
        assert!(Evaluator::new(Weights::default()).evaluate(&pos, Variant::Classic) < -1000);
    }

    #[test]
    fn win_in_one() {
        let mut cells = ['.'; 81];
        for m in [0, 1, 2, 9, 10, 11, 18, 19] {
            cells[m] = 'x';
        }
        for m in [45, 46, 47, 54, 55, 56, 63, 64] {
            cells[m] = 'o';
        }
        let cells: String = cells.iter().collect();
        let pos: Position = format!("{cells}/xx...oo../x/2").parse().unwrap();
        assert!(wins_now(&pos, Variant::Classic));
        // Sent elsewhere, X cannot.
        let pos: Position = format!("{cells}/xx...oo../x/3").parse().unwrap();
        assert!(!wins_now(&pos, Variant::Classic));
    }
}
