//! Ultimate Tic-Tac-Toe rules for the `classic`, `open` and `majority` variants.
//!
//! The rules follow `RULES.md` and mirror the Lean formalization in `proofs/`.

pub mod lines;
mod notation;
mod position;
mod types;

pub use position::{ALL_CELLS, Move, Moves, Position, board_of, cell_of, mv};
pub use types::{Outcome, Player, Variant};
