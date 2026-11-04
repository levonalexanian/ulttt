//! Ultimate Tic-Tac-Toe rules for the `classic`, `open` and `majority` variants.
//!
//! The rules follow `RULES.md` and mirror the Lean formalization in `proofs/`.

pub mod lines;
mod notation;
mod perft;
mod position;
pub mod random;
mod symmetry;
mod types;

pub use perft::perft;
pub use position::{ALL_CELLS, Move, Moves, Position, board_of, cell_of, mv};
pub use symmetry::Sym;
pub use types::{Outcome, Player, Variant};
