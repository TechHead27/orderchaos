// This file contains the game engine itself, including representation of the game state and
// utility functions like determining a winner.

const BOARD_SIDE: u8 = 6;

// Columns 'a'–'f' are accepted as move input; any other letter is rejected.
const UPPER_RANGE: (u32, u32) = ('A' as u32, 'F' as u32);
const LOWER_RANGE: (u32, u32) = ('a' as u32, 'f' as u32);

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
const COL_MASK: u64 = (1 << BOARD_SIDE) - 1;

// Selects row 1 of every column: bits 0, 6, 12, 18, 24, 30 (a1, b1, c1, d1, e1, f1).
// Shifting left by r yields the mask for row r+1 across all columns.
const ROW_MASK: u64 = construct_row_mask();

// The two winning column patterns (five consecutive bits within a 6-bit column).
// WINNING_COL_LOW  = rows 1–5 (bits 0–4): five-in-a-column not touching row 6.
// WINNING_COL_HIGH = rows 2–6 (bits 1–5): five-in-a-column not touching row 1.
const WINNING_COL_LOW: u64 = 0b011111;
const WINNING_COL_HIGH: u64 = 0b111110;

// The two winning row patterns, expressed in row-0 coordinates (before the per-row
// right-shift applied in set_finished).
// WINNING_ROW_LOW  = columns a–e: bits 0, 6, 12, 18, 24.
// WINNING_ROW_HIGH = columns b–f: bits 6, 12, 18, 24, 30.
//
// Derivation of WINNING_ROW_HIGH:
//   ROW_MASK & ((1 << 36) - 2)  →  clears bit 0 of ROW_MASK, keeping bits 6–30.
//
// Derivation of WINNING_ROW_LOW (operator precedence note):
//   `-` binds tighter than `<<` in Rust, so:
//   `1 << (BOARD_SIDE * (BOARD_SIDE - 1)) - 1`  =  `1 << (30 - 1)`  =  `1 << 29`
//   Then `ROW_MASK & (1<<29) - 1`  =  `ROW_MASK & 0x1FFFFFFF`  (clears bit 30).
const WINNING_ROW_HIGH: u64 = ROW_MASK & ((1 << BOARD_SIDE * BOARD_SIDE) - 2);
const WINNING_ROW_LOW: u64 = ROW_MASK & (1 << (BOARD_SIDE * (BOARD_SIDE - 1)) - 1) - 1;

// The main diagonal (a1→f6): bits 0, 7, 14, 21, 28, 35.  Step = BOARD_SIDE + 1.
const MAIN_DIAG_MASK: u64 = construct_diag_masks()[0];
// Off-diagonal shifted one row down (a2→e6): bits 1, 8, 15, 22, 29.
// Note: construct_diag_masks leaves a stray bit 36 in this mask (the loop overshoots by
// one cell). Bit 36 is beyond the 6×6 board, so it is never set in x_board or o_board;
// the count_ones() == 5 check in set_finished remains correct despite the extra bit.
const LOWER_DIAG_MASK: u64 = construct_diag_masks()[1];
// Off-diagonal shifted one column right (b1→f5): bits 6, 13, 20, 27, 34.
const UPPER_DIAG_MASK: u64 = construct_diag_masks()[2];

// The two winning subsets of the main diagonal (5 of its 6 cells).
// WINNING_DIAG_LOW  = a1–e5: bits 0, 7, 14, 21, 28  (clears f6 = bit 35).
// WINNING_DIAG_HIGH = b2–f6: bits 7, 14, 21, 28, 35  (clears a1 = bit 0).
//
// Operator precedence note for WINNING_DIAG_LOW:
//   `1 << (BOARD_SIDE * BOARD_SIDE) - 1`  =  `1 << (36 - 1)`  =  `1 << 35`  (bit 35 = f6).
const WINNING_DIAG_LOW: u64 = MAIN_DIAG_MASK & !(1 << (BOARD_SIDE * BOARD_SIDE) - 1);
const WINNING_DIAG_HIGH: u64 = MAIN_DIAG_MASK & !1;

// Compile-time sanity checks — verify every mask against an explicit bit enumeration.
const _: () = assert!(COL_MASK == 0b111111);
const _: () =
    assert!(ROW_MASK == (1 | (1 << 6) | (1 << 12) | (1 << 18) | (1 << 24) | (1u64 << 30)));
const _: () =
    assert!(WINNING_ROW_LOW == ((1 << 0) | (1 << 6) | (1 << 12) | (1 << 18) | (1u64 << 24)));
