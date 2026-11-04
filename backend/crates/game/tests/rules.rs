//! The concrete games from `proofs/Ulttt/Examples.lean`, checked against the Rust rules.

use ulttt_game::{Moves, Player, Position, Variant, board_of, cell_of, mv};

fn play_all(moves: &[(u8, u8)]) -> Position {
    let mut pos = Position::new();
    for &(b, c) in moves {
        pos.play(mv(b, c));
    }
    pos
}

fn all_legal(v: Variant, moves: &[(u8, u8)]) -> bool {
    let mut pos = Position::new();
    for &(b, c) in moves {
        if !pos.is_legal(v, mv(b, c)) {
            return false;
        }
        pos.play(mv(b, c));
    }
    true
}

fn legal_pairs(v: Variant, pos: &Position) -> Vec<(u8, u8)> {
    Moves(pos.legal_moves(v))
        .map(|m| (board_of(m), cell_of(m)))
        .collect()
}

const X_WINS_BOARD_0: [(u8, u8); 5] = [(0, 1), (1, 0), (0, 2), (2, 0), (0, 0)];

#[test]
fn every_cell_is_legal_at_the_start() {
    for v in Variant::ALL {
        assert_eq!(Moves(Position::new().legal_moves(v)).len(), 81);
    }
}

#[test]
fn opponent_is_sent_to_the_matching_board() {
    let pos = play_all(&[(0, 4)]);
    let expected: Vec<_> = (0..9).map(|c| (4, c)).collect();
    assert_eq!(legal_pairs(Variant::Classic, &pos), expected);
}

#[test]
fn winning_a_local_board() {
    for v in Variant::ALL {
        assert!(all_legal(v, &X_WINS_BOARD_0));
    }
    let pos = play_all(&X_WINS_BOARD_0);
    assert_eq!(pos.owner(0), Some(Player::X));
    assert_eq!(pos.sent(), Some(0));
}

#[test]
fn sent_to_a_won_board() {
    let pos = play_all(&X_WINS_BOARD_0);
    // classic: board 0 is closed, so O may play any empty cell of boards 1-8.
    assert_eq!(legal_pairs(Variant::Classic, &pos).len(), 72 - 2);
    assert_eq!(legal_pairs(Variant::Majority, &pos).len(), 72 - 2);
    // open: board 0 is still playable, so O must play one of its 6 empty cells.
    assert_eq!(
        legal_pairs(Variant::Open, &pos),
        [(0, 3), (0, 4), (0, 5), (0, 6), (0, 7), (0, 8)]
    );
}

#[test]
fn line_on_a_won_board_does_not_change_owner() {
    let mut moves = X_WINS_BOARD_0.to_vec();
    moves.extend([(0, 3), (3, 0), (0, 4), (4, 0), (0, 5)]);
    assert!(all_legal(Variant::Open, &moves));
    assert_eq!(play_all(&moves).owner(0), Some(Player::X));
    assert!(!all_legal(Variant::Classic, &moves));
}
