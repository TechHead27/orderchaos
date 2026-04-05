use core::f64;

use crate::game::{Game, Space};
use crate::game::constants::BOARD_SIDE;

const BOARD_SIDE_U64: u64 = BOARD_SIDE as u64;
const BOARD_SIZE: u64 = BOARD_SIDE_U64 * BOARD_SIDE_U64;
const FULL_BOARD: u64 = (1 << BOARD_SIZE) - 1;

// ─── Win masks ────────────────────────────────────────────────────────────────
// All windows of exactly 5 contiguous cells (rows, cols, diagonals).
// We exclude 6-in-a-row per the balanced ruleset: a run of 6 must NOT
// contain any sub-window-of-5 that is also a 6-window, so we mask out
// cells that are part of a length-6 run at eval time (see is_five_in_a_row).

fn generate_win_masks() -> Vec<u64> {
    let mut masks = Vec::new();

    // Rows: windows of exactly 5 within each row of 6
    for row in 0..BOARD_SIDE_U64 {
        for start_col in 0..=BOARD_SIDE_U64 - 5 {
            let mut mask: u64 = 0;
            for c in start_col..start_col + 5 {
                mask |= 1 << (c * BOARD_SIDE_U64 + row);
            }
            masks.push(mask);
        }
    }

    // Cols: windows of exactly 5 within each col of 6
    for col in 0..BOARD_SIDE_U64 {
        for start_row in 0..=BOARD_SIDE_U64 - 5 {
            let mut mask: u64 = 0;
            for r in start_row..start_row + 5 {
                mask |= 1 << (col * BOARD_SIDE_U64 + r);
            }
            masks.push(mask);
        }
    }

    // Diagonals (top-left to bottom-right)
    for start_col in 0..=BOARD_SIDE_U64 - 5 {
        for start_row in 0..=BOARD_SIDE_U64 - 5 {
            let mut mask: u64 = 0;
            for d in 0..5 {
                mask |= 1 << ((start_col + d) * BOARD_SIDE_U64 + (start_row + d));
            }
            masks.push(mask);
        }
    }

    // Anti-diagonals (top-right to bottom-left)
    for start_col in 0..=BOARD_SIDE_U64 - 5 {
        for start_row in (4..BOARD_SIDE_U64).rev() {
            let mut mask: u64 = 0;
            for d in 0..5u64 {
                let r = start_row as i64 - d as i64;
                if r < 0 { break; }
                mask |= 1 << ((start_col + d) * BOARD_SIDE_U64 + r as u64);
            }
            if mask.count_ones() == 5 {
                masks.push(mask);
            }
        }
    }

    masks
}

// Extend a 5-mask to its 6-cell enclosing window (if it exists on the board),
// used to detect and exclude 6-in-a-row "false wins".
fn six_mask_for(five_mask: u64) -> Option<u64> {
    // Try extending in each axis direction; return the 6-mask if all 6 cells
    // are on the same row/col/diagonal. This is used to disqualify a 5-window
    // that is embedded inside a 6-in-a-row.
    // For brevity we check both extensions (prepend / append one cell).
    let cells: Vec<u64> = (0..BOARD_SIZE)
        .filter(|&i| five_mask & (1 << i) != 0)
        .collect();

    let try_extend = |extra: i64| -> Option<u64> {
        if extra < 0 || extra >= BOARD_SIZE as i64 { return None; }
        let bit = 1u64 << extra;
        let six = five_mask | bit;
        // Validate it's a true straight line of 6
        // This works because if the bit is already inside the mask; the count doesn't change
        if six.count_ones() == 6 { Some(six) } else { None }
    };

    // Compute stride from first two cells
    if cells.len() < 2 { return None; }
    let stride = cells[1] as i64 - cells[0] as i64;
    let before = cells[0] as i64 - stride;
    let after  = *cells.last().unwrap() as i64 + stride;

    // A 6-mask exists if either extension lands on the board and is collinear
    try_extend(before).or_else(|| try_extend(after))
}

// ─── Role ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum AiRole {
    Order,  // MAX player — wants 5-in-a-row
    Chaos,  // MIN player — wants to fill board with no 5-in-a-row
}

// ─── Move ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
pub struct AiMove {
    pub cell: u8,    // 0..35, col * 6 + row
    pub piece: Space,
}

