use crate::lines::{FULL, has_line, line_avoiding};
use crate::types::{Outcome, Player, Variant};

/// A move is one of the 81 cells: cell `m % 9` of local board `m / 9`.
pub type Move = u8;

/// The move at cell `cell` of local board `board`.
#[inline]
pub const fn mv(board: u8, cell: u8) -> Move {
    9 * board + cell
}

/// The local board a move is played on.
#[inline]
pub const fn board_of(m: Move) -> u8 {
    m / 9
}

/// The cell a move is played on; the opponent is sent to the local board with this index.
#[inline]
pub const fn cell_of(m: Move) -> u8 {
    m % 9
}

/// Bits of all 81 cells.
pub const ALL_CELLS: u128 = (1 << 81) - 1;

/// Value of `sent` before the first move.
const NOT_SENT: u8 = 9;

/// The cells of each local board as an 81-bit mask, for each 9-bit set of boards.
static BOARDS_TO_CELLS: [u128; 512] = {
    let mut table = [0u128; 512];
    let mut mask = 0;
    while mask < 512 {
        let mut b = 0;
        while b < 9 {
            if mask & (1 << b) != 0 {
                table[mask] |= (FULL as u128) << (9 * b);
            }
            b += 1;
        }
        mask += 1;
    }
    table
};

/// A position. Mirrors `State` in `proofs/Ulttt/State.lean`.
///
/// Ownership of local boards is stored, not derived from the cells: in the `open`
/// variant a won board stays playable and its owner is whoever completed a line
/// on it first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Position {
    /// Occupied cells of each player; bit `m` is move `m`.
    cells: [u128; 2],
    /// Local boards owned by each player; bit `b` is board `b`.
    owner: [u16; 2],
    /// Local boards with all 9 cells occupied.
    full: u16,
    to_move: Player,
    /// The board the player to move was sent to, or `NOT_SENT`.
    sent: u8,
}

impl Default for Position {
    fn default() -> Self {
        Self::new()
    }
}

impl Position {
    /// The empty starting position, X to move.
    pub const fn new() -> Self {
        Position {
            cells: [0, 0],
            owner: [0, 0],
            full: 0,
            to_move: Player::X,
            sent: NOT_SENT,
        }
    }

    /// Builds a position from its raw parts. `full` must match `cells`.
    pub(crate) const fn from_parts(
        cells: [u128; 2],
        owner: [u16; 2],
        full: u16,
        to_move: Player,
        sent: Option<u8>,
    ) -> Self {
        Position {
            cells,
            owner,
            full,
            to_move,
            sent: match sent {
                Some(b) => b,
                None => NOT_SENT,
            },
        }
    }

    /// The raw parts: cells, owners, full boards, player to move, board sent to.
    #[allow(clippy::type_complexity)]
    pub(crate) fn parts(&self) -> ([u128; 2], [u16; 2], u16, Player, Option<u8>) {
        (self.cells, self.owner, self.full, self.to_move, self.sent())
    }

    /// A total order on positions, used to pick canonical representatives.
    #[inline]
    pub(crate) fn sort_key(&self) -> (u128, u128, u16, u16, u8) {
        (
            self.cells[0],
            self.cells[1],
            self.owner[0],
            self.owner[1],
            self.sent,
        )
    }

    /// A 64-bit hash of the position, for transposition tables. The player to
    /// move is implied by the cells, so it is not mixed in separately.
    #[inline]
    pub fn hash64(&self) -> u64 {
        #[inline]
        fn mix(x: u64) -> u64 {
            let x = (x ^ (x >> 32)).wrapping_mul(0xD6E8_FEB8_6659_FD93);
            let x = (x ^ (x >> 32)).wrapping_mul(0xD6E8_FEB8_6659_FD93);
            x ^ (x >> 32)
        }
        let meta = self.owner[0] as u64 | (self.owner[1] as u64) << 16 | (self.sent as u64) << 32;
        let mut h = mix(meta ^ 0x9E37_79B9_7F4A_7C15);
        h = mix(h ^ self.cells[0] as u64);
        h = mix(h ^ (self.cells[0] >> 64) as u64);
        h = mix(h ^ self.cells[1] as u64);
        mix(h ^ (self.cells[1] >> 64) as u64)
    }

    #[inline]
    pub fn to_move(&self) -> Player {
        self.to_move
    }

    /// The local board the player to move was sent to; `None` before the first move.
    #[inline]
    pub fn sent(&self) -> Option<u8> {
        (self.sent != NOT_SENT).then_some(self.sent)
    }

    /// The contents of a cell.
    #[inline]
    pub fn cell(&self, m: Move) -> Option<Player> {
        if self.cells[0] >> m & 1 != 0 {
            Some(Player::X)
        } else if self.cells[1] >> m & 1 != 0 {
            Some(Player::O)
        } else {
            None
        }
    }

    /// Occupied cells of a player as an 81-bit mask.
    #[inline]
    pub fn cells_of(&self, p: Player) -> u128 {
        self.cells[p.index()]
    }

    /// Occupied cells as an 81-bit mask.
    #[inline]
    pub fn occupied(&self) -> u128 {
        self.cells[0] | self.cells[1]
    }

    /// Number of empty cells.
    #[inline]
    pub fn empties(&self) -> u32 {
        81 - self.occupied().count_ones()
    }

    /// The owner of a local board.
    #[inline]
    pub fn owner(&self, board: u8) -> Option<Player> {
        if self.owner[0] >> board & 1 != 0 {
            Some(Player::X)
        } else if self.owner[1] >> board & 1 != 0 {
            Some(Player::O)
        } else {
            None
        }
    }

