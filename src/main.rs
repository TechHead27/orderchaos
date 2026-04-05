use std::io;

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

enum Screen {
    ModeSelect { cursor: usize },
    RoleSelect { cursor: usize },
    Playing(App),
}

struct Ui {
    screen: Screen,
    exit: bool,
}

impl Ui {
    fn new() -> Self {
        Ui {
            screen: Screen::ModeSelect { cursor: 0 },
            exit: false,
        }
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        match &mut self.screen {
            Screen::ModeSelect { cursor } => match key.code {
                KeyCode::Up => *cursor = cursor.saturating_sub(1),
                KeyCode::Down => *cursor = (*cursor + 1).min(1),
                KeyCode::Enter => {
                    if *cursor == 0 {
                        self.screen = Screen::Playing(App::new(SetupOptions {
                            has_ai: false,
                            player_order: None,
                        }));
                    } else {
                        self.screen = Screen::RoleSelect { cursor: 0 };
                    }
                }
                KeyCode::Esc => self.exit = true,
                _ => {}
            },
            Screen::RoleSelect { cursor } => match key.code {
                KeyCode::Up => *cursor = cursor.saturating_sub(1),
                KeyCode::Down => *cursor = (*cursor + 1).min(1),
                KeyCode::Enter => {
                    self.screen = Screen::Playing(App::new(SetupOptions {
                        has_ai: true,
                        player_order: Some(*cursor == 0),
                    }));
                }
                KeyCode::Esc => self.screen = Screen::ModeSelect { cursor: 0 },
                _ => {}
            },
            Screen::Playing(app) => app.handle_key(key),
        }
    }

    fn draw(&self, frame: &mut Frame) {
        match &self.screen {
            Screen::ModeSelect { cursor } => {
                Self::draw_menu(frame, " Game Mode ", &["Two Players", "vs Ai"], *cursor)
            }
            Screen::RoleSelect { cursor } => {
                Self::draw_menu(frame, " Play as ", &["Order", "Chaos"], *cursor)
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
}

struct App {
    game: Game,
    input: String,
    message: Option<String>,
    ai: Option<Ai>,
    exit: bool,
}

impl App {
    fn new(setup: SetupOptions) -> Self {
        let game_ai = if setup.has_ai {
            if setup.player_order.unwrap() {
                Some(Ai::new(ai::AiRole::Chaos, MAX_DEPTH))
            } else {
                Some(Ai::new(ai::AiRole::Order, MAX_DEPTH))
            }
        } else {
            None
        };
        App {
            game: Game::new(),
            input: String::new(),
            message: None,
            ai: game_ai,
            exit: false,
        }
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.exit = true;
            }
            KeyCode::Esc => self.exit = true,
            // Ignore further input once the game is over.
            _ if self.game.is_finished() => {}
            KeyCode::Enter => self.submit_move(),
            KeyCode::Backspace => {
                self.input.pop();
                self.message = None;
            }
            KeyCode::Char(c) => {
                self.input.push(c);
                self.message = None;
            }
            _ => {}
        }
    }

    fn submit_move(&mut self) {
        let move_str = std::mem::take(&mut self.input);
        if let Err(e) = self.game.process_move(&move_str) {
            self.message = Some(e.to_string());
        }

        // Also get AI move if it's their turn
        if let Some(ai) = &self.ai
            && let Ok(mv) = ai
                .get_move(&mut self.game, AI_TIME_LIMIT)
                .and_then(|mv| self.game.process_move(&mv).map(|_b| mv))
                .map_err(|e| self.message = Some(e.to_string()))
        {
            self.message = Some(format!("AI's move was {}", mv));
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

struct SetupOptions {
    has_ai: bool,
    player_order: Option<bool>,
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut ui = Ui::new();
    let result = (|| {
        while !ui.exit {
            terminal.draw(|f| ui.draw(f))?;
            if let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                ui.handle_key(key);
            }
        }
        Ok(())
    })();
    ratatui::restore();
    result
}
