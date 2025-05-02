// ui/api_key_input.rs

use crate::{
    app::{Action, InputMode},
    context::Context,
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
    Component, ComponentEnum, MainMenu, SettingsMenu, center_rect,
    textarea::{Mode, Transition, Vim},
};

#[derive(Debug)]
pub struct ApiKeyInput {
    textarea: TextArea<'static>,
    vim: Vim,
}

impl Component for ApiKeyInput {
    fn on_key(&mut self, key: KeyEvent, mut context: Context) -> Option<Action> {
        match self.vim.transition(key.into(), &mut self.textarea) {
            Transition::Mode(mode) if self.vim.mode != mode => {
                self.textarea
                    .set_block(mode.block().border_type(BorderType::Rounded));
                self.textarea.set_cursor_style(mode.cursor_style());
                self.vim.mode = mode;
                match mode {
                    Mode::Recording => None,
                    Mode::Normal => Some(Action::SwitchInputMode(InputMode::Normal)),
                    Mode::Insert => Some(Action::SwitchInputMode(InputMode::Editing)),
                    Mode::Visual => Some(Action::SwitchInputMode(InputMode::Normal)),
                    Mode::Operator(_) => None,
                }
            }
            Transition::Nop | Transition::Mode(_) => None,
            Transition::Pending(input) => {
                self.vim.pending = input;
                None
            }
            Transition::Validation => {
                if context.ai_client.is_none() {
                    self.validate_key(&mut context)
                } else {
                    Some(Action::SwitchComponent(ComponentEnum::from(
                        SettingsMenu::new(context),
                    )))
                }
            }
            Transition::Exit => Some(Action::SwitchComponent(ComponentEnum::from(
                MainMenu::default(),
            ))),
            Transition::Detail(_section_move) => None,
            Transition::EndRecording => {
                log::debug!("Transition::EndRecording");
                self.vim.mode = Mode::Normal;
                Some(Action::EndRecording)
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
            InputMode::Recording(_) => Style::default().bg(Color::Red),
        };

        self.textarea.set_block(Mode::Normal.block().style(style));
        title.render(chunks[0], buffer);
        self.textarea.render(chunks[1], buffer);

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
        let mut api_key_input = Self {
            textarea: TextArea::default(),
            vim: Vim::new(Mode::Normal),
        };
        if let Some(api_key) = api_key {
            api_key_input.textarea.set_placeholder_text(api_key);
            api_key_input.textarea.set_mask_char('*');
            api_key_input
                .textarea
                .set_cursor_line_style(Style::default());
            api_key_input
                .textarea
                .set_placeholder_style(Style::default().fg(Color::DarkGray));
        }
        api_key_input
    }

    fn validate_key(&mut self, context: &mut Context<'_>) -> Option<Action> {
        let api_key = self.textarea.lines()[0].to_string();

        context.ai_client = tokio::task::block_in_place(|| {
            Handle::current().block_on(Settings::validate_ai_client(&api_key))
        });

        if context.ai_client.is_some() {
            context.settings.openai_api_key = Some(api_key.clone());
            Some(Action::SwitchInputMode(InputMode::Normal))
        } else {
            self.textarea = TextArea::default();
            self.textarea.set_placeholder_text("This key is invalid");
            None
        }
    }
}
