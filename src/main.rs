use std::io;
use std::sync::{Arc, mpsc};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};

mod ai;
mod game;
use game::Game;

use crate::ai::Ai;

const MAX_DEPTH: u8 = 8;
const AI_TIME_LIMIT: u64 = 2000;

// ─── Screen ──────────────────────────────────────────────────────────────────

enum Screen {
    ModeSelect { cursor: usize },
    RoleSelect { cursor: usize },
    Playing(App),
}

// ─── AppEvent ────────────────────────────────────────────────────────────────

/// All external event sources. Extend this enum to add new event types
/// (timers, network moves, etc.) without touching handle_event's callers.
enum AppEvent {
    Key(crossterm::event::KeyEvent),
    AiResult(Result<String, String>),
}

// ─── Message ─────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
enum Message {
    CursorUp,
    CursorDown,
    /// Enter in menus; also used to submit a move while Playing
    Confirm,
    /// Esc — go back / quit depending on current screen
    Back,
    /// Ctrl-C — hard quit from any screen
    Quit,
    InputChar(char),
    InputBackspace,
    AiMoved(String),
    AiError(String),
}

// ─── Command ──────────────────────────────────────────────────────────────────

/// Side effects that update() wants the main loop to perform.
enum Command {
    SpawnAi { ai: Arc<Ai>, game: Game },
}

// ─── Model ───────────────────────────────────────────────────────────────────

struct Model {
    screen: Screen,
    exit: bool,
}

impl Model {
    fn new() -> Self {
        Model {
            screen: Screen::ModeSelect { cursor: 0 },
            exit: false,
        }
    }
}

// ─── handle_event (pure) ─────────────────────────────────────────────────────

fn handle_event(event: AppEvent) -> Option<Message> {
    match event {
        AppEvent::Key(key) => match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Message::Quit)
            }
            KeyCode::Esc => Some(Message::Back),
            KeyCode::Up => Some(Message::CursorUp),
            KeyCode::Down => Some(Message::CursorDown),
            KeyCode::Enter => Some(Message::Confirm),
            KeyCode::Backspace => Some(Message::InputBackspace),
            KeyCode::Char(c) => Some(Message::InputChar(c)),
            _ => None,
        },
        AppEvent::AiResult(Ok(mv)) => Some(Message::AiMoved(mv)),
        AppEvent::AiResult(Err(e)) => Some(Message::AiError(e)),
    }
}

// ─── update (all state mutation) ─────────────────────────────────────────────

fn update(model: &mut Model, msg: Message) -> Option<Command> {
    if msg == Message::Quit {
        model.exit = true;
        return None;
    }

    match &mut model.screen {
        Screen::ModeSelect { cursor } => match msg {
            Message::CursorUp => *cursor = cursor.saturating_sub(1),
            Message::CursorDown => *cursor = (*cursor + 1).min(1),
            Message::Confirm => {
                let selected = *cursor; // copy to end borrow before reassigning screen
                model.screen = if selected == 0 {
                    Screen::Playing(App::new(SetupOptions {
                        has_ai: false,
                        player_order: None,
                    }))
                } else {
                    Screen::RoleSelect { cursor: 0 }
                };
            }
            Message::Back => model.exit = true,
            _ => {}
        },
        Screen::RoleSelect { cursor } => match msg {
            Message::CursorUp => *cursor = cursor.saturating_sub(1),
            Message::CursorDown => *cursor = (*cursor + 1).min(1),
            Message::Confirm => {
                let is_order = *cursor == 0;
                let app = App::new(SetupOptions {
                    has_ai: true,
                    player_order: Some(is_order),
                });
                // If AI goes first (player chose Chaos), spawn immediately
                let cmd = if !app.is_player_turn() {
                    app.ai.as_ref().map(|ai| Command::SpawnAi {
                        ai: Arc::clone(ai),
                        game: app.game.clone(),
                    })
                } else {
                    None
                };
                model.screen = Screen::Playing(app);
                return cmd;
            }
            Message::Back => model.screen = Screen::ModeSelect { cursor: 0 },
            _ => {}
        },
        Screen::Playing(app) => match msg {
            Message::Back => {
                model.exit = true;
                return None;
            }
            _ if app.game.is_finished() => {}
            Message::Confirm if app.is_player_turn() => {
                if app.submit_move() 
                    && let Some(ai) = &app.ai
                    && !app.game.is_finished()
                {
                        return Some(Command::SpawnAi {
                            ai: Arc::clone(ai),
                            game: app.game.clone(),
                        });
                    
                }
            }
            Message::InputChar(c) => {
                app.input.push(c);
                app.message = None;
            }
            Message::InputBackspace => {
                app.input.pop();
                app.message = None;
            }
            Message::AiMoved(mv) => {
                if let Err(e) = app.game.process_move(&mv) {
                    app.message = Some(e.to_string());
                } else {
                    app.message = Some(format!("AI's move was {}", mv));
                }
            }
            Message::AiError(e) => {
                app.message = Some(e);
            }
            _ => {}
        },
    }
    None
}

