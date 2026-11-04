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

/// `may_win` never rules out a player who goes on to win, and once it rules a
/// player out it stays that way.
#[test]
fn may_win_is_sound_and_monotone() {
    let mut rng = ulttt_game::random::Rng::new(7);
    for i in 0..3000 {
        let v = Variant::ALL[i % 3];
        let moves = rng.game(v);
        let mut pos = Position::new();
        let mut history = vec![pos];
        for &m in &moves {
            pos.play(m);
            history.push(pos);
        }
        let winner = match pos.outcome(v) {
            ulttt_game::Outcome::Win(p) => Some(p),
            ulttt_game::Outcome::Draw => None,
        };
        for p in [Player::X, Player::O] {
            let flags: Vec<bool> = history.iter().map(|h| h.may_win(v, p)).collect();
            if winner == Some(p) {
                assert!(
                    flags.iter().all(|&f| f),
                    "{v}: winner ruled out in {moves:?}"
                );
            }
            assert!(
                flags.windows(2).all(|w| w[0] || !w[1]),
                "{v}: may_win came back for {p:?} in {moves:?}"
            );
        }
    }
}

/// `local_threats` matches a direct check: playing the cell completes a line.
#[test]
fn local_threats_match_brute_force() {
    use ulttt_game::lines::{LINES, completing_cells};
    for own in 0u16..512 {
        for opp in 0u16..512 {
            if own & opp != 0 {
                continue;
            }
            let brute = (0..9u16)
                .filter(|&c| (own | opp) >> c & 1 == 0)
                .filter(|&c| {
                    // Some line through `c` has its other two cells in `own`.
                    let after = own | 1 << c;
                    LINES
                        .iter()
                        .any(|l| l.contains(&(c as u8)) && l.iter().all(|&i| after >> i & 1 != 0))
                })
                .fold(0, |acc, c| acc | 1 << c);
            assert_eq!(completing_cells(own, opp), brute, "own {own:b} opp {opp:b}");
        }
    }
}

/// `hash64` spreads positions over table slots: distinct positions from random
/// games almost never share a hash, or the low bits used as a table index.
#[test]
fn hashes_are_well_spread() {
    let mut rng = ulttt_game::random::Rng::new(3);
    let mut positions = std::collections::HashSet::new();
    for i in 0..2000 {
        let v = Variant::ALL[i % 3];
        let mut pos = Position::new();
        for m in rng.game(v) {
            pos.play(m);
            positions.insert(pos);
        }
    }
    let hashes: std::collections::HashSet<u64> = positions.iter().map(|p| p.hash64()).collect();
    assert_eq!(hashes.len(), positions.len());
    // With 2^16 slots, the fullest slot should hold only a few times the average.
    let mut slots = vec![0u32; 1 << 16];
    for h in &hashes {
        slots[(h & 0xFFFF) as usize] += 1;
    }
    let average = positions.len() as f64 / slots.len() as f64;
    let fullest = *slots.iter().max().unwrap() as f64;
    assert!(
        fullest < 4.0 * average + 8.0,
        "fullest slot {fullest}, average {average}"
    );
}
