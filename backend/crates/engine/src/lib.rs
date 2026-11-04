//! A playing engine for positions too deep to solve exactly.
//!
//! The exact solvers in `ulttt-solver` only answer once they have seen every
//! line to the end, which is hopeless early in the game. This engine instead
//! searches as deep as its node budget allows (iterative-deepening alpha-beta)
//! and scores the positions where it stops with a heuristic ([`evaluate`]).
//! When the exact solver can settle the position within part of the budget, the
//! engine only considers moves that keep the solved value, so it never throws
//! away a proven win or draw.

mod eval;
mod options;
mod search;

pub use eval::{MAX_EVAL, WIN, evaluate};
pub use options::{Options, Weights};
pub use search::{Choice, Engine};
