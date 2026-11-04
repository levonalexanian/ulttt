//! Perft: counts the move sequences of a given length. Used to check move
//! generation and to benchmark it.

use crate::position::{Moves, Position};
use crate::types::Variant;

/// The number of distinct move sequences of exactly `depth` moves from `pos`.
/// Games that end sooner contribute nothing.
pub fn perft(pos: &Position, v: Variant, depth: u32) -> u64 {
    let legal = pos.legal_moves(v);
    match depth {
        0 => 1,
        1 => legal.count_ones() as u64,
        _ => Moves(legal)
            .map(|m| perft(&pos.after(m), v, depth - 1))
            .sum(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shallow_counts() {
        for v in Variant::ALL {
            let start = Position::new();
            assert_eq!(perft(&start, v, 0), 1);
            assert_eq!(perft(&start, v, 1), 81);
            // O is sent to a board with 9 empty cells, except after the 9 moves
            // (b, b), where that board already holds X's move.
            assert_eq!(perft(&start, v, 2), 81 * 9 - 9);
        }
    }

    /// Regression values. Move generation is checked against Lean by `ulttt-difftest`.
    #[test]
    fn depth_six() {
        let start = Position::new();
        let counts = [81, 720, 6336, 55080, 473256];
        for v in Variant::ALL {
            for (d, &n) in counts.iter().enumerate() {
                assert_eq!(perft(&start, v, d as u32 + 1), n);
            }
        }
        // X can first win a local board on move 5. `open` then has fewer replies,
        // since a won board stays playable and can still force the opponent.
        assert_eq!(perft(&start, Variant::Classic, 6), 4_020_960);
        assert_eq!(perft(&start, Variant::Majority, 6), 4_020_960);
        assert_eq!(perft(&start, Variant::Open, 6), 4_017_888);
    }
}
