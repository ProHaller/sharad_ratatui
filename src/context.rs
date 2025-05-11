use std::path::PathBuf;

use async_openai::{Client, config::OpenAIConfig};
use ratatui::layout::Size;
use tokio::sync::mpsc;

use crate::{
    app::InputMode, audio::AudioNarration, message::Message, save::SaveManager, settings::Settings,
};

#[allow(dead_code)]
#[derive(Debug)]
pub struct Context<'a> {
    // TODO: Make the openai_api_key_valid a date
    pub ai_client: &'a mut Option<Client<OpenAIConfig>>,
    pub size: &'a mut Size,
    pub image_sender: mpsc::UnboundedSender<PathBuf>,
    pub save_manager: &'a mut SaveManager,
    pub settings: &'a mut Settings,
    pub messages: &'a Vec<Message>,
    pub input_mode: &'a InputMode, // TODO: Move it into Input struct
    pub audio_narration: &'a mut AudioNarration,
}
