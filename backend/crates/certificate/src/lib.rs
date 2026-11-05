//! Certificates of solved positions.
//!
//! A certificate claims that `player` can force a result worth at least
//! `at_least` to them (1: a win, 0: at least a draw) from `position`. It is the
//! player's strategy: their moves in depth-first order. Where the opponent is
//! to move, every legal reply is followed, in increasing order. This is exactly
//! the format `check` in `proofs/Ulttt/Certificate.lean` consumes, and Lean
//! proves that an accepted certificate establishes the claim.
//!
//! A position's value needs one certificate if it is a win (for the winner) and
//! two if it is a draw (at least a draw for each player).

pub mod paper;

use std::fmt;
use std::str::FromStr;

use ulttt_game::{Move, Moves, Outcome, Player, Position, Variant};
use ulttt_solver::{Solver, Value};

/// A claim with the strategy that proves it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Certificate {
    pub variant: Variant,
    pub player: Player,
    /// 1: `player` can force a win; 0: `player` can force at least a draw.
    pub at_least: i8,
    pub position: Position,
    pub moves: Vec<Move>,
}

/// The score of an outcome for player `p`.
fn score_for(o: Outcome, p: Player) -> i8 {
    if p == Player::X {
        o.score()
    } else {
        -o.score()
    }
}

/// The value of a position for player `p`.
fn value_for(v: Value, p: Player) -> i8 {
    let x = v as i8;
    if p == Player::X { x } else { -x }
}

/// Builds a certificate that `player` can force at least `at_least` from
/// `pos`, using `solver` to pick moves. Returns `None` if the claim is false
/// (or the solver hit its node limit).
pub fn generate(
    solver: &mut dyn Solver,
    pos: &Position,
    v: Variant,
    player: Player,
    at_least: i8,
) -> Option<Certificate> {
    let mut moves = Vec::new();
    strategy(solver, pos, v, player, at_least, &mut moves).then_some(Certificate {
        variant: v,
        player,
        at_least,
        position: *pos,
        moves,
    })
}

fn strategy(
    solver: &mut dyn Solver,
    pos: &Position,
    v: Variant,
    p: Player,
    r: i8,
    out: &mut Vec<Move>,
) -> bool {
    if let Some(o) = pos.result(v) {
        return score_for(o, p) >= r;
    }
    // Proved in Lean (`valueFor_le_zero_of_mayWin_false`): if the opponent can
    // no longer win, a draw or better is guaranteed. The checker stops here too.
    if r <= 0 && !pos.may_win(v, p.other()) {
        return true;
    }
    let legal = Moves(pos.legal_moves(v));
    if pos.to_move() == p {
        // Try first the moves likely to need the smallest certificate: those
        // that end the game, then those that need no further moves (the
        // opponent can no longer win), then those leaving the opponent the
        // fewest replies. Any move that keeps the value would be correct.
        let mut candidates: Vec<(u32, Move)> = legal
            .map(|m| {
                let child = pos.after(m);
                let cost = if child.is_terminal(v) {
                    0
                } else if r <= 0 && !child.may_win(v, p.other()) {
                    1
                } else {
                    2 + child.legal_moves(v).count_ones()
                };
                (cost, m)
            })
            .collect();
        candidates.sort_unstable();
        for (_, m) in candidates {
            let child = pos.after(m);
            match solver.solve(&child, v) {
                Some(value) if value_for(value, p) >= r => {
                    out.push(m);
                    return strategy(solver, &child, v, p, r, out);
                }
                Some(_) => {}
                None => return false,
            }
        }
        false
    } else {
        legal
            .into_iter()
            .all(|m| strategy(solver, &pos.after(m), v, p, r, out))
    }
}

/// Checks a certificate the same way the Lean checker does. Useful to test the
/// generator; the Lean checker is the one to trust.
pub fn check(cert: &Certificate) -> bool {
    let mut rest = &cert.moves[..];
    check_from(cert, &cert.position, &mut rest) && rest.is_empty()
}

