// This file contains the game engine itself, including representation of the game state and
// utility functions like determining a winner.

use std::ops::BitAndAssign;

const BOARD_SIDE: u8 = 6;

const UPPER_RANGE: (u32, u32) = ('A' as u32, 'Z' as u32);
const LOWER_RANGE: (u32, u32) = ('a' as u32, 'z' as u32);

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
struct Game {
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
fn parse_move_string(move_string: String) -> Result<(Space, u8, u8), &'static str> {
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
    pub fn process_move(&mut self, move_string: String) -> Result<bool, &str> {
        if self.finished {
            return Err("Game is already finished");
        }

        let (piece, col, row) = parse_move_string(move_string)?;
        let offset: u64 = 1 << (col * BOARD_SIDE + row);

        if self.x_board & offset > 0 || self.o_board & offset > 0 {
            return Err("Space is not free");
        }

        match piece {
            Space::O => self.o_board.bitand_assign(offset),
            Space::X => self.x_board.bitand_assign(offset),
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
        // Check rows

    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_move_string_good() {
        // Lowercase column, X piece
        assert_eq!(
            parse_move_string("a1x".to_string()),
            Ok((Space::X, 0, 0))
        );
        // Lowercase column, O piece
        assert_eq!(
            parse_move_string("b3o".to_string()),
            Ok((Space::O, 1, 2))
        );
        // Uppercase column, uppercase piece
        assert_eq!(
            parse_move_string("A1X".to_string()),
            Ok((Space::X, 0, 0))
        );
        // Last valid column for a 6x6 board (f = index 5)
        assert_eq!(
            parse_move_string("f6o".to_string()),
            Ok((Space::O, 5, 5))
        );
    }

    #[test]
    fn test_parse_move_string_bad() {
        // Too short
        assert!(parse_move_string("a1".to_string()).is_err());
        // Too long
        assert!(parse_move_string("a1xx".to_string()).is_err());
        // Empty string
        assert!(parse_move_string("".to_string()).is_err());
        // Invalid piece character
        assert!(parse_move_string("a1z".to_string()).is_err());
        // Non-letter column
        assert!(parse_move_string("11x".to_string()).is_err());
        // Row out of range for a 6x6 board
        assert!(parse_move_string("a7x".to_string()).is_err());
        // Row out of range at 0
        assert!(parse_move_string("a0x".to_string()).is_err());
    }
}
