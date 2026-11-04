//! The 8 symmetries of the 3×3 grid, applied to the global board and every
//! local board at once. Mirrors `proofs/Ulttt/Symmetry.lean`, where `value_map`
//! proves symmetric positions have the same value.

use crate::lines::FULL;
use crate::position::{Move, Position, board_of, cell_of, mv};

/// A symmetry of the 3×3 grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sym {
    Id,
    Rot90,
    Rot180,
    Rot270,
    FlipH,
    FlipV,
    Transpose,
    Antitranspose,
}

/// The image of each index `0..8` under each symmetry, in `Sym::ALL` order.
/// Rotations are clockwise. Same tables as `Sym.table` in Lean.
const TABLE: [[u8; 9]; 8] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8],
    [2, 5, 8, 1, 4, 7, 0, 3, 6],
    [8, 7, 6, 5, 4, 3, 2, 1, 0],
    [6, 3, 0, 7, 4, 1, 8, 5, 2],
    [2, 1, 0, 5, 4, 3, 8, 7, 6],
    [6, 7, 8, 3, 4, 5, 0, 1, 2],
    [0, 3, 6, 1, 4, 7, 2, 5, 8],
    [8, 5, 2, 7, 4, 1, 6, 3, 0],
];

/// Each symmetry applied to every 9-bit set of grid indices.
static MASKS: [[u16; 512]; 8] = {
    let mut table = [[0u16; 512]; 8];
    let mut g = 0;
    while g < 8 {
        let mut mask = 0;
        while mask < 512 {
            let mut i = 0;
            while i < 9 {
                if mask & (1 << i) != 0 {
                    table[g][mask] |= 1 << TABLE[g][i];
                }
                i += 1;
            }
            mask += 1;
        }
        g += 1;
    }
    table
};

impl Sym {
    pub const ALL: [Sym; 8] = [
        Sym::Id,
        Sym::Rot90,
        Sym::Rot180,
        Sym::Rot270,
        Sym::FlipH,
        Sym::FlipV,
        Sym::Transpose,
        Sym::Antitranspose,
    ];

    /// The inverse symmetry.
    pub const fn inv(self) -> Sym {
        match self {
            Sym::Rot90 => Sym::Rot270,
            Sym::Rot270 => Sym::Rot90,
            g => g,
        }
    }

    /// Where the symmetry sends grid index `i`.
    #[inline]
    pub const fn apply(self, i: u8) -> u8 {
        TABLE[self as usize][i as usize]
    }

    /// The symmetry applied to a 9-bit set of grid indices.
    #[inline]
    pub fn apply_mask(self, mask: u16) -> u16 {
        MASKS[self as usize][(mask & FULL) as usize]
    }

    /// The symmetry applied to a move: the same symmetry on its board and cell.
    #[inline]
    pub const fn apply_move(self, m: Move) -> Move {
        mv(self.apply(board_of(m)), self.apply(cell_of(m)))
    }

    /// The symmetry applied to an 81-bit set of cells.
    #[inline]
    pub fn apply_cells(self, cells: u128) -> u128 {
        let mut out = 0u128;
        for b in 0..9u8 {
            let local = (cells >> (9 * b)) as u16 & FULL;
            if local != 0 {
                out |= (self.apply_mask(local) as u128) << (9 * self.apply(b));
            }
        }
        out
    }
}

impl Position {
    /// The position with symmetry `g` applied to the global board and every local board.
    pub fn transform(&self, g: Sym) -> Position {
        let (cells, owner, full, to_move, sent) = self.parts();
        Position::from_parts(
            [g.apply_cells(cells[0]), g.apply_cells(cells[1])],
            [g.apply_mask(owner[0]), g.apply_mask(owner[1])],
            g.apply_mask(full),
            to_move,
            sent.map(|b| g.apply(b)),
        )
    }

    /// A canonical representative of the position's symmetry class, and the
    /// symmetry that maps the position to it.
    pub fn canonical(&self) -> (Position, Sym) {
        Sym::ALL
            .into_iter()
            .map(|g| (self.transform(g), g))
            .min_by_key(|(p, _)| p.sort_key())
            .expect("8 symmetries")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Moves, Variant};

    #[test]
    fn inverses() {
        for g in Sym::ALL {
            for i in 0..9 {
                assert_eq!(g.inv().apply(g.apply(i)), i);
            }
            for m in 0..81 {
                assert_eq!(g.inv().apply_move(g.apply_move(m)), m);
            }
        }
    }

    #[test]
    fn lines_map_to_lines() {
        for g in Sym::ALL {
            for line in crate::lines::LINES {
                let mask = line.iter().fold(0u16, |acc, &i| acc | 1 << i);
                assert!(crate::lines::has_line(g.apply_mask(mask)));
            }
        }
    }

    /// Plays pseudo-random games and checks that every symmetry preserves legal
    /// moves, commutes with playing, and preserves results (`map_legal`,
    /// `map_play` and `map_outcome` in Lean).
    #[test]
    fn symmetries_preserve_the_rules() {
        let mut seed = 12345u64;
        for game in 0..300 {
            let v = Variant::ALL[game % 3];
            let mut pos = Position::new();
            loop {
                for g in Sym::ALL {
                    let t = pos.transform(g);
                    assert_eq!(t.legal_moves(v), g.apply_cells(pos.legal_moves(v)));
                    assert_eq!(t.result(v), pos.result(v));
                    assert_eq!(t.transform(g.inv()), pos);
                    assert_eq!(t.canonical().0, pos.canonical().0);
                }
                let legal: Vec<_> = Moves(pos.legal_moves(v)).collect();
                if legal.is_empty() {
                    break;
                }
                seed = seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let m = legal[(seed >> 33) as usize % legal.len()];
                for g in Sym::ALL {
                    assert_eq!(
                        pos.transform(g).after(g.apply_move(m)),
                        pos.after(m).transform(g)
                    );
                }
                pos.play(m);
            }
        }
    }
}
