use copypasta::ClipboardContext;
use ratatui_image::picker::Picker;

use crate::{app::InputMode, message::Message, save::SaveManager, settings::Settings};
use std::cell::RefCell;

pub struct Context<'a> {
    // TODO: Make the openai_api_key_valid a date
    pub openai_api_key_valid: bool,
    pub save_manager: &'a mut SaveManager,
    pub settings: &'a mut Settings,
    pub clipboard: ClipboardContext,
    pub console_messages: &'a RefCell<Vec<Message>>,
    pub input_mode: &'a InputMode, // TODO: Move it into Input struct
}
