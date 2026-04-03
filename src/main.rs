use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    DefaultTerminal, Frame,
};

mod game;
use game::Game;

struct App {
    game: Game,
    input: String,
    message: Option<String>,
    exit: bool,
}

impl App {
    fn new() -> Self {
        App {
            game: Game::new(),
            input: String::new(),
            message: None,
            exit: false,
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            if let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press {
                    self.handle_key(key);
                }
        }
        Ok(())
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

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new();
    let result = app.run(&mut terminal);
    ratatui::restore();
    result
}
