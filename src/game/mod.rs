// This module contains the game engine itself, including representation of the game state and
// utility functions like determining a winner.

pub mod constants;

use constants::*;

/// Flips the board horizontally (column a ↔ f, b ↔ e, c ↔ d).
///
/// Because the board is stored column-major, mirroring columns converts every
/// right-to-left diagonal into a left-to-right diagonal, allowing RL diagonal
/// win detection to reuse the LR diagonal logic.
fn mirror_board(board: u64) -> u64 {
    let s = BOARD_SIDE as u64;
    (board & COL_MASK) << (s * 5)
        | ((board >> s) & COL_MASK) << (s * 4)
        | ((board >> (s * 2)) & COL_MASK) << (s * 3)
        | ((board >> (s * 3)) & COL_MASK) << (s * 2)
        | ((board >> (s * 4)) & COL_MASK) << s
        | ((board >> (s * 5)) & COL_MASK)
}

/// Returns `true` if `board` contains a winning left-to-right diagonal sequence.
fn has_lr_diagonal_win(board: u64) -> bool {
    (board & LOWER_LR_DIAG_MASK).count_ones() == 5
        || (board & UPPER_LR_DIAG_MASK).count_ones() == 5
        || board & MAIN_LR_DIAG_MASK == WINNING_LR_DIAG_HIGH
        || board & MAIN_LR_DIAG_MASK == WINNING_LR_DIAG_LOW
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Space {
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

#[derive(Debug, PartialEq, Eq, Clone)]
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

        // Check diagonals (LR and RL).
        // Mirroring the board horizontally converts RL diagonals into LR diagonals,
        // so has_lr_diagonal_win can be reused for both directions.
        let x_mirror = mirror_board(self.x_board);
        let o_mirror = mirror_board(self.o_board);
        if has_lr_diagonal_win(self.x_board)
            || has_lr_diagonal_win(self.o_board)
            || has_lr_diagonal_win(x_mirror)
            || has_lr_diagonal_win(o_mirror)
        {
            self.finished = true;
            return;
        }

        // Check win for chaos
        if self.x_board.count_ones() + self.o_board.count_ones() == (BOARD_SIDE * BOARD_SIDE) as u32
        {
            self.finished = true;
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

    pub fn get_x_board(&self) -> u64 {
        self.x_board
    }

    pub fn get_o_board(&self) -> u64 {
        self.o_board
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
mod test;
