//! Seedable randomness for random games, benchmark suites and tests.

use crate::position::{Move, Moves, Position};
use crate::types::Variant;

/// SplitMix64: a small, fast, seedable generator. Not for cryptography.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..n`. Requires `n > 0`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    /// A uniformly random legal move, or `None` if the game is over.
    pub fn legal_move(&mut self, pos: &Position, v: Variant) -> Option<Move> {
        let legal = pos.legal_moves(v);
        let n = legal.count_ones() as usize;
        (n > 0).then(|| Moves(legal).nth(self.below(n)).expect("n moves"))
    }

    /// A random game played to the end, as its list of moves.
    pub fn game(&mut self, v: Variant) -> Vec<Move> {
        let mut pos = Position::new();
        let mut moves = Vec::new();
        while let Some(m) = self.legal_move(&pos, v) {
            pos.play(m);
            moves.push(m);
        }
        moves
    }

    /// The position after `plies` random moves, if the game lasts that long.
    pub fn position(&mut self, v: Variant, plies: usize) -> Option<Position> {
        let mut pos = Position::new();
        for _ in 0..plies {
            let m = self.legal_move(&pos, v)?;
            pos.play(m);
        }
        (!pos.is_terminal(v)).then_some(pos)
    }
}