const _: () =
    assert!(WINNING_ROW_HIGH == ((1 << 6) | (1 << 12) | (1 << 18) | (1 << 24) | (1u64 << 30)));
const _: () =
    assert!(MAIN_DIAG_MASK == (1 | (1 << 7) | (1 << 14) | (1 << 21) | (1 << 28) | (1u64 << 35)));
const _: () = assert!(WINNING_DIAG_LOW == (1 | (1 << 7) | (1 << 14) | (1 << 21) | (1u64 << 28)));
const _: () =
    assert!(WINNING_DIAG_HIGH == ((1 << 7) | (1 << 14) | (1 << 21) | (1 << 28) | (1u64 << 35)));
const _: () =
    assert!(LOWER_DIAG_MASK == ((1 << 1) | (1 << 8) | (1 << 15) | (1 << 22) | (1u64 << 29)));
const _: () =
    assert!(UPPER_DIAG_MASK == ((1 << 6) | (1 << 13) | (1 << 20) | (1 << 27) | (1u64 << 34)));

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

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum Space {
    X,
    O,
}

impl TryFrom<char> for Space {
    type Error = &'static str;

    /// Converts a character into a [`Space`] variant.
    ///
    /// # Arguments
    ///
    /// * `c` - A character representing a piece: `'x'` or `'X'` for X, `'o'` or `'O'` for O.
    ///
    /// # Returns
    ///
    /// `Ok(Space::X)` or `Ok(Space::O)` on success.
    ///
    /// # Errors
    ///
    /// Returns `Err("Unrecognized piece")` if `c` is not a recognised piece character.
    fn try_from(c: char) -> Result<Self, Self::Error> {
        match c {
            'x' | 'X' => Ok(Space::X),
            'o' | 'O' => Ok(Space::O),
            _ => Err("Unrecognized piece"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Game {
    x_board: u64,
    o_board: u64,
    order_turn: bool,
    finished: bool,
}

/// Parses a three-character move string into its components.
///
/// The expected format is `"<col><row><piece>"`, for example `"a2x"` to place an X
/// at column a, row 2. Column letters are case-insensitive (`'a'`–`'f'` / `'A'`–`'F'`
/// for a 6×6 board) and are converted to a 0-based index. Row must be a digit `'1'`–`'6'`.
/// Piece must be `'x'`/`'X'` or `'o'`/`'O'`.
///
/// # Arguments
///
/// * `move_string` - A string of exactly three characters encoding the column, row, and piece.
///
/// # Returns
///
/// `Ok((piece, col, row))` where `piece` is the [`Space`] to place, `col` is the 0-based
/// column index, and `row` is the 0-based row number.
///
/// # Errors
///
/// Returns a `&'static str` error in any of the following cases:
/// * The string is not exactly three characters.
/// * The column character is not a letter (`'a'`–`'z'` or `'A'`–`'Z'`).
/// * The row value exceeds [`BOARD_SIDE`].
/// * The piece character is not `'x'`, `'X'`, `'o'`, or `'O'`.
fn parse_move_string(move_string: &str) -> Result<(Space, u8, u8), &'static str> {
    let mut chars = move_string.chars();

    let col = chars.next().ok_or("Invalid length")? as u32;
    let row = chars.next().ok_or("Invalid length").and_then(|c| {
        c.to_digit(10)
            .and_then(|x| x.checked_sub(1))
            .ok_or("Invalid row value")
    })? as u8;
    let piece = chars
        .next()
        .ok_or("Invalid length")
        .and_then(Space::try_from)?;

    if chars.next().is_some() {
        return Err("Move string should be three characters.");
    }

    if (col < LOWER_RANGE.0 || col > LOWER_RANGE.1) && (col < UPPER_RANGE.0 || col > UPPER_RANGE.1)
    {
        return Err("Invalid column value");
    }

    let col_val: u8 = if col < LOWER_RANGE.0 {
        // uppercase
        (col - UPPER_RANGE.0) as u8
    } else {
        // lowercase
        (col - LOWER_RANGE.0) as u8
    };

    if row >= BOARD_SIDE {
        return Err("Invalid row value");
    }

    Ok((piece, col_val, row))
}

impl Game {
    /// Creates a new game with an empty board and Order moving first.
    ///
    /// # Returns
    ///
    /// A [`Game`] with all squares empty, `order_turn` set to `true`, and `finished` set to `false`.
    pub fn new() -> Self {
        Game {
            x_board: 0,
            o_board: 0,
            order_turn: true,
            finished: false,
        }
    }

    /// Applies a move to the game state.
    ///
    /// Parses `move_string`, places the chosen piece on the board, checks for a finished
    /// game, and advances the turn to the other player if the game is not over.
    ///
    /// # Arguments
    ///
    /// * `move_string` - A move in the format `"<col><row><piece>"` (e.g., `"a2x"`).
    ///   See [`parse_move_string`] for the full format description.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if the move ended the game, `Ok(false)` if the game continues.
    ///
    /// # Errors
    ///
    /// Returns a `&str` error if the move string is invalid, the target square is already occupied,
    /// or the game is already over.
    pub fn process_move(&mut self, move_string: &str) -> Result<bool, &str> {
        if self.finished {
            return Err("Game is already finished");
        }

        let (piece, col, row) = parse_move_string(move_string)?;
        let offset: u64 = 1 << (col * BOARD_SIDE + row);

        if (self.x_board | self.o_board) & offset > 0 {
            return Err("Space is not free");
        }

        match piece {
            Space::O => self.o_board |= offset,
            Space::X => self.x_board |= offset,
        };

        self.set_finished();

        if !self.finished {
            self.order_turn = !self.order_turn;
        }

        Ok(self.finished)
    }

    /// Checks the board and updates [`Game::finished`] if the game has ended.
    ///
    /// The game is finished when Order achieves exactly five pieces in a row
    /// (horizontally, vertically, or diagonally) or when every square is occupied
    /// without such a sequence (a win for Chaos). Sets `self.finished = true`
    /// when either condition is met.
    fn set_finished(&mut self) {
        // Check win for order
        // Check columns
        let mut col_mask = COL_MASK;
        for col in 0..BOARD_SIDE {
            let x_col_vals = (self.x_board & col_mask) >> (col * BOARD_SIDE);
            let o_col_vals = (self.o_board & col_mask) >> (col * BOARD_SIDE);
            if x_col_vals == WINNING_COL_LOW
                || x_col_vals == WINNING_COL_HIGH
                || o_col_vals == WINNING_COL_LOW
                || o_col_vals == WINNING_COL_HIGH
            {
                self.finished = true;
                return;
            }

            col_mask <<= BOARD_SIDE;
        }

        // Check rows
        let mut row_mask = ROW_MASK;
        for row in 0..BOARD_SIDE {
            let x_row_vals = (self.x_board & row_mask) >> row;
            let o_row_vals = (self.o_board & row_mask) >> row;
            if x_row_vals == WINNING_ROW_HIGH
                || x_row_vals == WINNING_ROW_LOW
                || o_row_vals == WINNING_ROW_HIGH
                || o_row_vals == WINNING_ROW_LOW
            {
                self.finished = true;
                return;
            }

            row_mask <<= 1;
        }

        // Check diagonals
        // 4 possible winning states (for each piece)

        // Lower diagonal
        if (self.x_board & LOWER_DIAG_MASK).count_ones() == 5
            || (self.o_board & LOWER_DIAG_MASK).count_ones() == 5
        {
            self.finished = true;
            return;
        }

        // Upper diagonal
        if (self.x_board & UPPER_DIAG_MASK).count_ones() == 5
            || (self.o_board & UPPER_DIAG_MASK).count_ones() == 5
        {
            self.finished = true;
            return;
        }

        // Middle diagonal (both ways)
        if self.x_board & MAIN_DIAG_MASK == WINNING_DIAG_HIGH
            || self.o_board & MAIN_DIAG_MASK == WINNING_DIAG_HIGH
        {
            self.finished = true;
            return;
        }

        if self.x_board & MAIN_DIAG_MASK == WINNING_DIAG_LOW
            || self.o_board & MAIN_DIAG_MASK == WINNING_DIAG_LOW
        {
            self.finished = true;
            return;
        }

        // Check win for chaos
        if self.x_board.count_ones() + self.o_board.count_ones() == (BOARD_SIDE * BOARD_SIDE) as u32
        {
            self.finished = true;
            return;
        }
    }

    /// Returns the piece occupying the given cell, if any.
    ///
    /// # Arguments
    ///
    /// * `col` - 0-based column index (0 = 'a').
    /// * `row` - 0-based row index (0 = row 1).
    ///
    /// # Returns
    ///
    /// `Some('X')`, `Some('O')`, or `None` for an empty square.
    pub fn piece_at(&self, col: u8, row: u8) -> Option<char> {
        let bit = 1u64 << (col * BOARD_SIDE + row);
        if self.x_board & bit != 0 {
            Some('X')
        } else if self.o_board & bit != 0 {
            Some('O')
        } else {
            None
        }
    }

    /// Returns `true` if it is currently Order's turn.
    pub fn is_order_turn(&self) -> bool {
        self.order_turn
    }

    /// Returns `true` if the game has ended.
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Returns `true` if every square is occupied.
    ///
    /// When the game is finished and the board is full, Chaos has won.
    pub fn is_board_full(&self) -> bool {
        (self.x_board | self.o_board).count_ones() == (BOARD_SIDE as u32 * BOARD_SIDE as u32)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_move_string_good() {
        // Lowercase column, X piece
        assert_eq!(parse_move_string("a1x"), Ok((Space::X, 0, 0)));
        // Lowercase column, O piece
        assert_eq!(parse_move_string("b3o"), Ok((Space::O, 1, 2)));
        // Uppercase column, uppercase piece
        assert_eq!(parse_move_string("A1X"), Ok((Space::X, 0, 0)));
        // Last valid column for a 6x6 board (f = index 5)
        assert_eq!(parse_move_string("f6o"), Ok((Space::O, 5, 5)));
    }

    #[test]
    fn test_parse_move_string_bad() {
        // Too short
        assert!(parse_move_string("a1").is_err());
        // Too long
        assert!(parse_move_string("a1xx").is_err());
        // Empty string
        assert!(parse_move_string("").is_err());
        // Invalid piece character
        assert!(parse_move_string("a1z").is_err());
        // Non-letter column
        assert!(parse_move_string("11x").is_err());
        // Row out of range for a 6x6 board
        assert!(parse_move_string("a7x").is_err());
        // Row out of range at 0
        assert!(parse_move_string("a0x").is_err());
    }

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

    // ---- process_move error cases ----

    #[test]
    fn test_process_move_error_invalid_string() {
        let mut game = Game::new();
        assert!(game.process_move("a1").is_err());
    }

    #[test]
    fn test_process_move_error_occupied() {
        let mut game = Game::new();
        game.process_move("a1x").unwrap();
        assert!(game.process_move("a1o").is_err());
    }

    #[test]
    fn test_process_move_error_already_finished() {
        let mut game = Game::new();
        game.finished = true;
        assert!(game.process_move("a1x").is_err());
    }

    // ---- process_move behaviour ----

    #[test]
    fn test_process_move_places_piece() {
        let mut game = Game::new();
        assert_eq!(game.process_move("a1x"), Ok(false));
        assert_eq!(game.piece_at(0, 0), Some('X'));
        assert!(!game.is_order_turn());
        assert!(!game.is_finished());
    }

    #[test]
    fn test_process_move_alternates_turns() {
        let mut game = Game::new();
        assert!(game.is_order_turn());
        game.process_move("a1x").unwrap();
        assert!(!game.is_order_turn());
        game.process_move("b2o").unwrap();
        assert!(game.is_order_turn());
    }

    #[test]
    fn test_process_move_sequence_column_win() {
        // Order builds five X's down column a (rows 1–5); Chaos plays O's in column b.
        // Board layout is column-major: col * 6 + row, so a1=bit 0, a2=bit 1, …
        let mut game = Game::new();
        assert_eq!(game.process_move("a1x"), Ok(false));
        assert_eq!(game.process_move("b1o"), Ok(false));
        assert_eq!(game.process_move("a2x"), Ok(false));
        assert_eq!(game.process_move("b2o"), Ok(false));
        assert_eq!(game.process_move("a3x"), Ok(false));
        assert_eq!(game.process_move("b3o"), Ok(false));
        assert_eq!(game.process_move("a4x"), Ok(false));
        assert_eq!(game.process_move("b4o"), Ok(false));
        // Fifth X in column a completes five-in-a-row.
        assert_eq!(game.process_move("a5x"), Ok(true));
        assert!(game.is_finished());
    }

    // ---- set_finished: no win ----

    #[test]
    fn test_set_finished_no_win() {
        let mut game = Game {
            x_board: 0b000001, // only a1
            o_board: 0b000010, // only a2
            order_turn: true,
            finished: false,
        };
        game.set_finished();
        assert!(!game.finished);
    }

    // ---- set_finished: column wins ----

    #[test]
    fn test_set_finished_column_win_low() {
        // X fills rows 1–5 of column a: bits 0–4 = 0b011111.
        let mut game = Game {
            x_board: 0b011111,
            o_board: 0,
            order_turn: true,
            finished: false,
        };
        game.set_finished();
        assert!(game.finished);
    }

    #[test]
    fn test_set_finished_column_win_high() {
        // O fills rows 2–6 of column a: bits 1–5 = 0b111110.
        let mut game = Game {
            x_board: 0,
            o_board: 0b111110,
            order_turn: true,
            finished: false,
        };
        game.set_finished();
        assert!(game.finished);
    }

    #[test]
    fn test_set_finished_six_in_column_no_win() {
        // Six-in-a-column must NOT count as a win for Order.
        let mut game = Game {
            x_board: 0b111111, // all 6 rows of column a
            o_board: 0,
            order_turn: true,
            finished: false,
        };
        game.set_finished();
        assert!(!game.finished);
    }

    // ---- set_finished: row wins ----

    #[test]
    fn test_set_finished_row_win_low() {
        // X fills columns a–e of row 1: bits 0, 6, 12, 18, 24 (a1, b1, c1, d1, e1).
        let mut game = Game {
            x_board: (1_u64 << 0) | (1_u64 << 6) | (1_u64 << 12) | (1_u64 << 18) | (1_u64 << 24),
            o_board: 0,
            order_turn: true,
            finished: false,
        };
        game.set_finished();
        assert!(game.finished);
    }

    #[test]
    fn test_set_finished_row_win_high() {
        // O fills columns b–f of row 1: bits 6, 12, 18, 24, 30 (b1, c1, d1, e1, f1).
        let mut game = Game {
            x_board: 0,
            o_board: (1_u64 << 6) | (1_u64 << 12) | (1_u64 << 18) | (1_u64 << 24) | (1_u64 << 30),
            order_turn: true,
            finished: false,
        };
        game.set_finished();
        assert!(game.finished);
    }

    #[test]
    fn test_set_finished_row_win_middle_row() {
        // X fills columns a–e of row 3: bits 2, 8, 14, 20, 26 (a3, b3, c3, d3, e3).
        let mut game = Game {
            x_board: (1_u64 << 2) | (1_u64 << 8) | (1_u64 << 14) | (1_u64 << 20) | (1_u64 << 26),
            o_board: 0,
            order_turn: true,
            finished: false,
        };
        game.set_finished();
        assert!(game.finished);
    }

    // ---- set_finished: diagonal wins ----

    #[test]
    fn test_set_finished_diagonal_win_main_high() {
        // X fills the high end of the main diagonal: b2, c3, d4, e5, f6 (bits 7,14,21,28,35).
        let mut game = Game {
            x_board: (1_u64 << 7) | (1_u64 << 14) | (1_u64 << 21) | (1_u64 << 28) | (1_u64 << 35),
            o_board: 0,
            order_turn: true,
            finished: false,
        };
        game.set_finished();
        assert!(game.finished);
    }

    #[test]
    fn test_set_finished_diagonal_win_main_low() {
        // X fills the low end of the main diagonal: a1, b2, c3, d4, e5 (bits 0,7,14,21,28).
        let mut game = Game {
            x_board: (1_u64 << 0) | (1_u64 << 7) | (1_u64 << 14) | (1_u64 << 21) | (1_u64 << 28),
            o_board: 0,
            order_turn: true,
            finished: false,
        };
        game.set_finished();
        assert!(game.finished);
    }

    #[test]
    fn test_set_finished_diagonal_win_lower_off() {
        // X fills the lower off-diagonal: a2, b3, c4, d5, e6 (bits 1,8,15,22,29).
        let mut game = Game {
            x_board: (1_u64 << 1) | (1_u64 << 8) | (1_u64 << 15) | (1_u64 << 22) | (1_u64 << 29),
            o_board: 0,
            order_turn: true,
            finished: false,
        };
        game.set_finished();
        assert!(game.finished);
    }

    #[test]
    fn test_set_finished_diagonal_win_upper_off() {
        // O fills the upper off-diagonal: b1, c2, d3, e4, f5 (bits 6,13,20,27,34).
        let mut game = Game {
            x_board: 0,
            o_board: (1_u64 << 6) | (1_u64 << 13) | (1_u64 << 20) | (1_u64 << 27) | (1_u64 << 34),
            order_turn: true,
            finished: false,
        };
        game.set_finished();
        assert!(game.finished);
    }

    // ---- set_finished: Chaos win ----

    #[test]
    fn test_set_finished_chaos_win() {
        // Fill every square without any five-in-a-row: alternate X/O by column.
        // Columns a, c, e → X (bits 0–5, 12–17, 24–29).
        // Columns b, d, f → O (bits 6–11, 18–23, 30–35).
        let mut game = Game {
            x_board: 0x3F | (0x3F << 12) | (0x3F_u64 << 24),
            o_board: (0x3F_u64 << 6) | (0x3F_u64 << 18) | (0x3F_u64 << 30),
            order_turn: true,
            finished: false,
        };
        game.set_finished();
        assert!(game.finished);
    }
}
