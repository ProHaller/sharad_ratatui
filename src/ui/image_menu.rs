use std::path::PathBuf;

use crate::{
    app::{Action, InputMode},
    audio::Transcription,
    context::Context,
    imager,
};
use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin},
    prelude::{Alignment, Buffer, Rect},
    style::{Color, Style},
    widgets::*,
};
use ratatui_image::{StatefulImage, protocol::StatefulProtocol};
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tui_textarea::TextArea;

use super::{
    Component, ComponentEnum, MainMenu,
    api_key_input::ApiKeyInput,
    center_rect,
    textarea::{Mode, Transition, Vim, Warning, new_textarea},
};

pub struct ImageMenu {
    textarea: TextArea<'static>,
    vim: Vim,
    transcription_receiver: Option<UnboundedReceiver<String>>,
    image_sender: mpsc::UnboundedSender<PathBuf>,
    pub image: Option<StatefulProtocol>,
}

impl std::fmt::Debug for ImageMenu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageMenu")
            .field("textarea", &self.textarea)
            .field("vim", &self.vim)
            .field("transcription_receiver", &self.transcription_receiver)
            .field("image_sender", &self.image_sender)
            .field(
                "image",
                if self.image.is_some() {
                    &"Some"
                } else {
                    &"None"
                },
            )
            .finish()
    }
}

impl ImageMenu {
    pub fn new(image_sender: mpsc::UnboundedSender<PathBuf>) -> Self {
        Self {
            textarea: new_textarea("Input a prompt to generate your image"),
            vim: Vim::new(Mode::Normal),
            transcription_receiver: None,
            image_sender,
            image: None,
        }
    }

    fn check_transcription(&mut self) {
        if let Some(receiver) = &mut self.transcription_receiver {
            if let Ok(transcription) = receiver.try_recv() {
                self.textarea.set_yank_text(transcription);
                self.textarea.paste();
                self.textarea.set_cursor_style(self.vim.mode.cursor_style());
                self.transcription_receiver = None;
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
            self.textarea
                .set_placeholder_style(Style::default().fg(Color::LightGreen));
            // TODO: Add a spinner
            None
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
                            self.vim.mode = Mode::Warning(Warning::AudioInputDisabled);
                            return None;
                        };
                        self.textarea.set_placeholder_text("   Recording...");
                        if let Ok((receiver, transcription)) =
                            Transcription::new(None, context.ai_client.clone().unwrap())
                        {
                            self.transcription_receiver = Some(receiver);
                            log::debug!("Sent the recording request");
                            Some(Action::SwitchInputMode(InputMode::Recording(transcription)))
                        } else {
                            self.vim.mode = Mode::Warning(Warning::FailedNewTranscription);
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
                    Mode::Warning(_) => None,
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
        if self.image.is_some() {
            self.textarea.set_placeholder_text("");
        }
        let horizontal_split =
            Layout::horizontal([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)]).split(area);
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
            .split(if self.image.is_none() {
                centered_area
            } else {
                horizontal_split[1]
            });

        let title = Paragraph::new(" Enter an image prompt ")
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center);
        title.render(chunks[0], buffer);

        self.textarea.render(chunks[1], buffer);

        if let Some(image) = &mut self.image {
            // HACK: Probably a better way to render the image.
            let image_block = Block::default()
                .border_type(BorderType::Rounded)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White));

            image_block.render(horizontal_split[0], buffer);
            // FIX: How to make the first rendering faster? Pre-rendering?
            StatefulImage::new().render(
                horizontal_split[0].inner(Margin::new(1, 1)),
                buffer,
                image,
            );
        }
    }
}

// Function to draw the image creation interface in the application.
