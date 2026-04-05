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

// ---- mirror_board ----

#[test]
fn test_mirror_board_col_a_to_f() {
    // Column a (bits 0–5) should map to column f (bits 30–35).
    assert_eq!(mirror_board(0b111111), 0b111111_u64 << 30);
}

#[test]
fn test_mirror_board_col_f_to_a() {
    assert_eq!(mirror_board(0b111111_u64 << 30), 0b111111);
}

#[test]
fn test_mirror_board_roundtrip() {
    // Mirroring twice should return the original board.
    let board: u64 = (1 << 7) | (1 << 14) | (1u64 << 28);
    assert_eq!(mirror_board(mirror_board(board)), board);
}

// ---- has_lr_diagonal_win ----

#[test]
fn test_has_lr_diagonal_win_main_low() {
    let board = (1_u64 << 0) | (1_u64 << 7) | (1_u64 << 14) | (1_u64 << 21) | (1_u64 << 28);
    assert!(has_lr_diagonal_win(board));
}

#[test]
fn test_has_lr_diagonal_win_lower_off() {
    let board = (1_u64 << 1) | (1_u64 << 8) | (1_u64 << 15) | (1_u64 << 22) | (1_u64 << 29);
    assert!(has_lr_diagonal_win(board));
}

#[test]
fn test_has_lr_diagonal_win_false() {
    assert!(!has_lr_diagonal_win(0b111111)); // full column, not a diagonal
}

// ---- set_finished: RL diagonal wins ----

#[test]
fn test_set_finished_rl_diagonal_main_low() {
    // X fills f1→b5 (the low half of the anti-diagonal): bits 30, 25, 20, 15, 10.
    let mut game = Game {
        x_board: (1_u64 << 30) | (1_u64 << 25) | (1_u64 << 20) | (1_u64 << 15) | (1_u64 << 10),
        o_board: 0,
        order_turn: true,
        finished: false,
    };
    game.set_finished();
    assert!(game.finished);
}

#[test]
fn test_set_finished_rl_diagonal_main_high() {
    // O fills e2→a6 (the high half of the anti-diagonal): bits 25, 20, 15, 10, 5.
    let mut game = Game {
        x_board: 0,
        o_board: (1_u64 << 25) | (1_u64 << 20) | (1_u64 << 15) | (1_u64 << 10) | (1_u64 << 5),
        order_turn: true,
        finished: false,
    };
    game.set_finished();
    assert!(game.finished);
}

#[test]
fn test_set_finished_rl_diagonal_upper_off() {
    // X fills e1→a5 (upper RL off-diagonal): bits 24, 19, 14, 9, 4.
    let mut game = Game {
        x_board: (1_u64 << 24) | (1_u64 << 19) | (1_u64 << 14) | (1_u64 << 9) | (1_u64 << 4),
        o_board: 0,
        order_turn: true,
        finished: false,
    };
    game.set_finished();
    assert!(game.finished);
}

#[test]
fn test_set_finished_rl_diagonal_lower_off() {
    // O fills f2→b6 (lower RL off-diagonal): bits 31, 26, 21, 16, 11.
    let mut game = Game {
        x_board: 0,
        o_board: (1_u64 << 31) | (1_u64 << 26) | (1_u64 << 21) | (1_u64 << 16) | (1_u64 << 11),
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
