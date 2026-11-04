//! A compact text format for positions:
//!
//! ```text
//! <81 cells>/<9 owners>/<player to move>/<board sent to>
//! ```
//!
//! Cells are listed by move index (board-major), owners by board index, each as
//! `x`, `o` or `.`. The board sent to is `0`-`8`, or `-` before the first move.
//! Owners are stored because in the `open` variant they are not determined by the
//! cells alone.

use std::fmt;
use std::str::FromStr;

use crate::lines::FULL;
use crate::position::Position;
use crate::types::Player;

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sym = |p: Option<Player>| p.map_or('.', Player::symbol);
        let cells: String = (0..81).map(|m| sym(self.cell(m))).collect();
        let owners: String = (0..9).map(|b| sym(self.owner(b))).collect();
        let sent = self.sent().map_or('-', |b| (b'0' + b) as char);
        write!(f, "{cells}/{owners}/{}/{sent}", self.to_move().symbol())
    }
}

fn parse_player(c: char) -> Result<Option<Player>, String> {
    match c {
        'x' => Ok(Some(Player::X)),
        'o' => Ok(Some(Player::O)),
        '.' => Ok(None),
        _ => Err(format!("unexpected character {c:?}")),
    }
}

impl FromStr for Position {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.trim().split('/').collect();
        let [cells, owners, to_move, sent] = parts[..] else {
            return Err("expected 4 fields separated by '/'".into());
        };
        if cells.chars().count() != 81 {
            return Err("expected 81 cells".into());
        }
        if owners.chars().count() != 9 {
            return Err("expected 9 owners".into());
        }
        let mut cell_bits = [0u128; 2];
        for (m, c) in cells.chars().enumerate() {
            if let Some(p) = parse_player(c)? {
                cell_bits[p.index()] |= 1 << m;
            }
        }
        let mut owner_bits = [0u16; 2];
        for (b, c) in owners.chars().enumerate() {
            if let Some(p) = parse_player(c)? {
                owner_bits[p.index()] |= 1 << b;
            }
        }
        let to_move = match to_move {
            "x" => Player::X,
            "o" => Player::O,
            _ => return Err(format!("bad player to move {to_move:?}")),
        };
        let sent = match sent {
            "-" => None,
            _ => match sent.parse::<u8>() {
                Ok(b) if b < 9 => Some(b),
                _ => return Err(format!("bad board sent to {sent:?}")),
            },
        };
        let occupied = cell_bits[0] | cell_bits[1];
        let full = (0..9)
            .filter(|&b| (occupied >> (9 * b)) as u16 & FULL == FULL)
            .fold(0u16, |acc, b| acc | 1 << b);
        Ok(Position::from_parts(
            cell_bits, owner_bits, full, to_move, sent,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mv;

    #[test]
    fn start_position() {
        let s = Position::new().to_string();
        assert_eq!(s, format!("{}/{}/x/-", ".".repeat(81), ".".repeat(9)));
        assert_eq!(s.parse::<Position>().unwrap(), Position::new());
    }

    #[test]
    fn round_trip() {
        let mut pos = Position::new();
        for (b, c) in [(0, 1), (1, 0), (0, 2), (2, 0), (0, 0), (0, 3)] {
            pos.play(mv(b, c));
        }
        let s = pos.to_string();
        assert!(s.ends_with("/x......../x/3"), "{s}");
        assert_eq!(s.parse::<Position>().unwrap(), pos);
    }

    #[test]
    fn rejects_garbage() {
        assert!("".parse::<Position>().is_err());
        assert!("x/y/z/w".parse::<Position>().is_err());
    }
}
