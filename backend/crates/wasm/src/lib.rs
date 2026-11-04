//! WebAssembly bindings to the rules and solver, for the frontend.
//!
//! Positions cross the boundary in the text notation of `ulttt_game`, variants
//! as their names (`classic`, `open`, `majority`) and values as numbers from
//! X's point of view: `1` X wins, `0` draw, `-1` O wins, [`UNKNOWN`] when the
//! node limit was reached first.

use std::cell::RefCell;

use ulttt_engine::Engine;
use ulttt_game::{Moves, Outcome, Player, Position, Variant};
use ulttt_solver::{Options, Search, Solver};
use wasm_bindgen::prelude::*;

/// The value reported when a search ran out of nodes.
pub const UNKNOWN: i32 = 2;

/// Transposition table size for the browser: 2^20 entries, 16 MiB.
const TT_BITS: u32 = 20;

thread_local! {
    /// The solver, kept between calls so its table stays warm. The table does
    /// not distinguish variants, so it is replaced when the variant changes.
    static SOLVER: RefCell<Option<(Variant, Search)>> = const { RefCell::new(None) };
    /// The playing engine, kept between moves for the same reason.
    static ENGINE: RefCell<Option<Engine>> = const { RefCell::new(None) };
}

/// Table sizes of the playing engine: 2^18 heuristic entries (4 MiB) and 2^19
/// exact ones (8 MiB).
const ENGINE_TT_BITS: u32 = 18;
const ENGINE_EXACT_TT_BITS: u32 = 19;

fn variant(name: &str) -> Result<Variant, JsError> {
    name.parse().map_err(|e: String| JsError::new(&e))
}

fn position(s: &str) -> Result<Position, JsError> {
    s.parse().map_err(|e: String| JsError::new(&e))
}

fn with_solver<T>(v: Variant, node_limit: u64, f: impl FnOnce(&mut Search) -> T) -> T {
    SOLVER.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.as_ref().is_none_or(|(sv, _)| *sv != v) {
            let opts = Options {
                tt_bits: TT_BITS,
                ..Options::default()
            };
            *slot = Some((v, Search::new(opts)));
        }
        let (_, solver) = slot.as_mut().expect("just set");
        solver.set_node_limit(Some(node_limit));
        f(solver)
    })
}

fn solve_with(solver: &mut Search, pos: &Position, v: Variant) -> i32 {
    solver.solve(pos, v).map_or(UNKNOWN, |value| value as i32)
}

/// The empty starting position.
#[wasm_bindgen]
pub fn start() -> String {
    Position::new().to_string()
}

/// The legal moves (0-80) of a position.
#[wasm_bindgen(js_name = legalMoves)]
pub fn legal_moves(variant_name: &str, pos: &str) -> Result<Vec<u8>, JsError> {
    let (v, pos) = (variant(variant_name)?, position(pos)?);
    Ok(Moves(pos.legal_moves(v)).collect())
}

/// The position after playing a legal move.
#[wasm_bindgen]
pub fn play(variant_name: &str, pos: &str, m: u8) -> Result<String, JsError> {
    let (v, pos) = (variant(variant_name)?, position(pos)?);
    if !pos.is_legal(v, m) {
        return Err(JsError::new(&format!("illegal move {m}")));
    }
    Ok(pos.after(m).to_string())
}

/// The result of a finished game: `"x"`, `"o"` or `"draw"`; `""` if not over.
#[wasm_bindgen]
pub fn result(variant_name: &str, pos: &str) -> Result<String, JsError> {
    let (v, pos) = (variant(variant_name)?, position(pos)?);
    Ok(match pos.result(v) {
        None => "",
        Some(Outcome::Win(Player::X)) => "x",
        Some(Outcome::Win(Player::O)) => "o",
        Some(Outcome::Draw) => "draw",
    }
    .to_string())
}

/// The value of a position, searching at most `node_limit` positions.
#[wasm_bindgen]
pub fn solve(variant_name: &str, pos: &str, node_limit: u32) -> Result<i32, JsError> {
    let (v, pos) = (variant(variant_name)?, position(pos)?);
    Ok(with_solver(v, node_limit as u64, |s| {
        solve_with(s, &pos, v)
    }))
}

/// The value of every legal move, as pairs `[move, value, move, value, ...]`,
/// each searched with at most `node_limit` positions.
#[wasm_bindgen]
pub fn analyze(variant_name: &str, pos: &str, node_limit: u32) -> Result<Vec<i32>, JsError> {
    let (v, pos) = (variant(variant_name)?, position(pos)?);
    Ok(with_solver(v, node_limit as u64, |s| {
        Moves(pos.legal_moves(v))
            .flat_map(|m| [m as i32, solve_with(s, &pos.after(m), v)])
            .collect()
    }))
}

/// The engine's move for the player to move, searching about `node_limit`
/// positions; `-1` if the game is over. Uses the heuristic engine, which plays
/// sensibly long before the exact solver can say anything, and keeps proven
/// wins and draws once it can.
#[wasm_bindgen(js_name = bestMove)]
pub fn best_move(variant_name: &str, pos: &str, node_limit: u32) -> Result<i32, JsError> {
    let (v, pos) = (variant(variant_name)?, position(pos)?);
    Ok(ENGINE.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.as_ref().is_none_or(|e| e.variant() != v) {
            *slot = Some(Engine::new(v, ENGINE_TT_BITS, ENGINE_EXACT_TT_BITS));
        }
        let engine = slot.as_mut().expect("just set");
        engine
            .best_move(&pos, node_limit as u64)
            .map_or(-1, |c| c.mv as i32)
    }))
}

/// A line of optimal play from the position, or an empty list if it could not
/// be solved within `node_limit` positions per search.
#[wasm_bindgen(js_name = principalVariation)]
pub fn principal_variation(
    variant_name: &str,
    pos: &str,
    node_limit: u32,
) -> Result<Vec<u8>, JsError> {
    let (v, pos) = (variant(variant_name)?, position(pos)?);
    Ok(with_solver(v, node_limit as u64, |s| {
        ulttt_solver::principal_variation(s, &pos, v).unwrap_or_default()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn play_a_short_game() {
        let mut pos = start();
        assert_eq!(legal_moves("classic", &pos).unwrap().len(), 81);
        pos = play("classic", &pos, 4).unwrap();
        assert_eq!(
            legal_moves("classic", &pos).unwrap(),
            (36..45).collect::<Vec<_>>()
        );
        assert_eq!(result("classic", &pos).unwrap(), "");
    }

    #[test]
    fn best_move_takes_a_win() {
        // X owns boards 0 and 1, has two in a row on board 2, and is sent there.
        let mut cells = vec!['.'; 81];
        for m in [0, 1, 2, 9, 10, 11, 18, 19] {
            cells[m] = 'x';
        }
        for m in [45, 46, 47, 54, 55, 56, 63, 64] {
            cells[m] = 'o';
        }
        let pos = format!("{}/xx...oo../x/2", cells.into_iter().collect::<String>());
        assert_eq!(best_move("classic", &pos, 100_000).unwrap(), 20);
        assert_eq!(solve("classic", &pos, 100_000).unwrap(), 1);
    }
}
