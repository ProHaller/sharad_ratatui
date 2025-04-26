// /ui/save_name_input.rs
use crate::{
    app::{Action, App, InputMode},
    context::Context,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Position, Rect},
    prelude::Alignment,
    style::{Color, Style},
    widgets::*,
};
use tui_input::{Input, backend::crossterm::EventHandler};

use super::{Component, ComponentEnum, main_menu::MainMenu};

#[derive(Default, Debug)]
pub struct SaveName {
    input: Input,
}

impl Component for SaveName {
    fn on_key(&mut self, key: KeyEvent, context: Context) -> Option<Action> {
        match context.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Char('e') => Some(Action::SwitchInputMode(InputMode::Editing)),
                KeyCode::Char('r') => Some(Action::SwitchInputMode(InputMode::Recording)),
                KeyCode::Esc => Some(Action::SwitchComponent(ComponentEnum::from(
                    MainMenu::default(),
                ))),
                KeyCode::Enter => {
                    if !self.input.value().is_empty() {
                        Some(Action::CreateNewGame(self.input.value().into()))
                    } else {
                        Some(Action::SwitchInputMode(InputMode::Editing))
                    }
                }
                _ => None,
            },
            InputMode::Editing => match key.code {
                KeyCode::Esc => Some(Action::SwitchInputMode(InputMode::Normal)),
                KeyCode::Char('v') => {
                    todo!("Centralize the text input handling for paste.")
                }
                KeyCode::Enter => Some(Action::SwitchInputMode(InputMode::Normal)),
                _ => {
                    self.input.handle_event(&crossterm::event::Event::Key(key));
                    None
                }
            },
            InputMode::Recording if key.code == KeyCode::Esc => {
                // TODO: Stop recording if not in InputMode::Recording
                todo!("Need to implement the voice recording");
            }
            _ => None,
        }
    }
    fn render(&mut self, area: Rect, buffer: &mut Buffer, context: &Context) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .flex(ratatui::layout::Flex::Center)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(1),
                ]
                .as_ref(),
            )
            .split(area);

        let title = Paragraph::new(" Enter Save Name ")
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center);
        title.render(chunks[0], buffer);

        let input = Paragraph::new(self.input.value())
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .border_type(BorderType::Rounded)
                    .borders(Borders::ALL)
                    .title(match context.input_mode {
                        // TODO: Make the key description dynamic based on a Config File.
                        InputMode::Normal => " Press 'e' to edit or 'r' to record ",
                        InputMode::Editing => " Editing ",
                        InputMode::Recording => " Recording… Press 'Esc' to stop ",
                    })
                    .border_style(Style::default().fg(match context.input_mode {
                        InputMode::Normal => Color::DarkGray,
                        InputMode::Editing => Color::Yellow,
                        InputMode::Recording => Color::Red,
                    })),
            );
        input.render(chunks[1], buffer);

        let mode_indicator = match context.input_mode {
            InputMode::Normal => " NORMAL ",
            InputMode::Editing => " EDITING ",
            InputMode::Recording => " RECORDING ",
        };
        let instructions = Paragraph::new(format!(
            "Mode:{} | Enter: confirm | Esc: cancel",
            mode_indicator
        ))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
        instructions.render(chunks[2], buffer);
    }
}