// ─── view (pure render) ───────────────────────────────────────────────────────

fn view(model: &Model, frame: &mut Frame) {
    match &model.screen {
        Screen::ModeSelect { cursor } => {
            draw_menu(frame, " Game Mode ", &["Two Players", "vs Ai"], *cursor)
        }
        Screen::RoleSelect { cursor } => {
            draw_menu(frame, " Play as ", &["Order", "Chaos"], *cursor)
        }
        Screen::Playing(app) => app.draw(frame),
    }
}

fn draw_menu(frame: &mut Frame, title: &str, options: &[&str], cursor: usize) {
    let items: Vec<Line> = options
        .iter()
        .enumerate()
        .map(|(i, label)| {
            if i == cursor {
                Line::from(Span::styled(
                    format!("▶ {}", label),
                    Style::default().fg(Color::Yellow),
                ))
            } else {
                Line::from(format!("  {}", label))
            }
        })
        .collect();

    let area = frame.area();
    frame.render_widget(
        Paragraph::new(items).block(Block::default().title(title).borders(Borders::ALL)),
        area,
    );
}

// ─── execute_command ──────────────────────────────────────────────────────────

fn execute_command(cmd: Command, tx: &mpsc::Sender<Result<String, String>>) {
    match cmd {
        Command::SpawnAi { ai, game } => {
            let tx = tx.clone();
            std::thread::spawn(move || {
                let mut game = game;
                let result = ai.get_move(&mut game, AI_TIME_LIMIT).map_err(|e| e.to_string());
                tx.send(result).unwrap();
            });
        }
    }
}

// ─── App ─────────────────────────────────────────────────────────────────────

struct App {
    game: Game,
    input: String,
    message: Option<String>,
    ai: Option<Arc<Ai>>,
}

impl App {
    fn new(setup: SetupOptions) -> Self {
        let game_ai = if setup.has_ai {
            if setup.player_order.unwrap() {
                Some(Arc::new(Ai::new(ai::AiRole::Chaos, MAX_DEPTH)))
            } else {
                Some(Arc::new(Ai::new(ai::AiRole::Order, MAX_DEPTH)))
            }
        } else {
            None
        };
        App {
            game: Game::new(),
            input: String::new(),
            message: None,
            ai: game_ai,
        }
    }

    fn is_player_turn(&self) -> bool {
        match &self.ai {
            None => true,
            Some(ai) => self.game.is_order_turn() != ai.is_order(),
        }
    }

    // Returns whether the move was successfully processed
    fn submit_move(&mut self) -> bool {
        let move_str = std::mem::take(&mut self.input);
        if let Err(e) = self.game.process_move(&move_str) {
            self.message = Some(e.to_string());
            false
        } else {
            true
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(9), // header row + 6 board rows + 2 border
                Constraint::Length(3), // status
                Constraint::Length(3), // move input
                Constraint::Length(3), // help / error
            ])
            .split(frame.area());

        frame.render_widget(
            Paragraph::new(self.render_board()).block(
                Block::default()
                    .title(" Order and Chaos ")
                    .borders(Borders::ALL),
            ),
            chunks[0],
        );

        let status = if self.game.is_finished() {
            if self.game.is_board_full() {
                "Chaos wins! The board is full."
            } else {
                "Order wins! Five in a row!"
            }
        } else if self.game.is_order_turn() {
            "Order's turn"
        } else {
            "Chaos's turn"
        };
        frame.render_widget(
            Paragraph::new(status).block(Block::default().title(" Status ").borders(Borders::ALL)),
            chunks[1],
        );

        frame.render_widget(
            Paragraph::new(format!("Move: {}_", self.input))
                .block(Block::default().title(" Input ").borders(Borders::ALL)),
            chunks[2],
        );