impl AiMove {
    /// Convert to the string format process_move expects, e.g. "a1X"
    pub fn to_move_string(&self) -> String {
        let col = self.cell as u64 / BOARD_SIDE_U64;
        let row = self.cell as u64 % BOARD_SIDE_U64;
        let col_char = (b'a' + col as u8) as char;
        let piece_char = match self.piece {
            Space::X => 'X',
            Space::O => 'O',
        };
        format!("{}{}{}", col_char, row + 1, piece_char)
    }
}

// ─── AI state (search-internal, never mutates Game) ───────────────────────────

#[derive(Clone)]
struct AiState {
    x_board: u64,
    o_board: u64,
    order_turn: bool,
}

impl AiState {
    fn from_game(game: &Game) -> Self {
        AiState {
            x_board: game.get_x_board(),
            o_board: game.get_o_board(),
            order_turn: game.is_order_turn(),
        }
    }

    fn occupied(&self) -> u64 {
        self.x_board | self.o_board
    }

    fn empty_cells(&self) -> impl Iterator<Item = u8> {
        let empty = !self.occupied() & FULL_BOARD;
        (0u8..BOARD_SIZE as u8).filter(move |&i| empty & (1 << i) != 0)
    }

    fn apply(&self, mv: AiMove) -> AiState {
        let bit = 1u64 << mv.cell;
        let (x, o) = match mv.piece {
            Space::X => (self.x_board | bit, self.o_board),
            Space::O => (self.x_board, self.o_board | bit),
        };
        AiState { x_board: x, o_board: o, order_turn: !self.order_turn }
    }

    fn is_full(&self) -> bool {
        self.occupied() & FULL_BOARD == FULL_BOARD
    }
}

// ─── Win detection (respects 6-in-a-row = no win rule) ───────────────────────

fn board_has_five(board: u64, masks: &[u64], six_masks: &[Option<u64>]) -> bool {
    for (mask, six_opt) in masks.iter().zip(six_masks.iter()) {
        if board & mask == *mask {
            // All 5 cells match — now check if this window is part of a 6-in-a-row
            if let Some(six) = six_opt {
                if board & six == *six {
                    continue; // 6-in-a-row: doesn't count as a win
                }
            }
            return true;
        }
    }
    false
}

fn order_wins(state: &AiState, masks: &[u64], six_masks: &[Option<u64>]) -> bool {
    board_has_five(state.x_board, masks, six_masks)
        || board_has_five(state.o_board, masks, six_masks)
}

// ─── Evaluation ───────────────────────────────────────────────────────────────

/// Score windows of 5 for a single piece board.
/// Returns a value > 0 benefiting Order.
fn score_board(piece_board: u64, other_board: u64, masks: &[u64]) -> f64 {
    let mut score = 0.0f64;
    for &mask in masks {
        if piece_board & other_board & mask != 0 {
            continue; // window is dead (mixed pieces)
        }
        let filled = (piece_board & mask).count_ones();
        score += match filled {
            0 => 0.05,
            1 => 1.0,
            2 => 5.0,
            3 => 20.0,
            4 => 100.0,
            _ => 0.0,
        };
    }
    score
}

// Returns evaluation of given game state. Order is always positive.
fn evaluate(
    state: &AiState,
    masks: &[u64],
    six_masks: &[Option<u64>],
) -> f64 {
    // Terminal: Order wins
    if order_wins(state, masks, six_masks) {
        let score = 10_000.0 - state.occupied().count_ones() as f64;
        return score;
    }

    // Terminal: board full, Chaos wins
    if state.is_full() {
        let score = 10_000.0 - state.occupied().count_ones() as f64;
        return -score;
    }

    // Heuristic: count live window threats for each piece type
    let x_score = score_board(state.x_board, state.o_board, masks);
    let o_score = score_board(state.o_board, state.x_board, masks);
    let threat_score = x_score + o_score; // both piece types help Order

    // Dead windows help Chaos
    let dead_windows = masks.iter().filter(|&&mask| {
        state.x_board & mask != 0 && state.o_board & mask != 0
    }).count() as f64;

    let raw = threat_score - dead_windows * 2.0;
    raw
}

// ─── Move ordering ────────────────────────────────────────────────────────────

