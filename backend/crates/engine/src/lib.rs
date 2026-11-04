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
mod mcts;
mod options;
mod search;

pub use eval::{MAX_EVAL, WIN, evaluate};
pub use mcts::{Mcts, MctsOptions};
pub use options::{Options, Weights};
pub use search::{Choice, Engine};

use ulttt_game::{Position, Variant};

/// Anything that picks moves: the alpha-beta [`Engine`] or [`Mcts`].
pub trait Agent {
    /// A move for the player to move, visiting about `node_limit` positions;
    /// `None` if the game is over.
    fn best_move(&mut self, pos: &Position, node_limit: u64) -> Option<Choice>;
    /// A clock for each move: the fraction of the move's time used so far.
    fn set_clock(&mut self, clock: Option<Box<dyn Fn() -> f32>>);
}

impl Agent for Engine {
    fn best_move(&mut self, pos: &Position, node_limit: u64) -> Option<Choice> {
        Engine::best_move(self, pos, node_limit)
    }
    fn set_clock(&mut self, clock: Option<Box<dyn Fn() -> f32>>) {
        Engine::set_clock(self, clock);
    }
}

impl Agent for Mcts {
    fn best_move(&mut self, pos: &Position, node_limit: u64) -> Option<Choice> {
        Mcts::best_move(self, pos, node_limit)
    }
    fn set_clock(&mut self, clock: Option<Box<dyn Fn() -> f32>>) {
        Mcts::set_clock(self, clock);
    }
}

/// An agent from a spec: `mcts` or `mcts:<settings>` (see
/// [`MctsOptions::parse`]), otherwise alpha-beta settings for
/// [`Options::parse`] (empty for the defaults).
pub fn agent(v: Variant, spec: &str) -> Result<Box<dyn Agent>, String> {
    match spec.strip_prefix("mcts") {
        Some(rest) if rest.is_empty() || rest.starts_with(':') => {
            let opts = MctsOptions::parse(rest.trim_start_matches(':'))?;
            Ok(Box::new(Mcts::new(v, opts)))
        }
        _ => Ok(Box::new(Engine::new(v, Options::parse(spec)?))),
    }
}
