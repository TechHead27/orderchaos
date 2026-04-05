pub const BOARD_SIDE: u8 = 6;

// Columns 'a'–'f' are accepted as move input; any other letter is rejected.
pub const UPPER_RANGE: (u32, u32) = ('A' as u32, 'F' as u32);
pub const LOWER_RANGE: (u32, u32) = ('a' as u32, 'f' as u32);

// ---- Bitboard layout ----
// The board is stored column-major: bit index = col * BOARD_SIDE + row, both 0-based.
// Columns a–f → col 0–5; rows 1–6 → row 0–5.
// Example mapping: a1 = bit 0, a6 = bit 5, b1 = bit 6, f6 = bit 35.
//
// Visual (bit indices):
//     col  a   b   c   d   e   f
// row 1:   0   6  12  18  24  30
// row 2:   1   7  13  19  25  31
// row 3:   2   8  14  20  26  32
// row 4:   3   9  15  21  27  33
// row 5:   4  10  16  22  28  34
// row 6:   5  11  17  23  29  35

// Selects all 6 rows of a single column when shifted to that column's base bit.
// = 0b111111 (bits 0–5).
pub const COL_MASK: u64 = (1 << BOARD_SIDE) - 1;

// Selects row 1 of every column: bits 0, 6, 12, 18, 24, 30 (a1, b1, c1, d1, e1, f1).
// Shifting left by r yields the mask for row r+1 across all columns.
pub const ROW_MASK: u64 = construct_row_mask();

// The two winning column patterns (five consecutive bits within a 6-bit column).
// WINNING_COL_LOW  = rows 1–5 (bits 0–4): five-in-a-column not touching row 6.
// WINNING_COL_HIGH = rows 2–6 (bits 1–5): five-in-a-column not touching row 1.
pub const WINNING_COL_LOW: u64 = 0b011111;
pub const WINNING_COL_HIGH: u64 = 0b111110;

// The two winning row patterns, expressed in row-0 coordinates (before the per-row
// right-shift applied in set_finished).
// WINNING_ROW_LOW  = columns a–e: bits 0, 6, 12, 18, 24.
// WINNING_ROW_HIGH = columns b–f: bits 6, 12, 18, 24, 30.
//
// Derivation of WINNING_ROW_HIGH:
//   ROW_MASK & ((1 << 36) - 2)  →  clears bit 0 of ROW_MASK, keeping bits 6–30.
//
// Derivation of WINNING_ROW_LOW:
//   `1 << (BOARD_SIDE * (BOARD_SIDE - 1)) - 1`  =  `1 << (30 - 1)`  =  `1 << 29`
//   Then `ROW_MASK & (1<<29) - 1`  =  `ROW_MASK & 0x1FFFFFFF`  (clears bit 30).
pub const WINNING_ROW_HIGH: u64 = ROW_MASK & ((1 << (BOARD_SIDE * BOARD_SIDE)) - 2);
pub const WINNING_ROW_LOW: u64 = ROW_MASK & ((1 << ((BOARD_SIDE * (BOARD_SIDE - 1)) - 1)) - 1);

// The main diagonal (a1→f6): bits 0, 7, 14, 21, 28, 35.  Step = BOARD_SIDE + 1.
pub const MAIN_LR_DIAG_MASK: u64 = construct_diag_masks()[0];
// Off-diagonal shifted one row down (a2→e6): bits 1, 8, 15, 22, 29.
// Note: construct_diag_masks leaves a stray bit 36 in this mask (the loop overshoots by
// one cell). Bit 36 is beyond the 6×6 board, so it is never set in x_board or o_board;
// the count_ones() == 5 check in set_finished remains correct despite the extra bit.
pub const LOWER_LR_DIAG_MASK: u64 = construct_diag_masks()[1];
// Off-diagonal shifted one column right (b1→f5): bits 6, 13, 20, 27, 34.
pub const UPPER_LR_DIAG_MASK: u64 = construct_diag_masks()[2];

