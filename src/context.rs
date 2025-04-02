// TODO: Make the openai_api_key_valid a date

use copypasta::ClipboardContext;

use crate::ai::GameAI;
use crate::error::ErrorMessage;
use crate::{Message, app::InputMode, save::SaveManager, settings::Settings};
use std::cell::RefCell;

pub(crate) struct Context<'a> {
    pub openai_api_key_valid: bool,
    pub save_manager: &'a SaveManager,
    pub save_name: &'a str,
    pub ai_client: &'a Option<GameAI>,
    pub settings: &'a Settings,
    pub clipboard: &'a ClipboardContext,
    pub console_messages: &'a RefCell<Vec<Message>>,
    pub error_messages: &'a Vec<ErrorMessage>,
    pub input_mode: &'a InputMode, // TODO: Move it into Input struct
}
