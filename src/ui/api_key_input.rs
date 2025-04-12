// ui/api_key_input.rs

use std::thread;

use crate::{
    app::{Action, App, InputMode},
    context::{self, Context},
    settings::Settings,
};
use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Position},
    prelude::{Alignment, Buffer, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::*,
};
use tokio::runtime::Handle;
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

use super::{Component, SettingsMenu, center_rect};

#[derive(Debug, Default)]
pub struct ApiKeyInput {
    input: Input,
}

impl Component for ApiKeyInput {
    fn on_key(&mut self, key: KeyEvent, context: Context) -> Option<Action> {
        match context.input_mode {
            InputMode::Normal => self.handle_normal_input(key, context),
            InputMode::Editing => self.handle_editing(key, context),
            InputMode::Recording => Some(Action::SwitchInputMode(InputMode::Normal)),
        }
    }

    fn render(&self, area: Rect, buffer: &mut Buffer, context: &Context) {
        let centered_area =
            center_rect(area, Constraint::Percentage(70), Constraint::Percentage(50));
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .flex(ratatui::layout::Flex::Center)
            .constraints(
                [
                    Constraint::Length(1),
                    Constraint::Length(3),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ]
                .as_ref(),
            )
            .split(centered_area);

        let (title, normal_style) = match context.openai_api_key_valid {
            true => {
                let title = Paragraph::new(" Your API Key is valid ".bold())
                    .style(Style::default().fg(Color::Green))
                    .alignment(Alignment::Center);
                let normal_style = Style::default().fg(Color::Green);
                (title, normal_style)
            }
            false => {
                let title = Paragraph::new(" Please input a Valid Api_key ")
                    .style(Style::default().fg(Color::Red))
                    .alignment(Alignment::Center);
                let normal_style = Style::default().fg(Color::Red);
                (title, normal_style)
            }
        };
        let style = match context.input_mode {
            InputMode::Normal => normal_style,
            InputMode::Editing => Style::default().fg(Color::Yellow),
            InputMode::Recording => Style::default().bg(Color::Red),
        };
        let input_field = Paragraph::new(self.input.value()).block(
            Block::default()
                .border_type(BorderType::Rounded)
                .borders(Borders::ALL)
                .border_style(style)
                .title(" API Key "),
        );
        title.render(chunks[0], buffer);
        input_field.render(chunks[1], buffer);

        let instructions = Paragraph::new(" Press e to edit, Enter to confirm, Esc to cancel ")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        instructions.render(chunks[2], buffer);

        let paste_info = Paragraph::new(" Use Ctrl+V to paste ")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        paste_info.render(chunks[3], buffer);
        // TODO: Make sure the cursor is properly set.
    }
}

impl ApiKeyInput {
    fn handle_editing(&mut self, key: KeyEvent, mut context: Context) -> Option<Action> {
        match key.code {
            KeyCode::Enter => {
                if !self.input.value().is_empty() {
                    self.validate_key(&mut context)
                } else {
                    Some(Action::SwitchInputMode(InputMode::Editing))
                }
            }
            KeyCode::Esc => Some(Action::SwitchInputMode(InputMode::Normal)),
            KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.paste(context)
            }
            _ => {
                self.input.handle_event(&crossterm::event::Event::Key(key));
                None
            }
        }
    }

    fn validate_key(&mut self, context: &mut Context<'_>) -> Option<Action> {
        let api_key = self.input.value().to_string();

        context.openai_api_key_valid = tokio::task::block_in_place(|| {
            Handle::current().block_on(Settings::validate_api_key(&api_key))
        });

        if context.openai_api_key_valid {
            context.settings.openai_api_key = Some(api_key.clone());
            Some(Action::SwitchInputMode(InputMode::Normal))
        } else {
            self.input.reset();
            self.input = Input::new("Invalid api key".into());
            None
        }
    }
    fn handle_normal_input(&mut self, key: KeyEvent, mut context: Context) -> Option<Action> {
        match key.code {
            KeyCode::Enter => {
                if !context.openai_api_key_valid {
                    self.validate_key(&mut context)
                } else {
                    Some(Action::SwitchComponent(Box::new(SettingsMenu::new(
                        context,
                    ))))
                }
            }
            KeyCode::Char('e') => Some(Action::SwitchInputMode(InputMode::Editing)),
            KeyCode::Esc => Some(Action::SwitchComponent(Box::new(SettingsMenu::new(
                context,
            )))),
            KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.paste(context)
            }
            _ => None,
        }
    }
    fn paste(&mut self, context: Context) -> Option<Action> {
        let mut clipboard = context.clipboard;
        self.input = Input::default().with_value(
            clipboard
                .get_contents()
                .expect("Expected a string from clipboard."),
        );
        None
    }
}