        let help: Line<'static> = if let Some(ref msg) = self.message {
            Line::from(Span::styled(msg.clone(), Style::default().fg(Color::Red)))
        } else if self.game.is_finished() {
            Line::from("[Esc] quit")
        } else if !self.is_player_turn() {
            Line::from("Thinking...")
        } else {
            Line::from("Format: <col><row><piece>  e.g. a1x    [Esc] quit")
        };
        frame.render_widget(
            Paragraph::new(help).block(Block::default().borders(Borders::ALL)),
            chunks[3],
        );
    }

    fn render_board(&self) -> Text<'static> {
        let mut lines: Vec<Line<'static>> = vec![Line::from("   a  b  c  d  e  f")];

        for row in 0..6u8 {
            let mut spans: Vec<Span<'static>> = vec![Span::raw(format!("{}  ", row + 1))];
            for col in 0..6u8 {
                let cell = match self.game.piece_at(col, row) {
                    Some('X') => Span::styled("X", Style::default().fg(Color::Blue)),
                    Some('O') => Span::styled("O", Style::default().fg(Color::Red)),
                    _ => Span::raw("·"),
                };
                spans.push(cell);
                if col < 5 {
                    spans.push(Span::raw("  "));
                }
            }
            lines.push(Line::from(spans));
        }

        Text::from(lines)
    }
}

// ─── SetupOptions ─────────────────────────────────────────────────────────────

struct SetupOptions {
    has_ai: bool,
    player_order: Option<bool>,
}

