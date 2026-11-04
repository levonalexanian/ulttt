use std::fmt;
use std::str::FromStr;

/// The two players. X moves first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Player {
    X = 0,
    O = 1,
}

impl Player {
    /// The opponent.
    #[inline]
    pub const fn other(self) -> Player {
        match self {
            Player::X => Player::O,
            Player::O => Player::X,
        }
    }

    /// Index into per-player arrays: X is 0, O is 1.
    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }

    pub const fn symbol(self) -> char {
        match self {
            Player::X => 'x',
            Player::O => 'o',
        }
    }
}

/// The result of a finished game.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Outcome {
    Win(Player),
    Draw,
}

impl Outcome {
    /// The score from X's point of view: 1, 0 or -1.
    pub const fn score(self) -> i8 {
        match self {
            Outcome::Win(Player::X) => 1,
            Outcome::Win(Player::O) => -1,
            Outcome::Draw => 0,
        }
    }
}

/// The supported rule variants. See `RULES.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Variant {
    /// Won or full local boards are closed.
    Classic,
    /// Won local boards stay playable until full (Bertholon et al., 2020).
    Open,
    /// `Classic`, but without a global line the player with more local boards wins.
    Majority,
}

impl Variant {
    pub const ALL: [Variant; 3] = [Variant::Classic, Variant::Open, Variant::Majority];

    pub const fn name(self) -> &'static str {
        match self {
            Variant::Classic => "classic",
            Variant::Open => "open",
            Variant::Majority => "majority",
        }
    }
}

impl fmt::Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Variant {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Variant::ALL
            .into_iter()
            .find(|v| v.name() == s)
            .ok_or_else(|| format!("unknown variant {s:?} (expected classic, open or majority)"))
    }
}
