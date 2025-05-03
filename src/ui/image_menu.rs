use std::{path::PathBuf, thread::sleep, time::Duration};

use crate::{
    app::{Action, InputMode},
    audio::Transcription,
    context::Context,
    imager,
};
use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::{Alignment, Buffer, Rect},
    style::{Color, Modifier, Style},
    widgets::*,
};
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tui_textarea::TextArea;

use super::{
    Component, ComponentEnum, MainMenu,
    api_key_input::ApiKeyInput,
    center_rect,
    textarea::{Mode, Transition, Vim, new_textarea},
};

#[derive(Debug)]
pub struct ImageMenu {
    textarea: TextArea<'static>,
    vim: Vim,
    receiver: Option<UnboundedReceiver<String>>,
    image_sender: mpsc::UnboundedSender<PathBuf>,
}

impl ImageMenu {
    pub fn new(image_sender: mpsc::UnboundedSender<PathBuf>) -> Self {
        Self {
            textarea: new_textarea("Input a prompt to generate your image"),
            vim: Vim::new(Mode::Normal),
            receiver: None,
            image_sender,
        }
    }

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

    fn request_image(&mut self, context: &mut Context<'_>) -> Option<Action> {
        if self.textarea.lines().concat().len() < 2 {
            return Some(Action::SwitchInputMode(InputMode::Editing));
        }
        let prompt = self.textarea.lines().join("\n");
        let image_sender = self.image_sender.clone();
        log::info!("Requested image creation with context: {context:#?}");
        if let Some(client) = context.ai_client.clone() {
            log::debug!("Spawning  the image generation");
            tokio::spawn(async move {
                log::debug!("Spawned  the image generation");
                let path = imager::generate_and_save_image(client, &prompt)
                    .await
                    .expect("Expected a valid image path");

                if let Err(e) = image_sender.send(path) {
                    log::error!("Failed to send path: {:#?}", e)
                }
            });

            self.textarea =
                new_textarea("Your Image is being generated, it will open when ready...");
            self.textarea.set_placeholder_style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::SLOW_BLINK),
            );
            sleep(Duration::from_secs(3));
            Some(Action::SwitchComponent(ComponentEnum::from(
                MainMenu::default(),
            )))
        } else {
            Some(Action::SwitchComponent(ComponentEnum::from(
                ApiKeyInput::new(&context.settings.openai_api_key),
            )))
        }
    }
}

impl Component for ImageMenu {
    fn on_key(&mut self, key: KeyEvent, context: &mut Context) -> Option<Action> {
        match self.vim.transition(key.into(), &mut self.textarea) {
            Transition::Mode(mode) if self.vim.mode != mode => {
                self.vim.mode = mode;
                self.textarea
                    .set_block(mode.block().border_type(BorderType::Rounded));
                self.textarea.set_cursor_style(mode.cursor_style());
                match mode {
                    Mode::Recording => {
                        if !context.settings.audio_input_enabled {
                            return None;
                        };
                        if let Ok((receiver, transcription)) =
                            Transcription::new(None, context.ai_client.clone().unwrap())
                        {
                            self.receiver = Some(receiver);
                            log::debug!("Sent the recording request");
                            Some(Action::SwitchInputMode(InputMode::Recording(transcription)))
                        } else {
                            None
                        }
                    }
                    Mode::Normal => {
                        self.vim.mode = Mode::Normal;
                        Some(Action::SwitchInputMode(InputMode::Normal))
                    }
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
                    self.request_image(context)
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

    // TODO: Implement an image viewer here.
    fn render(&mut self, area: Rect, buffer: &mut Buffer, context: &Context) {
        self.textarea.set_block(self.vim.mode.block());
        self.check_transcription();
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

        let title = Paragraph::new(" Enter an image prompt ")
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center);
        title.render(chunks[0], buffer);

        self.textarea.render(chunks[1], buffer);

        let mode_indicator = match context.input_mode {
            InputMode::Normal => " NORMAL ",
            InputMode::Editing => " EDITING ",
            InputMode::Recording(_) => " RECORDING ",
        };
        let instructions =
            Paragraph::new(format!("{} | Enter: confirm | Esc: cancel", mode_indicator))
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);
        instructions.render(chunks[2], buffer);
    }
}

// Function to draw the image creation interface in the application.
