//! Move ordering: try the moves most likely to be best first, so alpha-beta
//! cuts off sooner.

use ulttt_game::lines::has_line;
use ulttt_game::{Move, Moves, Position, Variant, board_of, cell_of};

/// Moves in the order to search them.
pub struct Ordered {
    moves: [Move; 81],
    len: usize,
}

impl Ordered {
    pub fn as_slice(&self) -> &[Move] {
        &self.moves[..self.len]
    }
}

/// A move that wins the game outright for the player to move, if any.
pub fn winning_move(pos: &Position, legal: u128) -> Option<Move> {
    let me = pos.to_move();
    let mine = pos.owned_by(me);
    let unowned = !(mine | pos.owned_by(me.other())) & 0x1FF;
    // Only boards that would complete a global line matter: at most a few.
    let mut boards = unowned;
    while boards != 0 {
        let b = boards.trailing_zeros() as u8;
        boards &= boards - 1;
        if !has_line(mine | 1 << b) {
            continue;
        }
        let legal_here = (legal >> (9 * b as u32)) as u16 & 0x1FF;
        let wins = pos.local_threats(me, b) & legal_here;
        if wins != 0 {
            return Some(9 * b + wins.trailing_zeros() as u8);
        }
    }
    None
}

/// Orders `legal` by a static heuristic, with `first` (e.g. the best move from
/// the transposition table) ahead of everything. Ties are broken by `history`:
/// how often each move caused a cutoff before.
pub fn order(
    pos: &Position,
    v: Variant,
    legal: u128,
    first: Option<Move>,
    history: &[u32; 81],
) -> Ordered {
    let me = pos.to_move();
    let opp = me.other();
    let owned = pos.owned_by(me) | pos.owned_by(opp);
    let closed = pos.closed(v);
    let mut scored = [(0i64, 0u8); 81];
    let mut len = 0;
    for m in Moves(legal) {
        let (b, c) = (board_of(m), cell_of(m));
        let mut score: i64 = 0;
        if first == Some(m) {
            score += 10_000;
        }
        // Winning a local board.
        let wins_board = owned >> b & 1 == 0 && pos.local_threats(me, b) >> c & 1 != 0;
        if wins_board {
            score += 100;
        }
        // Where the opponent is sent. If that board is (or this move makes it)
        // closed, they get a free choice, which is usually bad for us.
        let fills_board = pos.local_cells(me, b) | pos.local_cells(opp, b) | 1 << c == 0x1FF;
        let closes_b = fills_board || (wins_board && v != Variant::Open);
        if closed >> c & 1 != 0 || (c == b && closes_b) {
            score -= 50;
        } else if owned >> c & 1 == 0 && pos.local_threats(opp, c) != 0 {
            // The opponent can win the board they are sent to.
            score -= 30;
        }
        scored[len] = (score << 32 | history[m as usize] as i64, m);
        len += 1;
    }
    scored[..len].sort_by_key(|&(s, _)| -s);
    let mut moves = [0; 81];
    for (i, &(_, m)) in scored[..len].iter().enumerate() {
        moves[i] = m;
    }
    Ordered { moves, len }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ulttt_game::random::Rng;

    /// `winning_move` finds a move exactly when some legal move wins at once.
    #[test]
    fn winning_move_matches_brute_force() {
        let mut rng = Rng::new(21);
        for i in 0..3000 {
            let v = Variant::ALL[i % 3];
            let game = rng.game(v);
            let mut pos = Position::new();
            for &m in &game {
                let legal = pos.legal_moves(v);
                let brute = Moves(legal).find(|&w| pos.after(w).global_line(pos.to_move()));
                let found = winning_move(&pos, legal);
                assert_eq!(found.is_some(), brute.is_some(), "{v} {pos}");
                if let Some(w) = found {
                    assert!(legal >> w & 1 != 0 && pos.after(w).global_line(pos.to_move()));
                }
                pos.play(m);
            }
        }
    }
}
