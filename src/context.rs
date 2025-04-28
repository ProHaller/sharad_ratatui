use async_openai::{Client, config::OpenAIConfig};
use copypasta::ClipboardContext;

use crate::{
    app::InputMode, audio::AudioNarration, message::Message, save::SaveManager, settings::Settings,
};

pub struct Context<'a> {
    // TODO: Make the openai_api_key_valid a date
    pub ai_client: Option<Client<OpenAIConfig>>,
    pub save_manager: &'a mut SaveManager,
    pub settings: &'a mut Settings,
    pub clipboard: ClipboardContext,
    pub messages: &'a Vec<Message>,
    pub input_mode: &'a InputMode, // TODO: Move it into Input struct
    pub audio_narration: &'a mut AudioNarration,
}
