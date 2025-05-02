// /ui/save_name_input.rs
use crate::{
    app::{Action, InputMode},
    audio::Transcription,
    context::Context,
};
use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    prelude::Alignment,
    style::{Color, Style},
    widgets::*,
};
use tui_textarea::TextArea;

use super::{Component, ComponentEnum, game::SectionMove, main_menu::MainMenu, textarea::*};

#[derive(Default, Debug)]
pub struct SaveName {
    textarea: TextArea<'static>,
    vim: Vim,
}

impl SaveName {
    pub fn new() -> Self {
        let mut save_name = SaveName::default();
        save_name
            .textarea
            .set_placeholder_text("Input your Save Name");
        save_name.textarea.set_cursor_line_style(Style::default());
        save_name
            .textarea
            .set_placeholder_style(Style::default().fg(Color::DarkGray));
        save_name
    }
}

impl Component for SaveName {
    fn on_key(&mut self, key: KeyEvent, context: Context) -> Option<Action> {
        match self.vim.transition(key.into(), &mut self.textarea) {
            Transition::Mode(mode) if self.vim.mode != mode => {
                self.textarea
                    .set_block(mode.block().border_type(BorderType::Rounded));
                self.textarea.set_cursor_style(mode.cursor_style());
                self.vim.mode = mode;
                match mode {
                    Mode::Recording => Some(Action::SwitchInputMode(InputMode::Recording)),
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
                if !self.textarea.lines().is_empty() {
                    Some(Action::CreateNewGame(self.textarea.lines()[0].to_string()))
                } else {
                    None
                }
            }
            Transition::Exit => Some(Action::SwitchComponent(ComponentEnum::from(
                MainMenu::default(),
            ))),
            Transition::Detail(_section_move) => None,
        }
    }
    fn render(&mut self, area: Rect, buffer: &mut Buffer, context: &Context) {
        self.textarea.set_block(Mode::Normal.block());
        self.textarea.set_cursor_style(Mode::Normal.cursor_style());

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

        self.textarea.render(chunks[1], buffer);

        let mode_indicator = match context.input_mode {
            InputMode::Normal => " NORMAL ",
            InputMode::Editing => " EDITING ",
            InputMode::Recording(_) => " RECORDING ",
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

impl SaveName {
    fn check_transcription(&mut self) {
        if let Some(receiver) = &mut self.receiver {
            if let Ok(transcription) = receiver.try_recv() {
                let input_value = format!("{} {}", self.input.value(), transcription);
                self.input = Input::with_value(self.input.clone(), input_value);
                self.receiver = None;
            }
        }
    }
}
