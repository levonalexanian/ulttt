//! Lines of a 3×3 grid, as 9-bit masks with bit `i` for index `i` (row-major).

/// The 8 lines (rows, columns, diagonals) of a 3×3 grid.
pub const LINES: [[u8; 3]; 8] = [
    [0, 1, 2],
    [3, 4, 5],
    [6, 7, 8],
    [0, 3, 6],
    [1, 4, 7],
    [2, 5, 8],
    [0, 4, 8],
    [2, 4, 6],
];

/// All 9 bits of a 3×3 grid.
pub const FULL: u16 = 0x1FF;

const fn line_mask(line: [u8; 3]) -> u16 {
    (1 << line[0]) | (1 << line[1]) | (1 << line[2])
}

const fn build_has_line() -> [bool; 512] {
    let mut table = [false; 512];
    let mut mask = 0;
    while mask < 512 {
        let mut i = 0;
        while i < LINES.len() {
            let l = line_mask(LINES[i]);
            if mask as u16 & l == l {
                table[mask] = true;
            }
            i += 1;
        }
        mask += 1;
    }
    table
}

static HAS_LINE: [bool; 512] = build_has_line();

/// Whether the set of grid indices `mask` contains a full line.
#[inline]
pub fn has_line(mask: u16) -> bool {
    HAS_LINE[(mask & FULL) as usize]
}

const fn build_line_avoiding() -> [bool; 512] {
    let mut table = [false; 512];
    let mut mask = 0;
    while mask < 512 {
        let mut i = 0;
        while i < LINES.len() {
            if mask as u16 & line_mask(LINES[i]) == 0 {
                table[mask] = true;
            }
            i += 1;
        }
        mask += 1;
    }
    table
}

static LINE_AVOIDING: [bool; 512] = build_line_avoiding();

const fn build_threats() -> [u16; 512] {
    let mut table = [0u16; 512];
    let mut mask = 0;
    while mask < 512 {
        let mut i = 0;
        while i < LINES.len() {
            let l = line_mask(LINES[i]);
            let have = mask as u16 & l;
            if have.count_ones() == 2 {
                table[mask] |= l & !have;
            }
            i += 1;
        }
        mask += 1;
    }
    table
}

/// For each set of indices, the indices that would complete a line with two
/// of them.
static THREATS: [u16; 512] = build_threats();

/// Empty indices where a player holding `own` (with the opponent holding
/// `opp`) would complete a line.
#[inline]
pub fn completing_cells(own: u16, opp: u16) -> u16 {
    THREATS[(own & FULL) as usize] & !(own | opp) & FULL
}

/// Whether some line contains none of the indices in `mask`, i.e. a player
/// whose opponent holds `mask` could still complete a line.
#[inline]
pub fn line_avoiding(mask: u16) -> bool {
    LINE_AVOIDING[(mask & FULL) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_are_lines() {
        for line in LINES {
            assert!(has_line(line_mask(line)));
        }
        assert!(!has_line(0));
        assert!(!has_line(0b000_000_011));
        // A drawn grid: x o x / x o o / o x x
        assert!(!has_line(0b110_001_101));
    }

    #[test]
    fn completing() {
        // x x . / . o . / . . . : x completes the top row at 2.
        assert_eq!(completing_cells(0b11, 0b1_0000), 0b100);
        // Blocked by o at 2.
        assert_eq!(completing_cells(0b11, 0b100), 0);
        // x at 0 and 4: completes the diagonal at 8.
        assert_eq!(completing_cells(0b1_0001, 0), 1 << 8);
    }
}
