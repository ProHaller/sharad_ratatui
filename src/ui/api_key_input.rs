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

use super::{Component, ComponentEnum, SettingsMenu, center_rect, input::Pastable};

#[derive(Debug)]
pub struct ApiKeyInput {
    input: Input,
}

impl Component for ApiKeyInput {
    fn on_key(&mut self, key: KeyEvent, context: Context) -> Option<Action> {
        match context.input_mode {
            InputMode::Normal => self.handle_normal_input(key, context),
            InputMode::Editing => self.handle_editing_input(key, context),
            // TODO: handle the voice recording
            InputMode::Recording => Some(Action::SwitchInputMode(InputMode::Normal)),
        }
    }

    fn render(&mut self, area: Rect, buffer: &mut Buffer, context: &Context) {
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

        let (title, normal_style) = match context.ai_client {
            Some(_) => {
                let title = Paragraph::new(" Your API Key is valid ".bold())
                    .style(Style::default().fg(Color::Green))
                    .alignment(Alignment::Center);
                let normal_style = Style::default().fg(Color::Green);
                (title, normal_style)
            }
            None => {
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
        let input_field = Paragraph::new(self.input.value())
            .style(Style::new().fg(Color::DarkGray))
            .block(
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
    pub fn new(api_key: &Option<String>) -> Self {
        Self {
            input: api_input_default(api_key),
        }
    }

    fn handle_editing_input(&mut self, key: KeyEvent, mut context: Context) -> Option<Action> {
        // HACK: I should make this into a more robust check for the different cases. maybe an
        // enum
        if self.input.value().contains(' ') {
            self.input.reset();
        }

        match key.code {
            KeyCode::Enter => {
                if !self.input.value().is_empty() {
                    self.validate_key(&mut context)
                } else {
                    Some(Action::SwitchInputMode(InputMode::Editing))
                }
            }
            KeyCode::Esc => {
                self.input = api_input_default(&context.settings.openai_api_key);
                Some(Action::SwitchInputMode(InputMode::Normal))
            }
            KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.reset();
                self.input.paste(context);
                None
            }
            _ => {
                self.input.handle_event(&crossterm::event::Event::Key(key));
                None
            }
        }
    }

    fn validate_key(&mut self, context: &mut Context<'_>) -> Option<Action> {
        let api_key = self.input.value().to_string();

        context.ai_client = tokio::task::block_in_place(|| {
            Handle::current().block_on(Settings::validate_ai_client(&api_key))
        });

        if context.ai_client.is_some() {
            context.settings.openai_api_key = Some(api_key.clone());
            Some(Action::SwitchInputMode(InputMode::Normal))
        } else {
            self.input.reset();
            self.input = Input::new("This key is invalid".into());
            None
        }
    }
    fn handle_normal_input(&mut self, key: KeyEvent, mut context: Context) -> Option<Action> {
        match key.code {
            KeyCode::Enter => {
                if context.ai_client.is_none() {
                    self.validate_key(&mut context)
                } else {
                    Some(Action::SwitchComponent(ComponentEnum::from(
                        SettingsMenu::new(context),
                    )))
                }
            }
            KeyCode::Char('e') => {
                self.input = self
                    .input
                    .clone()
                    .with_value("Editing this will delete your current key".into());
                Some(Action::SwitchInputMode(InputMode::Editing))
            }
            KeyCode::Esc => Some(Action::SwitchComponent(ComponentEnum::from(
                SettingsMenu::new(context),
            ))),
            KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.reset();
                self.input.paste(context);
                None
            }
            _ => None,
        }
    }
}

pub fn api_input_default(api_key: &Option<String>) -> Input {
    match api_key {
        None => Input::new("Please input a valid API key".into()),
        Some(api_key) => Input::new(hide_api(&api_key)),
    }
}
fn hide_api(s: &str) -> String {
    let head_len = 7;
    let tail_len = 3;

    if s.len() < head_len + tail_len + 3 {
        return s.to_string();
    }

    let head = &s[..head_len];
    let tail = &s[s.len() - tail_len..];

    format!("{}...{}", head, tail)
}