// ─── main ─────────────────────────────────────────────────────────────────────

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut model = Model::new();
    let (ai_tx, ai_rx) = mpsc::channel::<Result<String, String>>();

    let result = (|| {
        while !model.exit {
            terminal.draw(|f| view(&model, f))?;

            // Non-blocking check for an AI result
            if let Ok(result) = ai_rx.try_recv() {
                if let Some(msg) = handle_event(AppEvent::AiResult(result))
                    && let Some(cmd) = update(&mut model, msg)
                {
                    execute_command(cmd, &ai_tx);
                }
                continue;
            }

            // Wait up to 16ms for a key event so AI results are picked up promptly
            if event::poll(Duration::from_millis(16))?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
                && let Some(msg) = handle_event(AppEvent::Key(key))
                && let Some(cmd) = update(&mut model, msg)
            {
                execute_command(cmd, &ai_tx);
            }
        }
        Ok(())
    })();
    ratatui::restore();
    result
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> AppEvent {
        AppEvent::Key(KeyEvent::new(code, KeyModifiers::empty()))
    }

    fn mode_select() -> Model {
        Model::new()
    }

    fn role_select() -> Model {
        Model {
            screen: Screen::RoleSelect { cursor: 0 },
            exit: false,
        }
    }

    fn playing(has_ai: bool) -> Model {
        Model {
            screen: Screen::Playing(App::new(SetupOptions {
                has_ai,
                player_order: if has_ai { Some(true) } else { None },
            })),
            exit: false,
        }
    }

    // ── handle_event ──────────────────────────────────────────────────────────

    #[test]
    fn handle_event_esc_back() {
        assert_eq!(handle_event(key(KeyCode::Esc)), Some(Message::Back));
    }

    #[test]
    fn handle_event_ctrl_c_quit() {
        let ev = AppEvent::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(handle_event(ev), Some(Message::Quit));
    }

    #[test]
    fn handle_event_enter_confirm() {
        assert_eq!(handle_event(key(KeyCode::Enter)), Some(Message::Confirm));
    }

    #[test]
    fn handle_event_char() {
        assert_eq!(
            handle_event(key(KeyCode::Char('a'))),
            Some(Message::InputChar('a'))
        );
    }

    #[test]
    fn handle_event_unknown_none() {
        assert_eq!(handle_event(key(KeyCode::F(5))), None);
    }

    #[test]
    fn handle_event_ai_result_ok() {
        assert_eq!(
            handle_event(AppEvent::AiResult(Ok("a1x".into()))),
            Some(Message::AiMoved("a1x".into()))
        );
    }

    #[test]
    fn handle_event_ai_result_err() {
        assert_eq!(
            handle_event(AppEvent::AiResult(Err("oops".into()))),
            Some(Message::AiError("oops".into()))
        );
    }

    // ── update / ModeSelect ───────────────────────────────────────────────────

    #[test]
    fn mode_select_cursor_down() {
        let mut m = mode_select();
        update(&mut m, Message::CursorDown);
        assert!(matches!(m.screen, Screen::ModeSelect { cursor: 1 }));
    }

    #[test]
    fn mode_select_cursor_up_clamps() {
        let mut m = mode_select();
        update(&mut m, Message::CursorUp);
        assert!(matches!(m.screen, Screen::ModeSelect { cursor: 0 }));
    }

    #[test]
    fn mode_select_confirm_0_playing() {
        let mut m = mode_select();
        update(&mut m, Message::Confirm);
        assert!(matches!(m.screen, Screen::Playing(_)));
    }

    #[test]
    fn mode_select_confirm_1_role_select() {
        let mut m = mode_select();
        update(&mut m, Message::CursorDown);
        update(&mut m, Message::Confirm);
        assert!(matches!(m.screen, Screen::RoleSelect { cursor: 0 }));
    }

    #[test]
    fn mode_select_back_exits() {
        let mut m = mode_select();
        update(&mut m, Message::Back);
        assert!(m.exit);
    }

    // ── update / RoleSelect ───────────────────────────────────────────────────

    #[test]
    fn role_select_confirm_playing() {
        let mut m = role_select();
        update(&mut m, Message::Confirm);
        assert!(matches!(m.screen, Screen::Playing(_)));
    }

    #[test]
    fn role_select_back_mode_select() {
        let mut m = role_select();
        update(&mut m, Message::Back);
        assert!(matches!(m.screen, Screen::ModeSelect { cursor: 0 }));
    }

    // ── update / Playing ──────────────────────────────────────────────────────

    #[test]
    fn playing_back_exits() {
        let mut m = playing(false);
        update(&mut m, Message::Back);
        assert!(m.exit);
    }

    #[test]
    fn playing_quit_exits() {
        let mut m = playing(false);
        update(&mut m, Message::Quit);
        assert!(m.exit);
    }

    #[test]
    fn playing_input_char_appends() {
        let mut m = playing(false);
        update(&mut m, Message::InputChar('a'));
        update(&mut m, Message::InputChar('1'));
        let Screen::Playing(ref app) = m.screen else { panic!() };
        assert_eq!(app.input, "a1");
    }

    #[test]
    fn playing_backspace_pops() {
        let mut m = playing(false);
        update(&mut m, Message::InputChar('a'));
        update(&mut m, Message::InputChar('1'));
        update(&mut m, Message::InputBackspace);
        let Screen::Playing(ref app) = m.screen else { panic!() };
        assert_eq!(app.input, "a");
    }

    #[test]
    fn playing_confirm_valid_move_advances_game() {
        let mut m = playing(false);
        for c in "a1x".chars() {
            update(&mut m, Message::InputChar(c));
        }
        update(&mut m, Message::Confirm);
        let Screen::Playing(ref app) = m.screen else { panic!() };
        assert_eq!(app.game.piece_at(0, 0), Some('X'));
        assert_eq!(app.input, "");
    }

    #[test]
    fn playing_confirm_invalid_move_sets_message() {
        let mut m = playing(false);
        for c in "zzz".chars() {
            update(&mut m, Message::InputChar(c));
        }
        update(&mut m, Message::Confirm);
        let Screen::Playing(ref app) = m.screen else { panic!() };
        assert!(app.message.is_some());
        assert!(app.is_player_turn());
    }

    #[test]
    fn playing_input_ignored_after_game_over() {
        let mut m = playing(false);
        let Screen::Playing(ref mut app) = m.screen else { panic!() };
        // Build an Order win: X in column a rows 1-5
        for r in ['1', '2', '3', '4', '5'] {
            app.game.process_move(&format!("a{}x", r)).unwrap();
            if !app.game.is_finished() {
                app.game.process_move(&format!("b{}o", r)).unwrap();
            }
        }
        assert!(app.game.is_finished());
        // Now InputChar should be ignored
        update(&mut m, Message::InputChar('z'));
        let Screen::Playing(ref app) = m.screen else { panic!() };
        assert_eq!(app.input, "");
    }

    #[test]
    fn playing_ai_moved_applies_move() {
        let mut m = playing(false);
        update(&mut m, Message::AiMoved("a1x".into()));
        let Screen::Playing(ref app) = m.screen else { panic!() };
        assert_eq!(app.game.piece_at(0, 0), Some('X'));
        assert!(app.message.as_deref().unwrap_or("").contains("a1x"));
    }

    #[test]
    fn playing_ai_error_sets_message() {
        let mut m = playing(false);
        update(&mut m, Message::AiError("no moves".into()));
        let Screen::Playing(ref app) = m.screen else { panic!() };
        assert_eq!(app.message.as_deref(), Some("no moves"));
    }
}
