use std::{fmt, path::PathBuf};

use async_openai::{Client, config::OpenAIConfig};
use copypasta::ClipboardContext;
use tokio::sync::mpsc;

use crate::{
    app::InputMode, audio::AudioNarration, message::Message, save::SaveManager, settings::Settings,
};

pub struct Context<'a> {
    // TODO: Make the openai_api_key_valid a date
    pub ai_client: Option<Client<OpenAIConfig>>,
    pub image_sender: mpsc::UnboundedSender<PathBuf>,
    pub save_manager: &'a mut SaveManager,
    pub settings: &'a mut Settings,
    pub clipboard: ClipboardContext,
    pub messages: &'a Vec<Message>,
    pub input_mode: &'a InputMode, // TODO: Move it into Input struct
    pub audio_narration: &'a mut AudioNarration,
}

impl fmt::Debug for Context<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .field("ai_client", &self.ai_client)
            .field("image_sender", &"UnboundedSender<PathBuf>")
            .field("save_manager", &"SaveManager")
            .field("settings", &self.settings)
            .field("clipboard", &"<ClipboardContext omitted>")
            .field("messages", &self.messages)
            .field("input_mode", &self.input_mode)
            .field("audio_narration", &"AudioNarration")
            .finish()
    }
}
