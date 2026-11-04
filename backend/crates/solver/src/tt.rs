//! A transposition table of value bounds and best moves.
//!
//! Values are in `-1..=1` for the player to move, so an entry stores a lower
//! and an upper bound, plus the move that produced the best score, if any.
//! Entries are always replaced on collision.

use ulttt_game::Move;

/// Marks a slot as holding an entry, so all-zero slots are empty.
const USED: u64 = 1 << 16;

/// A stored entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub lower: i8,
    pub upper: i8,
    pub best: Option<Move>,
}

fn pack(e: Entry) -> u64 {
    USED | (e.lower as u8 as u64)
        | (e.upper as u8 as u64) << 8
        | (e.best.map_or(0, |m| m as u64 + 1)) << 24
}

fn unpack(data: u64) -> Entry {
    let best = (data >> 24) as u8;
    Entry {
        lower: data as u8 as i8,
        upper: (data >> 8) as u8 as i8,
        best: best.checked_sub(1),
    }
}

pub struct Table {
    /// `[key, data]` pairs. All zeros means empty, so the table can be
    /// allocated as zeroed memory and filled lazily by the OS.
    entries: Vec<[u64; 2]>,
    mask: u64,
}

impl Table {
    /// A table with `2^bits` entries.
    pub fn new(bits: u32) -> Table {
        Table {
            entries: vec![[0; 2]; 1 << bits],
            mask: (1 << bits) - 1,
        }
    }

    /// Empties the table.
    pub fn clear(&mut self) {
        self.entries.fill([0; 2]);
    }

    /// The stored entry for a position hash, if present.
    #[inline]
    pub fn probe(&self, key: u64) -> Option<Entry> {
        let [k, data] = self.entries[(key & self.mask) as usize];
        (k == key && data & USED != 0).then(|| unpack(data))
    }

    /// Stores an entry for a position hash. Bounds already stored for the same
    /// position are kept where tighter; the best move is replaced if given.
    #[inline]
    pub fn store(&mut self, key: u64, e: Entry) {
        let slot = &mut self.entries[(key & self.mask) as usize];
        if slot[0] == key && slot[1] & USED != 0 {
            let old = unpack(slot[1]);
            slot[1] = pack(Entry {
                lower: old.lower.max(e.lower),
                upper: old.upper.min(e.upper),
                best: e.best.or(old.best),
            });
        } else {
            *slot = [key, pack(e)];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(lower: i8, upper: i8, best: Option<Move>) -> Entry {
        Entry { lower, upper, best }
    }

    #[test]
    fn empty_entries_never_match() {
        let t = Table::new(4);
        assert_eq!(t.probe(0), None);
        assert_eq!(t.probe(17), None);
    }

    #[test]
    fn bounds_tighten() {
        let mut t = Table::new(4);
        t.store(5, entry(-1, 0, Some(80)));
        assert_eq!(t.probe(5), Some(entry(-1, 0, Some(80))));
        t.store(5, entry(0, 1, None));
        assert_eq!(t.probe(5), Some(entry(0, 0, Some(80))));
        t.store(5, entry(0, 1, Some(0)));
        assert_eq!(t.probe(5), Some(entry(0, 0, Some(0))));
        // A different key in the same slot replaces it.
        t.store(21, entry(1, 1, None));
        assert_eq!(t.probe(5), None);
        assert_eq!(t.probe(21), Some(entry(1, 1, None)));
    }
}
