//! The winning strategy for X in `open` of Bertholon, Géraud-Stewart,
//! Kugelmann, Lenoir and Naccache, "At Most 43 Moves, At Least 29" (2020).
//!
//! The paper's three phases, as a function of the position alone:
//!
//! - Opening: play the centre of the centre board, then the centre of every
//!   board O sends X to. O is sent back to the centre board each time, until
//!   it is full; the board O sends X to last is called `f` here, and `g` is the
//!   board opposite it (`8 - f`).
//! - Middlegame: play cell `f` of `f`, sending O to `f`. When O plays cell `k`
//!   of `f`, answer cell `f` of board `k`, sending O back to `f`; when O plays
//!   cell 4 or `g` (sending X to the full centre, or to `g`), answer cell `f`
//!   of `g`, or cell `g` of `g` once that is taken, which ends the middlegame.
//! - Endgame: on the board `k` X is sent to, play cell `f` if free, else cell
//!   `g`.
//!
//! Every board other than `f`, 4 and `g` ends up with X on cells `f`, 4 and
//! `g`, which are in a line, and X wins.

use std::collections::HashMap;

use ulttt_game::{Move, Moves, Outcome, Player, Position, Variant};

/// The move at cell `c` of board `b`.
fn mv(b: u8, c: u8) -> Move {
    9 * b + c
}

/// The paper's move for X, or `None` where it says nothing (O to move, the
/// game over, or a position the strategy never reaches).
pub fn strategy(pos: &Position) -> Option<Move> {
    let v = Variant::Open;
    if pos.to_move() != Player::X || pos.is_terminal(v) {
        return None;
    }
    let free = |m: Move| pos.cell(m).is_none();
    let full = |b: u8| pos.full_boards() >> b & 1 != 0;
    // Opening.
    if pos.empties() == 81 {
        return Some(mv(4, 4));
    }
    if !full(4) {
        return pos.sent().map(|k| mv(k, 4));
    }
    // After the opening, `f` is the one board whose centre X has not taken.
    let mut fs = (0..9u8).filter(|&b| b != 4 && pos.cell(mv(b, 4)) != Some(Player::X));
    let (f, None) = (fs.next()?, fs.next()) else {
        return None;
    };
    let g = 8 - f;
    let m = match pos.sent() {
        Some(k) if k == f => mv(f, f),
        Some(k) if k != 4 && k != g && !full(k) => {
            if free(mv(k, f)) {
                mv(k, f)
            } else {
                mv(k, g)
            }
        }
        // Sent to the full centre board, or to `g`.
        _ => {
            if free(mv(g, f)) {
                mv(g, f)
            } else {
                mv(g, g)
            }
        }
    };
    pos.is_legal(v, m).then_some(m)
}

/// What exploring the strategy against every reply of O found.
#[derive(Debug, Default)]
pub struct Exploration {
    /// Distinct positions reached (either side to move), finished games included.
    pub positions: usize,
    /// Number of different games (sequences of O's replies), i.e. the size of
    /// a strategy tree that does not share transpositions.
    pub games: u128,
    /// The longest game, in moves.
    pub longest: u32,
    /// A position where the strategy has no move or a game it does not win.
    pub failure: Option<Position>,
}

/// Plays the strategy from the empty board against every reply of O.
pub fn explore() -> Exploration {
    let mut seen: HashMap<Position, (u128, u32)> = HashMap::new();
    let mut failure = None;
    let (games, longest) = walk(&Position::new(), &mut seen, &mut failure);
    Exploration {
        positions: seen.len(),
        games,
        longest,
        failure,
    }
}

/// Games and longest remaining length from `pos`, memoised in `seen`.
fn walk(
    pos: &Position,
    seen: &mut HashMap<Position, (u128, u32)>,
    failure: &mut Option<Position>,
) -> (u128, u32) {
    if let Some(&r) = seen.get(pos) {
        return r;
    }
    let v = Variant::Open;
    let r = if let Some(o) = pos.result(v) {
        if o != Outcome::Win(Player::X) {
            failure.get_or_insert(*pos);
        }
        (1, 0)
    } else if pos.to_move() == Player::X {
        match strategy(pos) {
            Some(m) => {
                let (n, l) = walk(&pos.after(m), seen, failure);
                (n, l + 1)
            }
            None => {
                failure.get_or_insert(*pos);
                (1, 0)
            }
        }
    } else {
        Moves(pos.legal_moves(v)).fold((0, 0), |(n, l), m| {
            let (n2, l2) = walk(&pos.after(m), seen, failure);
            (n + n2, l.max(l2 + 1))
        })
    };
    seen.insert(*pos, r);
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_strategy_wins_against_every_reply() {
        let e = explore();
        assert_eq!(e.failure, None, "{e:?}");
        println!("{e:?}");
    }
}
