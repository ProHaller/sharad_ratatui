use copypasta::ClipboardContext;

use crate::ai::GameAI;
use crate::{app::InputMode, message::Message, save::SaveManager, settings::Settings};
use std::cell::RefCell;

pub struct Context<'a> {
    // TODO: Make the openai_api_key_valid a date
    pub openai_api_key_valid: bool,
    pub save_manager: &'a mut SaveManager,
    pub save_name: &'a str,
    pub ai_client: &'a Option<GameAI>,
    pub settings: &'a mut Settings,
    pub clipboard: &'a ClipboardContext,
    pub console_messages: &'a RefCell<Vec<Message>>,
    pub input_mode: &'a InputMode, // TODO: Move it into Input struct
}