fn check_from(cert: &Certificate, pos: &Position, rest: &mut &[Move]) -> bool {
    let v = cert.variant;
    if let Some(o) = pos.result(v) {
        return score_for(o, cert.player) >= cert.at_least;
    }
    if cert.at_least <= 0 && !pos.may_win(v, cert.player.other()) {
        return true;
    }
    if pos.to_move() == cert.player {
        let Some((&m, tail)) = rest.split_first() else {
            return false;
        };
        *rest = tail;
        pos.is_legal(v, m) && check_from(cert, &pos.after(m), rest)
    } else {
        Moves(pos.legal_moves(v)).all(|m| check_from(cert, &pos.after(m), rest))
    }
}

/// The text format read by the Lean checker (`ulttt-check`):
///
/// ```text
/// ulttt-certificate 1
/// variant <classic|open|majority>
/// player <x|o>
/// at-least <0|1>
/// position <position notation>
/// moves <move> <move> ...
/// ```
impl fmt::Display for Certificate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ulttt-certificate 1")?;
        writeln!(f, "variant {}", self.variant)?;
        writeln!(f, "player {}", self.player.symbol())?;
        writeln!(f, "at-least {}", self.at_least)?;
        writeln!(f, "position {}", self.position)?;
        write!(f, "moves")?;
        for m in &self.moves {
            write!(f, " {m}")?;
        }
        writeln!(f)
    }
}

impl FromStr for Certificate {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut fields = std::collections::HashMap::new();
        let mut lines = s.lines();
        if lines.next() != Some("ulttt-certificate 1") {
            return Err("missing header".into());
        }
        for line in lines {
            if let Some((k, v)) = line.split_once(' ') {
                fields.insert(k, v);
            } else if line == "moves" {
                fields.insert("moves", "");
            }
        }
        let get = |k: &str| fields.get(k).copied().ok_or(format!("missing {k}"));
        let player = match get("player")? {
            "x" => Player::X,
            "o" => Player::O,
            p => return Err(format!("bad player {p:?}")),
        };
        let moves = get("moves")?
            .split_whitespace()
            .map(|m| m.parse::<Move>().map_err(|e| e.to_string()))
            .collect::<Result<_, _>>()?;
        Ok(Certificate {
            variant: get("variant")?.parse()?,
            player,
            at_least: get("at-least")?.parse().map_err(|_| "bad at-least")?,
            position: get("position")?.parse()?,
            moves,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ulttt_game::random::Rng;
    use ulttt_solver::{Options, Search};

    #[test]
    fn certificates_for_endgames() {
        let mut rng = Rng::new(3);
        let mut solver = Search::new(Options::parse("tt=16").unwrap());
        for i in 0..30 {
            let v = Variant::ALL[i % 3];
            let Some(pos) = rng.position(v, 54) else {
                continue;
            };
            let value = solver.solve(&pos, v).unwrap();
            let claims: Vec<(Player, i8)> = match value {
                Value::XWins => vec![(Player::X, 1)],
                Value::OWins => vec![(Player::O, 1)],
                Value::Draw => vec![(Player::X, 0), (Player::O, 0)],
            };
            for (p, r) in claims {
                let cert = generate(&mut solver, &pos, v, p, r).expect("claim holds");
                assert!(check(&cert));
                let parsed: Certificate = cert.to_string().parse().unwrap();
                assert_eq!(parsed, cert);
                // A stronger claim than the value fails.
                if r == 0 {
                    assert!(generate(&mut solver, &pos, v, p, 1).is_none());
                }
            }
        }
    }

    #[test]
    fn tampered_certificates_fail() {
        let mut rng = Rng::new(5);
        let mut solver = Search::new(Options::parse("tt=16").unwrap());
        let mut tested = 0;
        while tested < 10 {
            let Some(pos) = rng.position(Variant::Classic, 56) else {
                continue;
            };
            let p = pos.to_move();
            let Some(mut cert) = generate(&mut solver, &pos, Variant::Classic, p, 0) else {
                continue;
            };
            if cert.moves.is_empty() {
                continue;
            }
            cert.moves.pop();
            assert!(!check(&cert));
            tested += 1;
        }
    }
}
