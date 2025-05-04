// ui/api_key_input.rs

use crate::{
    app::{Action, InputMode},
    context::Context,
    save::get_game_data_dir,
    settings::Settings,
};
use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::{Alignment, Buffer, Rect},
    style::{Color, Style, Stylize},
    widgets::*,
};
use tokio::runtime::Handle;
use tui_textarea::TextArea;

use super::{
    Component, ComponentEnum, SettingsMenu, center_rect,
    textarea::{Mode, Transition, Vim, Warning, new_textarea},
};

#[derive(Debug)]
pub struct ApiKeyInput {
    textarea: TextArea<'static>,
    vim: Vim,
}

impl Component for ApiKeyInput {
    fn on_key(&mut self, key: KeyEvent, context: &mut Context) -> Option<Action> {
        match self.vim.transition(key.into(), &mut self.textarea) {
            Transition::Mode(mode) if self.vim.mode != mode => {
                self.textarea
                    .set_block(mode.block().border_type(BorderType::Rounded));
                self.textarea.set_cursor_style(mode.cursor_style());
                self.vim.mode = mode;
                match mode {
                    Mode::Recording => {
                        if !context.settings.audio_input_enabled {
                            self.vim.mode = Mode::Warning(Warning::AudioInputDisabled);
                            return None;
                        };
                        self.textarea.set_cursor_style(mode.cursor_style());
                        None
                    }
                    Mode::Normal => Some(Action::SwitchInputMode(InputMode::Normal)),
                    Mode::Insert => Some(Action::SwitchInputMode(InputMode::Editing)),
                    Mode::Visual => Some(Action::SwitchInputMode(InputMode::Normal)),
                    Mode::Operator(_) => None,
                    Mode::Warning(warning) => None,
                }
            }
            Transition::Nop | Transition::Mode(_) => None,
            Transition::Pending(input) => {
                self.vim.pending = input;
                None
            }
            Transition::Validation => self.validate_key(context),
            Transition::Exit => Some(Action::SwitchComponent(ComponentEnum::from(
                SettingsMenu::new(context),
            ))),
            Transition::Detail(_section_move) => None,
            Transition::EndRecording => {
                self.vim.mode = Mode::Normal;
                None
            }
            Transition::ScrollTop => None,
            Transition::ScrollBottom => None,
            Transition::PageUp => None,
            Transition::PageDown => None,
            Transition::ScrollUp => None,
            Transition::ScrollDown => None,
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
                ]
                .as_ref(),
            )
            .split(centered_area);

        let title = match context.ai_client {
            Some(_) => {
                let title = Paragraph::new(" Your Api Key is valid! ".bold())
                    .style(Style::default().fg(Color::Green))
                    .alignment(Alignment::Center);

                title
            }
            None => {
                let title = Paragraph::new(" Please input a Valid Api Key ")
                    .style(Style::default().fg(Color::Red))
                    .alignment(Alignment::Center);
                log::debug!("Title set to: {title:#?}");
                title
            }
        };

        self.textarea.set_block(Mode::Normal.block());
        self.textarea.render(chunks[1], buffer);
        title.render(chunks[0], buffer);

        let paste_info =
            Paragraph::new(" Use Ctrl+v or 'p' to paste, or insert 'reset' to reset your Api Key ")
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);
        paste_info.render(chunks[2], buffer);
        // TODO: Make sure the cursor is properly set.
    }
}

impl ApiKeyInput {
    pub fn new(api_key: &Option<String>) -> Self {
        let textarea = new_textarea_with_key(api_key);
        Self {
            textarea,
            vim: Vim::new(Mode::Normal),
        }
    }

    fn reset_key(&mut self, context: &mut Context<'_>) {
        *context.ai_client = None;
        context.settings.openai_api_key = None;
        log::info!("context reset: {:#?}", context);
        if let Err(e) = context
            .settings
            .save_to_file(get_game_data_dir().join("settings.json"))
        {
            log::error!("Failed to save_to_file: {e:#?}");
            self.textarea = new_textarea(
                "The Api key Reset could not be saved to file. Please delete your settings file manually.",
            );
            self.textarea
                .set_placeholder_style(Style::new().fg(Color::Red));
        } else {
            self.textarea = new_textarea("Your Api key has been reset.");
        }
    }

    fn validate_key(&mut self, context: &mut Context<'_>) -> Option<Action> {
        let Some(api_ref) = self.textarea.lines().first() else {
            self.textarea =
                new_textarea("Please input a valid Api Key (or 'reset' to reset you Api Key)");
            return None;
        };
        let api_key = api_ref.to_string();
        if api_ref.to_lowercase().starts_with("reset") {
            self.reset_key(context);
            log::info!("Reset key done");
            return Some(Action::SwitchComponent(ComponentEnum::ApiKeyInput(
                ApiKeyInput::new(&context.settings.openai_api_key),
            )));
        }
        self.textarea = new_textarea(" Please wait a moment while we verify the key");

        let new_ai_client = tokio::task::block_in_place(|| {
            Handle::current().block_on(Settings::validate_ai_client(&api_key))
        });

        log::debug!("new_ai_client: {new_ai_client:#?}");
        if new_ai_client.is_some() {
            *context.ai_client = new_ai_client;
            context.settings.openai_api_key = Some(api_key);
            if let Err(e) = context.settings.save() {
                log::error!("Failed to save to default path: {:#?}", e);
            }
            log::debug!("New context set: {context:#?}");
            self.textarea = new_textarea(" Your Api Key is Valid!");
            Some(Action::SwitchInputMode(InputMode::Normal))
        } else {
            self.textarea = new_textarea("This key is invalid");
            None
        }
    }
}

pub fn new_textarea_with_key(api_key: &Option<String>) -> TextArea<'static> {
    match api_key {
        None => new_textarea("Please input a valid Api key"),
        Some(api_key) => new_textarea(hide_api(api_key)),
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

    format!(" Valid Api Key: {}...{}", head, tail)
}