fn order_moves(
    state: &AiState,
    masks: &[u64],
    six_masks: &[Option<u64>]
) -> Vec<AiMove> {
    let mut moves: Vec<(f64, AiMove)> = state
        .empty_cells()
        .flat_map(|cell| {
            [Space::X, Space::O].into_iter().map(move |piece| AiMove { cell, piece })
        })
        .map(|mv| {
            let child = state.apply(mv);
            // Immediate wins get highest priority
            let priority = if order_wins(&child, masks, six_masks) {
                if state.order_turn { 1_000_000.0 } else { -1_000_000.0 }
            } else {
                let e = evaluate(&child, masks, six_masks);
                if state.order_turn { e } else { -e }
            };
            (priority, mv)
        })
        .collect();

    // Sort descending — best moves first for the current player
    moves.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    moves.into_iter().map(|(_, mv)| mv).collect()
}

// ─── Alpha-beta ───────────────────────────────────────────────────────────────

fn alphabeta(
    state: &AiState,
    depth: u8,
    mut alpha: f64,
    mut beta: f64,
    masks: &[u64],
    six_masks: &[Option<u64>],
) -> f64 {
    // Terminal or depth limit
    if depth == 0 || order_wins(state, masks, six_masks) || state.is_full() {
        return evaluate(state, masks, six_masks);
    }

    let moves = order_moves(state, masks, six_masks);
    let maximizing = state.order_turn;

    if maximizing {
        let mut value = f64::NEG_INFINITY;
        for mv in moves {
            let child = state.apply(mv);
            value = value.max(alphabeta(
                &child, depth - 1, alpha, beta, masks, six_masks
            ));
            alpha = alpha.max(value);
            if alpha >= beta {
                break; // β-cutoff
            }
        }
        value
    } else {
        let mut value = f64::INFINITY;
        for mv in moves {
            let child = state.apply(mv);
            value = value.min(alphabeta(
                &child, depth - 1, alpha, beta, masks, six_masks
            ));
            beta = beta.min(value);
            if alpha >= beta {
                break; // α-cutoff
            }
        }
        value
    }
}

// ─── Public AI entry point ────────────────────────────────────────────────────

pub struct Ai {
    role: AiRole,
    masks: Vec<u64>,
    six_masks: Vec<Option<u64>>,
    max_depth: u8,
}

impl Ai {
    pub fn new(role: AiRole, max_depth: u8) -> Self {
        let masks = generate_win_masks();
        let six_masks = masks.iter().map(|&m| six_mask_for(m)).collect();
        Ai { role, masks, six_masks, max_depth }
    }

    /// Iterative deepening: searches deeper until time budget is exhausted.
    /// Returns the best move found at the deepest completed iteration.
    pub fn best_move(&self, game: &Game, time_limit_ms: u64) -> Option<AiMove> {
        use std::time::Instant;

        let state = AiState::from_game(game);
        let start = Instant::now();
        let mut best: Option<AiMove> = None;

        for depth in 1..=self.max_depth {
            if start.elapsed().as_millis() as u64 >= time_limit_ms {
                return best;
            }

            // Order is MAX, Chaos is MIN
            let maximizing = self.role == AiRole::Order;
            let mut best_score = if maximizing { f64::NEG_INFINITY } else { f64::INFINITY };
            let mut alpha = f64::NEG_INFINITY;
            let mut beta = f64::INFINITY;
            let mut current_best: Option<AiMove> = None;

            for mv in order_moves(&state, &self.masks, &self.six_masks) {
                if start.elapsed().as_millis() as u64 >= time_limit_ms {
                    return best;
                }
                let child = state.apply(mv);
                let score = alphabeta(
                    &child,
                    depth - 1,
                    alpha,
                    beta,
                    &self.masks,
                    &self.six_masks,
                );

                let better = if maximizing { score > best_score } else { score < best_score };
                if better {
                    best_score = score;
                    current_best = Some(mv);
                }
                if score > alpha && maximizing {
                    alpha = score;
                }

                if score < beta && !maximizing {
                    beta = score;
                }
            }

            if let Some(mv) = current_best {
                best = Some(mv); // only update best on a fully completed iteration
            }
        }

        best
    }

    /// Returns best move as a string.
    pub fn get_move(&self, game: &mut Game, time_limit_ms: u64) -> Result<String, &str> {
        self.best_move(game, time_limit_ms).ok_or("No moves available").map(|mv| mv.to_move_string()) 
    }
}