// The two winning subsets of the main diagonal (5 of its 6 cells).
// WINNING_DIAG_LOW  = a1–e5: bits 0, 7, 14, 21, 28  (clears f6 = bit 35).
// WINNING_DIAG_HIGH = b2–f6: bits 7, 14, 21, 28, 35  (clears a1 = bit 0).
pub const WINNING_LR_DIAG_LOW: u64 = MAIN_LR_DIAG_MASK & !(1 << ((BOARD_SIDE * BOARD_SIDE) - 1));
pub const WINNING_LR_DIAG_HIGH: u64 = MAIN_LR_DIAG_MASK & !1;

// Compile-time sanity checks — verify every mask against an explicit bit enumeration.
const _: () = assert!(COL_MASK == 0b111111);
const _: () =
    assert!(ROW_MASK == (1 | (1 << 6) | (1 << 12) | (1 << 18) | (1 << 24) | (1u64 << 30)));
const _: () =
    assert!(WINNING_ROW_LOW == ((1 << 0) | (1 << 6) | (1 << 12) | (1 << 18) | (1u64 << 24)));
const _: () =
    assert!(WINNING_ROW_HIGH == ((1 << 6) | (1 << 12) | (1 << 18) | (1 << 24) | (1u64 << 30)));
const _: () =
    assert!(MAIN_LR_DIAG_MASK == (1 | (1 << 7) | (1 << 14) | (1 << 21) | (1 << 28) | (1u64 << 35)));
const _: () = assert!(WINNING_LR_DIAG_LOW == (1 | (1 << 7) | (1 << 14) | (1 << 21) | (1u64 << 28)));
const _: () =
    assert!(WINNING_LR_DIAG_HIGH == ((1 << 7) | (1 << 14) | (1 << 21) | (1 << 28) | (1u64 << 35)));
const _: () =
    assert!(LOWER_LR_DIAG_MASK == ((1 << 1) | (1 << 8) | (1 << 15) | (1 << 22) | (1u64 << 29)));
const _: () =
    assert!(UPPER_LR_DIAG_MASK == ((1 << 6) | (1 << 13) | (1 << 20) | (1 << 27) | (1u64 << 34)));

const fn construct_diag_masks() -> [u64; 3] {
    // 3 diagonals
    let mut masks = [0; 3];

    // Lower diagonal
    let mut bit = 1;
    let mut i = 0;

    while i < BOARD_SIDE {
        masks[0] |= bit;
        masks[1] |= bit << 1;
        masks[2] |= bit << BOARD_SIDE;
        bit <<= BOARD_SIDE + 1;
        i += 1;
    }

    masks[1] &= !(1u64 << (BOARD_SIDE * BOARD_SIDE));
    masks[2] &= !(1u64 << (BOARD_SIDE * BOARD_SIDE + (BOARD_SIDE - 1)));

    masks
}

const fn construct_row_mask() -> u64 {
    let mut mask = 1;
    let mut bit = 1;
    let mut i = 0;

    while i < BOARD_SIDE {
        mask |= bit;
        bit <<= BOARD_SIDE;
        i += 1;
    }

    mask
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_construct_row_mask() {
        assert_eq!(construct_row_mask(), 0b000001000001000001000001000001000001);
    }

    #[test]
    fn test_construct_diag_masks() {
        let masks = construct_diag_masks();
        // Main diagonal: a1, b2, c3, d4, e5, f6 (bits 0, 7, 14, 21, 28, 35).
        assert_eq!(
            masks[0],
            1 | (1 << 7) | (1 << 14) | (1 << 21) | (1 << 28) | (1u64 << 35)
        );
        // Lower off-diagonal: a2, b3, c4, d5, e6 (bits 1, 8, 15, 22, 29).
        assert_eq!(
            masks[1],
            (1 << 1) | (1 << 8) | (1 << 15) | (1 << 22) | (1u64 << 29)
        );
        // Upper off-diagonal: b1, c2, d3, e4, f5 (bits 6, 13, 20, 27, 34).
        assert_eq!(
            masks[2],
            (1 << 6) | (1 << 13) | (1 << 20) | (1 << 27) | (1u64 << 34)
        );
    }
}
