//! A transposition table of value bounds.
//!
//! Values are in `-1..=1` for the player to move, so an entry stores a lower
//! and an upper bound. Entries are always replaced on collision.

/// Marks a slot as holding an entry, so all-zero slots are empty.
const USED: u64 = 1 << 16;

fn pack(lower: i8, upper: i8) -> u64 {
    USED | (lower as u8 as u64) | (upper as u8 as u64) << 8
}

fn unpack(data: u64) -> (i8, i8) {
    (data as u8 as i8, (data >> 8) as u8 as i8)
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

    /// The stored `(lower, upper)` bounds for a position hash, if present.
    #[inline]
    pub fn probe(&self, key: u64) -> Option<(i8, i8)> {
        let [k, data] = self.entries[(key & self.mask) as usize];
        (k == key && data & USED != 0).then(|| unpack(data))
    }

    /// Stores bounds for a position hash, tightening any bounds already stored
    /// for the same position.
    #[inline]
    pub fn store(&mut self, key: u64, lower: i8, upper: i8) {
        let slot = &mut self.entries[(key & self.mask) as usize];
        if slot[0] == key && slot[1] & USED != 0 {
            let (l, u) = unpack(slot[1]);
            slot[1] = pack(l.max(lower), u.min(upper));
        } else {
            *slot = [key, pack(lower, upper)];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_entries_never_match() {
        let t = Table::new(4);
        assert_eq!(t.probe(0), None);
        assert_eq!(t.probe(17), None);
    }

    #[test]
    fn bounds_tighten() {
        let mut t = Table::new(4);
        t.store(5, -1, 0);
        assert_eq!(t.probe(5), Some((-1, 0)));
        t.store(5, 0, 1);
        assert_eq!(t.probe(5), Some((0, 0)));
        // A different key in the same slot replaces it.
        t.store(21, 1, 1);
        assert_eq!(t.probe(5), None);
        assert_eq!(t.probe(21), Some((1, 1)));
    }
}
