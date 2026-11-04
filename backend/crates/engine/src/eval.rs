//! A heuristic evaluation of positions the search cannot see to the end.
//!
//! Each local board gets, for each player, a rough chance of being won by that
//! player: 1 if they own it, 0 if they cannot win it any more, and otherwise a
//! share based on the open lines on it and how far along they are. The global
//! picture combines these the same way: every global line is worth the product
//! of its three boards' chances, so two owned boards in a line with a likely
//! third are worth almost a win, and a line with a board the player cannot win
//! is worth nothing.

use ulttt_game::lines::{FULL, LINES};
use ulttt_game::{Player, Position, Variant};

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

/// How much each board matters on its own: the center is on four global lines,
/// corners on three, edges on two.
const BOARD_WEIGHT: [f32; 9] = [3.0, 2.0, 3.0, 2.0, 4.0, 2.0, 3.0, 2.0, 3.0];

/// Weight of an open local line holding 0, 1 or 2 of the player's marks.
const OPEN_LINE: [f32; 3] = [1.0, 3.0, 10.0];

/// The potential of `own` on a local board where the opponent holds `opp`: the
/// open lines (no opponent mark), weighted by how many marks they already have.
fn potential(own: u16, opp: u16) -> f32 {
    LINE_MASKS
        .iter()
        .filter(|&&l| l & opp == 0)
        .map(|&l| OPEN_LINE[(l & own).count_ones() as usize])
        .sum()
}

/// Each player's chance (X, O) of winning board `b`, in `0..=1`.
fn board_chances(pos: &Position, b: u8) -> (f32, f32) {
    match pos.owner(b) {
        Some(Player::X) => (1.0, 0.0),
        Some(Player::O) => (0.0, 1.0),
        None if pos.full_boards() >> b & 1 != 0 => (0.0, 0.0),
        None => {
            let (x, o) = (pos.local_cells(Player::X, b), pos.local_cells(Player::O, b));
            let (px, po) = (potential(x, o), potential(o, x));
            // Even an uncontested board is not won yet, and it gets harder to
            // win as it fills up.
            let free = (FULL & !(x | o)).count_ones() as f32 / 9.0;
            let total = px + po + 4.0;
            let scale = 0.5 + 0.4 * (1.0 - free);
            (scale * px / total, scale * po / total)
        }
    }
}

/// The heuristic value of a position that is not over, for the player to move.
pub fn evaluate(pos: &Position, v: Variant) -> i32 {
    let mut cx = [0.0f32; 9];
    let mut co = [0.0f32; 9];
    let mut local = 0.0;
    for b in 0..9u8 {
        let (x, o) = board_chances(pos, b);
        cx[b as usize] = x;
        co[b as usize] = o;
        local += BOARD_WEIGHT[b as usize] * (x - o);
    }
    let mut global = 0.0;
    for line in LINES {
        let [a, b, c] = line.map(usize::from);
        global += cx[a] * cx[b] * cx[c] - co[a] * co[b] * co[c];
    }
    let mut score = 6000.0 * global + 400.0 * local;
    if v == Variant::Majority {
        // Boards count even without a line.
        score += 1500.0 * (cx.iter().sum::<f32>() - co.iter().sum::<f32>());
    }
    // Choosing any board is worth something.
    let free_choice = pos.allowed_boards(v).count_ones() > 1;
    let tempo = if free_choice { 300.0 } else { 0.0 };
    let for_x = (score as i32).clamp(-MAX_EVAL, MAX_EVAL);
    let s = if pos.to_move() == Player::X {
        for_x
    } else {
        -for_x
    };
    s + tempo as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_is_balanced() {
        let pos = Position::new();
        for v in Variant::ALL {
            // Only the tempo of a free choice.
            assert_eq!(evaluate(&pos, v), 300);
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
        assert!(evaluate(&pos, Variant::Classic) < -1000);
    }
}
