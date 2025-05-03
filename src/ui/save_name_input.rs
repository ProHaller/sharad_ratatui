// /ui/save_name_input.rs
use crate::{
    app::{Action, InputMode},
    audio::Transcription,
    context::Context,
};
use crossterm::event::KeyEvent;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    prelude::Alignment,
    style::{Color, Style},
    widgets::*,
};
use tokio::sync::mpsc::UnboundedReceiver;
use tui_textarea::TextArea;

use super::{Component, ComponentEnum, center_rect, main_menu::MainMenu, textarea::*};

#[derive(Default, Debug)]
pub struct SaveName {
    textarea: TextArea<'static>,
    vim: Vim,
    receiver: Option<UnboundedReceiver<String>>,
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
                            return None;
                        };
                        if let Ok((receiver, transcription)) =
                            Transcription::new(None, context.ai_client.clone().unwrap())
                        {
                            self.receiver = Some(receiver);
                            Some(Action::SwitchInputMode(InputMode::Recording(transcription)))
                        } else {
                            None
                        }
                    }
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
    fn render(&mut self, area: Rect, buffer: &mut Buffer, _context: &Context) {
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

        let title = Paragraph::new(" Enter Save Name ")
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center);
        title.render(chunks[0], buffer);

        self.textarea.set_block(self.vim.mode.block());
        self.check_transcription();
        self.textarea.render(chunks[1], buffer);
    }
}

impl SaveName {
    fn check_transcription(&mut self) {
        if let Some(receiver) = &mut self.receiver {
            if let Ok(transcription) = receiver.try_recv() {
                self.textarea.set_yank_text(transcription);
                self.textarea.paste();
                self.textarea.set_cursor_style(self.vim.mode.cursor_style());
                self.receiver = None;
            }
        }
    }
}