    /// Local boards owned by a player as a 9-bit mask.
    #[inline]
    pub fn owned_by(&self, p: Player) -> u16 {
        self.owner[p.index()]
    }

    /// Local boards with every cell occupied, as a 9-bit mask.
    #[inline]
    pub fn full_boards(&self) -> u16 {
        self.full
    }

    /// Local boards that can no longer be played in, as a 9-bit mask.
    #[inline]
    pub fn closed(&self, v: Variant) -> u16 {
        match v {
            Variant::Open => self.full,
            Variant::Classic | Variant::Majority => self.owner[0] | self.owner[1] | self.full,
        }
    }

    /// Local boards that player `p` owns or might still win, as a 9-bit mask: an
    /// over-approximation. An unowned board can still be won if it is not
    /// closed and some line on it holds no opponent mark.
    pub fn winnable_boards(&self, v: Variant, p: Player) -> u16 {
        let opp = self.cells[p.other().index()];
        let mut mask = self.owner[p.index()];
        let candidates = !(self.owner[0] | self.owner[1] | self.closed(v)) & FULL;
        for b in 0..9u8 {
            if candidates >> b & 1 != 0 && line_avoiding(Self::local(opp, b)) {
                mask |= 1 << b;
            }
        }
        mask
    }

    /// Whether player `p` might still win the game: `false` means `p` can do no
    /// better than a draw. Sound but not exact: `true` does not mean `p` can
    /// force a win, or even that some sequence of moves leads to one.
    pub fn may_win(&self, v: Variant, p: Player) -> bool {
        let winnable = self.winnable_boards(v, p);
        if has_line(winnable) {
            return true;
        }
        // In `majority` a player can also win on board count, as long as they
        // can end up with more boards than the opponent already owns.
        v == Variant::Majority && winnable.count_ones() > self.won_count(p.other())
    }

    /// Whether player `p` owns three local boards in a line.
    #[inline]
    pub fn global_line(&self, p: Player) -> bool {
        has_line(self.owner[p.index()])
    }

    /// Number of local boards owned by `p`.
    #[inline]
    pub fn won_count(&self, p: Player) -> u32 {
        self.owner[p.index()].count_ones()
    }

    /// Whether the game is over: someone has a global line or nothing is playable.
    #[inline]
    pub fn is_terminal(&self, v: Variant) -> bool {
        self.global_line(Player::X) || self.global_line(Player::O) || self.closed(v) == FULL
    }

    /// The local boards the player to move may play on, as a 9-bit mask.
    /// Assumes the game is not over.
    #[inline]
    pub fn allowed_boards(&self, v: Variant) -> u16 {
        let open = !self.closed(v) & FULL;
        if self.sent != NOT_SENT && open >> self.sent & 1 != 0 {
            1 << self.sent
        } else {
            open
        }
    }

    /// All legal moves as an 81-bit mask; empty when the game is over.
    #[inline]
    pub fn legal_moves(&self, v: Variant) -> u128 {
        if self.is_terminal(v) {
            return 0;
        }
        BOARDS_TO_CELLS[self.allowed_boards(v) as usize] & !self.occupied()
    }

    /// Whether `m` is a legal move.
    #[inline]
    pub fn is_legal(&self, v: Variant, m: Move) -> bool {
        m < 81 && self.legal_moves(v) >> m & 1 != 0
    }

    /// The local board `board` of player `p` as a 9-bit mask.
    #[inline]
    fn local(cells: u128, board: u8) -> u16 {
        (cells >> (9 * board)) as u16 & FULL
    }

    /// Plays `m` for the player to move. The same in every variant; only which
    /// moves are legal differs. Assumes `m` is legal.
    #[inline]
    pub fn play(&mut self, m: Move) {
        let p = self.to_move.index();
        let b = board_of(m);
        self.cells[p] |= 1 << m;
        if (self.owner[0] | self.owner[1]) >> b & 1 == 0 && has_line(Self::local(self.cells[p], b))
        {
            self.owner[p] |= 1 << b;
        }
        if Self::local(self.occupied(), b) == FULL {
            self.full |= 1 << b;
        }
        self.to_move = self.to_move.other();
        self.sent = cell_of(m);
    }

    /// The position after playing `m`.
    #[inline]
    pub fn after(&self, m: Move) -> Position {
        let mut next = *self;
        next.play(m);
        next
    }

    /// The result of a finished game. Only meaningful when the game is over.
    pub fn outcome(&self, v: Variant) -> Outcome {
        if self.global_line(Player::X) {
            Outcome::Win(Player::X)
        } else if self.global_line(Player::O) {
            Outcome::Win(Player::O)
        } else if v == Variant::Majority {
            let (x, o) = (self.won_count(Player::X), self.won_count(Player::O));
            match x.cmp(&o) {
                std::cmp::Ordering::Greater => Outcome::Win(Player::X),
                std::cmp::Ordering::Less => Outcome::Win(Player::O),
                std::cmp::Ordering::Equal => Outcome::Draw,
            }
        } else {
            Outcome::Draw
        }
    }

    /// The result if the game is over, `None` otherwise.
    pub fn result(&self, v: Variant) -> Option<Outcome> {
        self.is_terminal(v).then(|| self.outcome(v))
    }
}

/// Iterates over the set bits of an 81-bit move mask, lowest first.
#[derive(Clone, Copy, Debug)]
pub struct Moves(pub u128);

impl Iterator for Moves {
    type Item = Move;

    #[inline]
    fn next(&mut self) -> Option<Move> {
        if self.0 == 0 {
            return None;
        }
        let m = self.0.trailing_zeros() as Move;
        self.0 &= self.0 - 1;
        Some(m)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.0.count_ones() as usize;
        (n, Some(n))
    }
}

impl ExactSizeIterator for Moves {}
