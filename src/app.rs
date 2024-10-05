use crate::ai::{GameAI, GameConversationState};
use crate::ai_response::{create_user_message, UserMessage};
use crate::app_state::AppState;
use crate::assistant::{create_assistant, delete_assistant, get_assistant_id};
use crate::audio::{self, play_audio};
use crate::character::CharacterSheet;
use crate::cleanup::cleanup;
use crate::error::{send_global_error, AppError, ErrorMessage, ShadowrunError};
use crate::game_state::GameState;
use crate::image;
use crate::message::{self, AIMessage, GameMessage, Message, MessageType};
use crate::save::SaveManager;
use crate::settings::Settings;
use crate::settings_state::SettingsState;
use crate::ui::utils::Spinner;
use crate::ui::{game, game::HighlightedSection};

use chrono::Local;
use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use futures::stream::{FuturesOrdered, StreamExt};
use ratatui::widgets::ListState;
use ratatui::{layout::Alignment, text::Line};
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::fs::OpenOptions;
use std::io::Write;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::sync::{mpsc, Mutex};
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;
use tui_input::InputRequest;

pub enum AppCommand {
    LoadGame(String),
    StartNewGame(String),
    ProcessMessage(String),
    AIResponse(Result<GameMessage, AppError>),
    ApiKeyValidationResult(bool),
    TranscriptionResult(String, TranscriptionTarget),
    TranscriptionError(String),
}

pub enum TranscriptionTarget {
    UserInput,
    SaveNameInput,
    ImagePrompt,
}

#[derive(PartialEq, Eq, Clone)]
pub enum InputMode {
    Normal,
    Editing,
    Recording,
}

pub struct App {
    // Application state and control flow
    pub should_quit: bool,
    pub state: AppState,
    pub highlighted_section: HighlightedSection,
    pub input_mode: InputMode,
    pub openai_api_key_valid: bool,
    pub error_messages: Vec<ErrorMessage>,

    // Saves and loads
    pub save_manager: SaveManager,
    pub save_name_input: Input,
    pub current_save_name: Arc<RwLock<String>>,

    // Menu states
    pub main_menu_state: ListState,
    pub load_game_menu_state: ListState,
    pub settings_state: SettingsState,

    // Game state and AI interaction
    pub ai_client: Option<GameAI>,
    pub current_game: Option<Arc<Mutex<GameState>>>,
    pub current_game_response: Option<GameMessage>,

    // User inputs and interaction handling
    pub user_input: Input,
    pub api_key_input: Input,
    pub image_prompt: Input,
    pub is_recording: Arc<AtomicBool>,

    // Game content management
    pub game_content: RefCell<Vec<message::Message>>,
    pub visible_messages: usize,
    pub game_content_scroll: usize,
    pub cached_game_content: Option<Rc<Vec<(Line<'static>, Alignment)>>>,
    pub cached_content_len: usize,
    pub visible_lines: usize,
    pub total_lines: usize,
    pub message_line_counts: Vec<usize>,
    pub last_user_message: Option<UserMessage>,

    // Debugging and logging
    pub debug_info: RefCell<String>,

    // Settings and configurations
    pub settings: Settings,

    // Clipboard handling
    clipboard: ClipboardContext,

    // Asynchronous message handling
    ai_sender: mpsc::UnboundedSender<AIMessage>,
    pub command_sender: mpsc::UnboundedSender<AppCommand>,

    // UI components and helpers
    pub backspace_counter: bool,
    pub spinner: Spinner,
    pub spinner_active: bool,
    pub last_spinner_update: Instant,

    // Last known data
    pub last_known_character_sheet: Option<CharacterSheet>,
}

impl App {
    pub async fn new(
        ai_sender: mpsc::UnboundedSender<AIMessage>,
    ) -> (Self, mpsc::UnboundedReceiver<AppCommand>) {
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        let mut main_menu_state = ListState::default();
        main_menu_state.select(Some(0));

        let settings =
            Settings::load_settings_from_file("./data/settings.json").unwrap_or_default();
        let settings_state = SettingsState::from_settings(&settings);

        let mut load_game_menu_state = ListState::default();
        load_game_menu_state.select(Some(0));

        let openai_api_key_valid = if let Some(ref api_key) = settings.openai_api_key {
            Settings::validate_api_key(api_key).await
        } else {
            false
        };

        let app = Self {
            should_quit: false,
            state: AppState::MainMenu,
            highlighted_section: HighlightedSection::None,
            main_menu_state,
            ai_client: None,
            current_game: None,
            command_sender,
            settings,
            user_input: Input::default(),
            api_key_input: Input::default(),
            save_name_input: Input::default(),
            image_prompt: Input::default(),
            input_mode: InputMode::Normal,
            settings_state,
            save_manager: SaveManager::new(),
            load_game_menu_state,
            openai_api_key_valid,
            error_messages: Vec::new(),
            game_content: RefCell::new(Vec::new()),
            game_content_scroll: 0,
            cached_game_content: None,
            cached_content_len: 0,
            debug_info: RefCell::new(String::new()),
            visible_messages: 0,
            total_lines: 0,
            visible_lines: 0,
            message_line_counts: Vec::new(),
            clipboard: ClipboardContext::new().expect("Failed to initialize clipboard"),
            ai_sender,
            current_game_response: None,
            last_user_message: None,
            backspace_counter: false,
            spinner: Spinner::new(),
            spinner_active: false,
            last_spinner_update: Instant::now(),
            current_save_name: Arc::new(RwLock::new(String::new())),
            last_known_character_sheet: None,
            is_recording: Arc::new(AtomicBool::new(false)),
        };

        (app, command_receiver)
    }

    pub fn update_cached_content(&mut self, max_width: usize) {
        let parsed_content = game::parse_game_content(self, max_width);
        self.cached_game_content = Some(Rc::new(parsed_content));
        self.cached_content_len = self.game_content.borrow().len();
    }

    pub async fn initialize_ai_client(&mut self) -> Result<(), AppError> {
        let api_key = self
            .settings
            .openai_api_key
            .as_ref()
            .ok_or(AppError::AIClientNotInitialized)?
            .clone();

        let ai_sender = self.ai_sender.clone();
        let debug_callback = move |message: String| {
            let _ = ai_sender.send(message::AIMessage::Debug(message));
        };

        self.ai_client = Some(GameAI::new(api_key, debug_callback).await?);

        Ok(())
    }

    pub fn process_message(&mut self, message: String) {
        let user_message = create_user_message(&self.settings.language, &message);
        let formatted_message = serde_json::to_string(&user_message).unwrap();

        self.start_spinner();

        let ai_client = self.ai_client.clone();
        let current_game = self.current_game.clone();
        let sender = self.command_sender.clone();

        tokio::spawn(async move {
            if let (Some(mut ai), Some(game_state)) = (ai_client, current_game) {
                let mut game_state = game_state.lock().await;
                let result = ai
                    .send_message(&formatted_message, &mut game_state)
                    .await
                    .map_err(AppError::Shadowrun);
                let _ = sender.send(AppCommand::AIResponse(result));
            } else {
                let _ = sender.send(AppCommand::AIResponse(Err(AppError::NoCurrentGame)));
            }
        });
    }

    pub async fn handle_ai_response(&mut self, result: Result<GameMessage, AppError>) {
        self.stop_spinner();
        self.add_debug_message(format!("Spinner: {:#?}", self.spinner_active));

        match result {
            Ok(game_message) => {
                self.add_debug_message(format!(
                    "Received game message from AI: {:#?}",
                    game_message
                ));

                let game_message_json = serde_json::to_string(&game_message).unwrap();
                self.add_debug_message(format!("Game message: {:#?}", game_message_json.clone()));
                self.add_message(Message::new(MessageType::Game, game_message_json.clone()));

                if self.settings.audio_output_enabled {
                    self.add_debug_message(format!(
                        "generating audio from {:#?}",
                        game_message.fluff.clone()
                    ));
                    if let Some(ai_client) = self.ai_client.clone() {
                        let mut game_message_clone = game_message.clone();
                        let save_name = match self.save_manager.current_save.clone() {
                            Some(game_state) => game_state.save_name,
                            None => "unknown".to_string(),
                        };
                        tokio::spawn(async move {
                            game_message_clone
                                .fluff
                                .speakers
                                .iter_mut()
                                .for_each(|speaker| speaker.assign_voice());

                            let mut audio_futures = FuturesOrdered::new();

                            for (index, fluff_line) in
                                game_message_clone.fluff.dialogue.iter_mut().enumerate()
                            {
                                let voice = game_message_clone
                                    .fluff
                                    .speakers
                                    .iter()
                                    .find(|s| s.index == fluff_line.speaker_index)
                                    .and_then(|s| s.voice.clone())
                                    .expect("Voice not found for speaker");

                                let ai_client = ai_client.clone();
                                let text = fluff_line.text.clone();
                                let save_name = save_name.clone();

                                // Generate the audio in parallel, keeping track of the index
                                audio_futures.push_back(async move {
                                    let result = audio::generate_audio(
                                        &ai_client.client,
                                        &save_name,
                                        &text,
                                        voice,
                                    )
                                    .await;
                                    (result, index)
                                });
                            }

                            // Process the results in order
                            while let Some((result, index)) = audio_futures.next().await {
                                if let Ok(path) = result {
                                    game_message_clone.fluff.dialogue[index].audio = Some(path);
                                }
                            }

                            // Play audio sequentially
                            for file in game_message_clone.fluff.dialogue.iter() {
                                if let Some(audio_path) = &file.audio {
                                    let _status = play_audio(audio_path.clone());
                                }
                            }
                        });
                    }
                }

                // Update the UI
                self.cached_game_content = None; // Force recalculation of cached content
                self.cached_content_len = 0;
                self.scroll_to_bottom();

                if let Some(character_sheet) = game_message.character_sheet {
                    self.add_debug_message("Updating character sheet".to_string());
                    self.update_character_sheet(character_sheet).await;
                }
                self.add_debug_message("Updated character sheet".to_string());

                if let Err(e) = self.save_current_game().await {
                    self.add_debug_message(format!("Failed to save game: {:#?}", e));
                    self.add_message(Message::new(
                        MessageType::System,
                        format!("Failed to save game after AI response: {:#?}", e),
                    ));
                }
                self.add_debug_message("saved game".to_string());
            }
            Err(e) => {
                self.add_debug_message(format!("Error: {:#?}", e));
                self.add_message(Message::new(
                    MessageType::System,
                    format!("AI Error: {:#?}", e),
                ));
            }
        }
    }

    fn handle_paste(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(contents) = self.clipboard.get_contents() {
            match self.state {
                AppState::InGame => {
                    for c in contents.chars() {
                        self.user_input.handle(InputRequest::InsertChar(c));
                    }
                }
                AppState::InputSaveName => {
                    for c in contents.chars() {
                        self.save_name_input.handle(InputRequest::InsertChar(c));
                    }
                }
                AppState::InputApiKey => {
                    for c in contents.chars() {
                        self.api_key_input.handle(InputRequest::InsertChar(c));
                    }
                }
                _ => {} // Other states don't have editable inputs
            }
        }
        Ok(())
    }

    pub fn handle_input(&mut self, key: KeyEvent) {
        match self.input_mode {
            InputMode::Normal => match self.state {
                AppState::MainMenu => self.handle_main_menu_input(key),
                AppState::InGame => self.handle_in_game_input(key),
                AppState::LoadMenu => self.handle_load_game_input(key),
                AppState::CreateImage => self.handle_create_image_input(key),
                AppState::SettingsMenu => self.handle_settings_input(key),
                AppState::InputApiKey => self.handle_api_key_input(key),
                AppState::InputSaveName => self.handle_save_name_input(key),
            },
            InputMode::Editing => match self.state {
                AppState::InGame => self.handle_in_game_editing(key),
                AppState::InputSaveName => self.handle_save_name_editing(key),
                AppState::InputApiKey => self.handle_api_key_editing(key),
                AppState::CreateImage => self.handle_create_image_editing(key),
                _ => {} // Other states don't have editing mode
            },
            InputMode::Recording => {
                match key.code {
                    KeyCode::Esc => {
                        self.stop_recording();
                    }
                    _ => {
                        // Ignore other keys during recording
                    }
                }
            }
        }
    }

    pub fn start_recording(&mut self) {
        self.is_recording.store(true, Ordering::SeqCst);
        audio::start_recording(&self.is_recording);
        self.input_mode = InputMode::Recording;
    }

    pub fn stop_recording(&mut self) {
        self.is_recording.store(false, Ordering::SeqCst);

        // Wait a bit to ensure the recording has stopped
        std::thread::sleep(Duration::from_millis(100));

        self.input_mode = InputMode::Normal;

        if self.ai_client.is_none() {
            self.add_message(Message::new(
                MessageType::System,
                "AI client not initialized. Cannot transcribe audio.".to_string(),
            ));
            self.add_debug_message("Transcription failed: AI client not initialized".to_string());
            return;
        }

        let ai_client = self.ai_client.clone();
        let state = self.state.clone();
        let sender = self.command_sender.clone();

        tokio::spawn(async move {
            if let Some(ai_client) = ai_client {
                match audio::transcribe_audio(&ai_client.client).await {
                    Ok(transcription) => {
                        let command = match state {
                            AppState::InGame => AppCommand::TranscriptionResult(
                                transcription,
                                TranscriptionTarget::UserInput,
                            ),
                            AppState::InputSaveName => AppCommand::TranscriptionResult(
                                transcription,
                                TranscriptionTarget::SaveNameInput,
                            ),
                            AppState::CreateImage => AppCommand::TranscriptionResult(
                                transcription,
                                TranscriptionTarget::ImagePrompt,
                            ),
                            _ => return,
                        };
                        let _ = sender.send(command);
                    }
                    Err(e) => {
                        let _ = sender.send(AppCommand::TranscriptionError(format!("{}", e)));
                    }
                }
            }
        });
    }

    pub async fn update_save_name(&self, new_name: String) {
        let mut save_name = self.current_save_name.write().await;
        *save_name = new_name;
    }

    fn handle_save_name_editing(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                // Handle save name submission
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Char('v') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    if let Err(e) = self.handle_paste() {
                        self.add_debug_message(format!("Failed to paste: {:#?}", e));
                    }
                } else {
                    self.save_name_input.handle_event(&Event::Key(key));
                }
            }
            _ => {
                self.save_name_input.handle_event(&Event::Key(key));
            }
        }
    }

    fn handle_save_name_input(&mut self, key: KeyEvent) {
        match self.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Char('e') => {
                    self.input_mode = InputMode::Editing;
                }
                KeyCode::Char('r') => {
                    self.start_recording();
                }
                KeyCode::Esc => {
                    self.state = AppState::MainMenu;
                    self.save_name_input.reset();
                }
                KeyCode::Enter => {
                    if !self.save_name_input.value().is_empty() {
                        self.game_content.borrow_mut().clear();
                        self.current_game = None;
                        if let Err(e) = self.command_sender.send(AppCommand::StartNewGame(
                            self.save_name_input.value().to_string(),
                        )) {
                            self.add_message(Message::new(
                                MessageType::System,
                                format!("Failed to send start new game command: {:#?}", e),
                            ));
                        }
                        self.save_name_input.reset();
                        self.state = AppState::InGame;
                    }
                }
                _ => {}
            },
            InputMode::Editing => match key.code {
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Char('v') => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        if let Err(e) = self.handle_paste() {
                            self.add_debug_message(format!("Failed to paste: {:#?}", e));
                        }
                    } else {
                        self.save_name_input.handle_event(&Event::Key(key));
                    }
                }
                _ => {
                    self.save_name_input.handle_event(&Event::Key(key));
                }
            },
            InputMode::Recording if key.code == KeyCode::Esc => {
                self.stop_recording();
            }
            _ => {}
        }
    }

    fn handle_api_key_editing(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                // Handle API key submission
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Char('v') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    if let Err(e) = self.handle_paste() {
                        self.add_debug_message(format!("Failed to paste: {:#?}", e));
                    }
                } else {
                    self.api_key_input.handle_event(&Event::Key(key));
                }
            }
            _ => {
                self.api_key_input.handle_event(&Event::Key(key));
            }
        }
    }

    fn handle_api_key_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                if !self.api_key_input.value().is_empty() {
                    let api_key = self.api_key_input.value().to_string();
                    self.settings.openai_api_key = Some(api_key.clone());

                    let sender = self.command_sender.clone();
                    tokio::spawn(async move {
                        let is_valid = Settings::validate_api_key(&api_key).await;
                        let _ = sender.send(AppCommand::ApiKeyValidationResult(is_valid));
                    });

                    self.state = AppState::SettingsMenu;
                }
            }
            KeyCode::Esc => {
                self.state = AppState::SettingsMenu;
            }
            KeyCode::Char('v') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    if let Err(e) = self.handle_paste() {
                        self.add_debug_message(format!("Failed to paste: {:#?}", e));
                    }
                } else {
                    self.api_key_input.handle_event(&Event::Key(key));
                }
            }
            _ => {
                self.api_key_input.handle_event(&Event::Key(key));
            }
        }
    }

    fn handle_in_game_editing(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Char('v') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    if let Err(e) = self.handle_paste() {
                        self.add_debug_message(format!("Failed to paste: {:#?}", e));
                    }
                } else {
                    self.user_input.handle_event(&Event::Key(key));
                }
            }
            _ => {
                // Let tui_input handle all other key events
                self.user_input.handle_event(&Event::Key(key));
            }
        }
    }

    fn handle_in_game_input(&mut self, key: KeyEvent) {
        match self.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Char('e') => {
                    self.input_mode = InputMode::Editing;
                }
                KeyCode::Char('r') => {
                    self.start_recording();
                }
                KeyCode::Esc if (self.highlighted_section != HighlightedSection::None) => {
                    self.highlighted_section = HighlightedSection::None;
                }
                KeyCode::Esc => {
                    self.game_content.borrow_mut().clear();
                    self.current_game = None;
                    self.last_known_character_sheet = None;
                    self.user_input.reset();
                    self.state = AppState::MainMenu;
                    self.save_manager.available_saves = SaveManager::scan_save_files();
                    self.add_message(Message::new(
                        MessageType::System,
                        "Game paused. Returned to main menu.".to_string(),
                    ))
                }
                KeyCode::Enter => {
                    if !self.user_input.value().is_empty() {
                        self.submit_user_input();
                    }
                }
                KeyCode::PageUp => {
                    for _ in 0..self.visible_lines {
                        self.scroll_up();
                    }
                }
                KeyCode::PageDown => {
                    for _ in 0..self.visible_lines {
                        self.scroll_down();
                    }
                }
                KeyCode::Up => self.scroll_up(),
                KeyCode::Down => self.scroll_down(),

                KeyCode::Tab => self.cycle_highlighted_section(),

                KeyCode::Home => {
                    self.game_content_scroll = 0;
                }
                KeyCode::End => {
                    self.game_content_scroll = self.total_lines.saturating_sub(self.visible_lines);
                }
                _ => {}
            },
            InputMode::Editing => match key.code {
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Enter => {
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Char('v') => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        if let Err(e) = self.handle_paste() {
                            self.add_debug_message(format!("Failed to paste: {:#?}", e));
                        }
                    } else {
                        self.user_input.handle_event(&Event::Key(key));
                    }
                }
                _ => {
                    self.user_input.handle_event(&Event::Key(key));
                }
            },
            InputMode::Recording => {
                match key.code {
                    KeyCode::Esc => {
                        self.stop_recording();
                    }
                    _ => {
                        // Ignore other keys during recording
                    }
                }
            }
        }
    }

    pub fn handle_api_key_validation_result(&mut self, is_valid: bool) {
        if !is_valid {
            self.settings.openai_api_key = None;
            self.add_message(Message::new(
                MessageType::System,
                "We could not validate your API Key. Please verify your key and internet connection and try again.".to_string(),
            ));
            self.openai_api_key_valid = false;
        } else {
            self.openai_api_key_valid = true;
            self.add_message(Message::new(
                MessageType::System,
                "API Key Validated, Thank you.".to_string(),
            ));
        }
        if let Err(e) = self.settings.save_to_file("./data/settings.json") {
            self.add_debug_message(format!("Failed to save settings: {:#?}", e));
        }
    }

    fn handle_settings_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => {
                self.settings_state.selected_setting =
                    (self.settings_state.selected_setting + 4) % 5; // Wrap around 5 settings
            }
            KeyCode::Down => {
                self.settings_state.selected_setting =
                    (self.settings_state.selected_setting + 1) % 5; // Wrap around 5 settings
            }
            KeyCode::Left => {
                let current_setting = self.settings_state.selected_setting;
                if current_setting == 0 {
                    // Language setting
                    let current_option = self.settings_state.selected_options[current_setting];
                    self.settings_state.selected_options[current_setting] =
                        (current_option + 2) % 3;
                } else if current_setting != 1 {
                    // Not API Key setting
                    self.settings_state.selected_options[current_setting] =
                        1 - self.settings_state.selected_options[current_setting];
                }
                self.apply_settings();
            }
            KeyCode::Right => {
                let current_setting = self.settings_state.selected_setting;
                if current_setting == 0 {
                    // Language setting
                    let current_option = self.settings_state.selected_options[current_setting];
                    self.settings_state.selected_options[current_setting] =
                        (current_option + 1) % 3;
                } else if current_setting != 1 {
                    // Not API Key setting
                    self.settings_state.selected_options[current_setting] =
                        1 - self.settings_state.selected_options[current_setting];
                }
                self.apply_settings();
            }
            KeyCode::Enter => {
                let current_setting = self.settings_state.selected_setting;
                if current_setting == 1 {
                    // API Key setting
                    self.state = AppState::InputApiKey;
                } else {
                    let current_option = self.settings_state.selected_options[current_setting];
                    let new_option = match current_setting {
                        0 => (current_option + 1) % 3, // Language (3 options)
                        2..=4 => 1 - current_option,   // Toggle settings (2 options)
                        _ => current_option,
                    };
                    self.settings_state.selected_options[current_setting] = new_option;
                    self.apply_settings();
                }
            }
            KeyCode::Esc => {
                self.state = AppState::MainMenu;
            }
            KeyCode::Char(c) => {
                if let Some(digit) = c.to_digit(10) {
                    if digit <= 5 {
                        self.settings_state.selected_setting = (digit - 1) as usize;
                        let current_setting = self.settings_state.selected_setting;
                        if current_setting == 1 {
                            // API Key setting
                            self.state = AppState::InputApiKey;
                        } else {
                            let current_option =
                                self.settings_state.selected_options[current_setting];
                            let new_option = match current_setting {
                                0 => (current_option + 1) % 3, // Language (3 options)
                                2..=5 => 1 - current_option,   // Toggle settings (2 options)
                                _ => current_option,
                            };
                            self.settings_state.selected_options[current_setting] = new_option;
                        }
                        self.apply_settings();
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_load_game_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                if let Some(selected) = self.load_game_menu_state.selected() {
                    if selected < self.save_manager.available_saves.len() {
                        if let Err(e) = self.command_sender.send(AppCommand::LoadGame(
                            self.save_manager.available_saves[selected].clone(),
                        )) {
                            self.add_message(Message::new(
                                MessageType::System,
                                format!("Failed to send load game command: {:#?}", e),
                            ));
                        } else {
                            // Add a message to indicate that the game is being loaded
                            self.add_message(Message::new(
                                MessageType::System,
                                "Loading game...".to_string(),
                            ));
                        }
                    }
                }
            }
            KeyCode::Esc => {
                self.state = AppState::MainMenu;
            }
            KeyCode::Up => {
                self.backspace_counter = false;
                self.navigate_load_game_menu(-1)
            }
            KeyCode::Down => {
                self.backspace_counter = false;
                self.navigate_load_game_menu(1)
            }
            KeyCode::Backspace => {
                if self.backspace_counter {
                    if !self.save_manager.available_saves.is_empty() {
                        let _ = self.delete_selected_save();
                    }
                    self.backspace_counter = false;
                } else {
                    self.backspace_counter = true;
                }
            }

            KeyCode::Char(c) => {
                if let Some(digit) = c.to_digit(10) {
                    let selected = ((digit as usize).saturating_sub(1))
                        % self.save_manager.available_saves.len();
                    self.load_game_menu_state.select(Some(selected));
                    let save_name = self.save_manager.available_saves[selected].clone();
                    if let Err(e) = self.command_sender.send(AppCommand::LoadGame(save_name)) {
                        self.add_message(Message::new(
                            MessageType::System,
                            format!("Failed to send load game command: {:#?}", e),
                        ));
                    } else {
                        self.add_message(Message::new(
                            MessageType::System,
                            "Loading game...".to_string(),
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_main_menu_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                match self.main_menu_state.selected() {
                    Some(0) => {
                        // Start New Game
                        self.state = AppState::InputSaveName;
                        self.save_name_input.reset(); // Clear any previous input
                    }
                    Some(1) => {
                        // Load Game
                        self.state = AppState::LoadMenu;
                        self.save_manager.available_saves = SaveManager::scan_save_files();
                        self.load_game_menu_state.select(Some(0));
                    }
                    Some(2) => {
                        if self.openai_api_key_valid {
                            self.state = AppState::CreateImage;
                        } else {
                            self.state = AppState::InputApiKey;
                        }
                    }
                    Some(3) => {
                        self.state = AppState::SettingsMenu;
                    }
                    _ => {}
                }
            }
            KeyCode::Up => self.navigate_main_menu(-1),
            KeyCode::Down => self.navigate_main_menu(1),
            KeyCode::Char(c) if ('1'..='4').contains(&c) => self.select_main_menu_by_char(c),
            KeyCode::Char('q') => {
                cleanup();
                std::process::exit(0);
            }
            _ => {}
        }
    }

    fn handle_create_image_input(&mut self, key: KeyEvent) {
        match self.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Char('e') => {
                    self.input_mode = InputMode::Editing;
                }
                KeyCode::Char('r') => {
                    self.start_recording();
                }
                KeyCode::Esc => self.state = AppState::MainMenu,
                KeyCode::Enter => {
                    let prompt = self.image_prompt.value().to_owned();

                    tokio::spawn(async move {
                        let _ = image::generate_and_save_image(&prompt).await;
                    });
                    self.add_message(Message::new(
                        MessageType::System,
                        "Generating image...".to_string(),
                    ));
                    self.image_prompt.reset();
                    self.state = AppState::MainMenu;
                }
                _ => {}
            },
            InputMode::Editing => match key.code {
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Char('v') => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        if let Err(e) = self.handle_paste() {
                            self.add_debug_message(format!("Failed to paste: {:#?}", e));
                        }
                    } else {
                        self.image_prompt.handle_event(&Event::Key(key));
                    }
                }
                _ => {
                    self.image_prompt.handle_event(&Event::Key(key));
                }
            },
            InputMode::Recording if key.code == KeyCode::Esc => {
                self.stop_recording();
            }
            _ => {}
        }
    }

    fn handle_create_image_editing(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                // Handle save name submission
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Char('v') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    if let Err(e) = self.handle_paste() {
                        self.add_debug_message(format!("Failed to paste: {:#?}", e));
                    }
                } else {
                    self.image_prompt.handle_event(&Event::Key(key));
                }
            }
            _ => {
                self.image_prompt.handle_event(&Event::Key(key));
            }
        }
    }

    fn cycle_highlighted_section(&mut self) {
        // This is a basic implementation. You might want to adjust this based on your layout.
        self.highlighted_section = match self.highlighted_section {
            HighlightedSection::None => HighlightedSection::Backstory,
            HighlightedSection::Backstory => {
                if let Some((name, _)) = self
                    .last_known_character_sheet
                    .as_ref()
                    .and_then(|sheet| sheet.inventory.iter().next())
                {
                    HighlightedSection::InventoryItem(name.clone())
                } else {
                    HighlightedSection::None
                }
            }
            HighlightedSection::InventoryItem(_) => {
                if let Some((name, _)) = self
                    .last_known_character_sheet
                    .as_ref()
                    .and_then(|sheet| sheet.contacts.iter().next())
                {
                    HighlightedSection::Contact(name.clone())
                } else {
                    HighlightedSection::None
                }
            }
            HighlightedSection::Contact(_) => HighlightedSection::None,
        };
    }

    fn submit_user_input(&mut self) {
        let input = self.user_input.value().trim().to_string();
        self.start_spinner();

        if input.is_empty() {
            return;
        }

        self.add_message(Message::new(MessageType::User, input.clone()));

        // Send a command to process the message
        if let Err(e) = self.command_sender.send(AppCommand::ProcessMessage(input)) {
            self.add_message(Message::new(
                MessageType::System,
                format!("Error sending message command: {:#?}", e),
            ));
        }

        // Clear the user input
        self.user_input = Input::default();
        self.scroll_to_bottom();
    }

    pub fn apply_settings(&mut self) {
        // Apply changes from settings_state to settings
        self.settings.language = match self.settings_state.selected_options[0] {
            0 => "English".to_string(),
            1 => "Français".to_string(),
            2 => "日本語".to_string(),
            _ => self.settings.language.clone(),
        };
        self.settings.audio_output_enabled = self.settings_state.selected_options[2] == 0;
        self.settings.audio_input_enabled = self.settings_state.selected_options[3] == 0;
        self.settings.debug_mode = self.settings_state.selected_options[4] == 1;

        // Save settings to file
        if let Err(e) = self.settings.save_to_file("./data/settings.json") {
            eprintln!("Failed to save settings: {:#?}", e);
        }
    }

    fn navigate_main_menu(&mut self, direction: isize) {
        let i = self.main_menu_state.selected().unwrap_or(0) as isize;
        let new_i = (i + direction).rem_euclid(4) as usize;
        self.main_menu_state.select(Some(new_i));
    }

    fn select_main_menu_option(&mut self) {
        match self.main_menu_state.selected() {
            Some(0) => {
                self.state = if self.openai_api_key_valid {
                    AppState::InputSaveName
                } else {
                    AppState::InputApiKey
                }
            }
            Some(1) => self.state = AppState::LoadMenu,
            Some(2) => {
                self.state = {
                    if self.openai_api_key_valid {
                        AppState::CreateImage
                    } else {
                        AppState::InputApiKey
                    }
                }
            }

            Some(3) => self.state = AppState::SettingsMenu,
            _ => {}
        }
    }

    fn select_main_menu_by_char(&mut self, c: char) {
        let index = (c as usize - 1) % 4;
        self.main_menu_state.select(Some(index));
        self.select_main_menu_option();
    }

    pub fn start_spinner(&mut self) {
        self.spinner_active = true;
        self.last_spinner_update = Instant::now();
    }

    pub fn stop_spinner(&mut self) {
        self.spinner_active = false;
    }

    pub fn update_spinner(&mut self) {
        if self.spinner_active && self.last_spinner_update.elapsed() >= Duration::from_millis(100) {
            self.spinner.next_frame();
            self.last_spinner_update = Instant::now();
        }
    }

    pub fn scroll_up(&mut self) {
        if self.game_content_scroll > 0 {
            self.game_content_scroll -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        if self.game_content_scroll < self.total_lines.saturating_sub(self.visible_lines) {
            self.game_content_scroll += 1;
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        // Recalculate total lines
        self.total_lines = self.calculate_total_lines();

        // Update the scroll position
        self.game_content_scroll = self.total_lines.saturating_sub(self.visible_lines);

        // Force UI update
        self.cached_game_content = None;
    }

    fn calculate_total_lines(&self) -> usize {
        self.game_content
            .borrow()
            .iter()
            .map(|message| {
                let wrapped_lines = textwrap::wrap(&message.content, self.visible_lines);
                wrapped_lines.len()
            })
            .sum()
    }

    pub fn update_scroll(&mut self) {
        let max_scroll = self.total_lines.saturating_sub(self.visible_lines);
        self.game_content_scroll = self.game_content_scroll.min(max_scroll);
    }

    pub fn add_debug_message(&self, message: String) {
        self.debug_info.borrow_mut().push_str(&message);
        self.debug_info.borrow_mut().push('\n');

        if !self.settings.debug_mode {
            return;
        }

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("sharad_debug.log")
        {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            let _ = writeln!(file, "[{}] {}", timestamp, &message);
        }
    }

    pub fn add_error(&mut self, error: ShadowrunError) {
        self.error_messages.push(ErrorMessage::new(error));
    }

    pub fn clean_old_errors(&mut self, max_age: Duration) {
        self.error_messages
            .retain(|msg| msg.timestamp.elapsed() < max_age);
    }

    pub fn update_debug_info(&mut self) {
        if !self.settings.debug_mode {
            return;
        }
        self.debug_info = format!(
            "Scroll: {}/{}, Visible Lines: {}, Total Lines: {}, Messages: {}",
            self.game_content_scroll,
            self.total_lines.saturating_sub(self.visible_lines),
            self.visible_lines,
            self.total_lines,
            self.game_content.borrow().len()
        )
        .into();
    }

    pub fn add_message(&self, message: message::Message) {
        self.game_content.borrow_mut().push(message);
    }

    pub async fn send_message(&mut self, message: String) -> Result<(), AppError> {
        let user_message = create_user_message(&self.settings.language, &message);
        let formatted_message = serde_json::to_string(&user_message)
            .map_err(|e| AppError::Shadowrun(ShadowrunError::Serialization(e.to_string())))?;

        self.start_spinner();

        let result: Result<GameMessage, AppError> = {
            if let (Some(ai), Some(game_state)) = (&mut self.ai_client, &self.current_game) {
                let mut game_state = game_state.lock().await;
                ai.send_message(&formatted_message, &mut game_state)
                    .await
                    .map_err(|e| AppError::Shadowrun(ShadowrunError::from(e)))
            } else if self.ai_client.is_none() {
                Err(AppError::AIClientNotInitialized)
            } else {
                Err(AppError::NoCurrentGame)
            }
        };

        self.stop_spinner();

        match &result {
            Ok(game_message) => {
                if let Err(e) = self
                    .ai_sender
                    .send(AIMessage::Response(Ok(game_message.clone())))
                {
                    eprintln!("Failed to send AI response: {}", e);
                }
                self.add_message(Message::new(
                    MessageType::Game,
                    serde_json::to_string(game_message).map_err(|e| {
                        AppError::Shadowrun(ShadowrunError::Serialization(e.to_string()))
                    })?,
                ));
                if let Some(character_sheet) = &game_message.character_sheet {
                    self.update_character_sheet(character_sheet.clone()).await;
                }
                Ok(())
            }
            Err(e) => {
                eprintln!("Failed to send AI error response {:?}", e);
                if let Err(send_err) = self.ai_sender.send(AIMessage::Response(Err(e.clone()))) {
                    eprintln!("Failed to send AI error through channel: {}", send_err);
                }
                Err(e.clone())
            }
        }
    }

    pub fn on_tick(&mut self) {
        if self.settings.debug_mode {
            self.update_debug_info();
        }
    }

    pub async fn start_new_game(
        &mut self,
        save_name: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Initialize AI client if not already initialized
        if self.ai_client.is_none() {
            self.initialize_ai_client().await?;
        }

        let client = self.ai_client.clone().unwrap().client;
        let assistant = match create_assistant(&client, &save_name).await {
            Ok(assistant) => assistant,
            Err(err) => {
                println!("{}", err);
                return Err(err);
            }
        };
        let assistant_id = &assistant.id;
        // let assistant_id = "asst_oavbUQD3KMkNKgyYRj42tKsM"; //Original
        // let assistant_id = "asst_4kaphuqlAkwnsbBrf482Z6dR"; //copy

        if let Some(ai) = &self.ai_client {
            // Start a new conversation
            ai.start_new_conversation(
                assistant_id,
                GameConversationState {
                    assistant_id: assistant_id.to_string(),
                    thread_id: String::new(),
                    character_sheet: None,
                },
            )
            .await?;

            // Get the thread_id from the conversation state
            let thread_id = ai
                .conversation_state
                .lock()
                .await
                .as_ref()
                .ok_or("Conversation state not initialized")?
                .thread_id
                .clone();

            // Create a new game state
            let new_game_state = Arc::new(Mutex::new(GameState {
                assistant_id: assistant_id.to_string(),
                thread_id,
                main_character_sheet: None,
                characters: Vec::new(),
                save_name: save_name.clone(),
            }));

            self.current_game = Some(new_game_state);

            // Save the game
            self.save_current_game().await?;

            self.state = AppState::InGame;
            self.add_message(message::Message::new(
                message::MessageType::System,
                format!("New game '{}' started!", save_name),
            ));

            // Start the spinner
            self.start_spinner();

            // Send initial message to start the game
            self.process_message(format!(
                "Start the game. Respond with the fluff in the following language: {}",
                self.settings.language
            ));

            Ok(())
        } else {
            Err("AI client not initialized".into())
        }
    }

    pub async fn update_character_sheet(&mut self, character_sheet: CharacterSheet) {
        if let Some(game_state) = &self.current_game {
            let mut game_state = game_state.lock().await;
            if let Some(ai) = &self.ai_client {
                if let Err(e) = ai.update_character_sheet(&mut game_state, character_sheet) {
                    self.add_message(Message::new(
                        MessageType::System,
                        format!("Failed to update character sheet: {:#?}", e),
                    ));
                } else {
                    self.add_debug_message("Character sheet updated successfully".to_string());
                }
            }
        }
    }

    pub async fn save_current_game(&mut self) -> Result<(), AppError> {
        let game_state = match &self.current_game {
            Some(arc_mutex) => arc_mutex,
            None => return Err(AppError::NoCurrentGame),
        };

        // Clone the Arc to get a new reference
        let game_state_clone = Arc::clone(game_state);

        // Clone the save_name to own the data
        let mut save_manager_clone = self.save_manager.clone();

        // Spawn a new task to handle the saving process
        tokio::spawn(async move {
            // Now we can safely lock the mutex without blocking the main thread
            let game_state = game_state_clone.lock().await;
            save_manager_clone.current_save = Some(game_state.clone());

            let _ = save_manager_clone.save();
        });

        // Update self.save_manager.current_save with the current game state
        let game_state = game_state.lock().await;
        self.save_manager.current_save = Some(game_state.clone());

        Ok(())
    }

    fn delete_selected_save(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(selected) = self.load_game_menu_state.selected() {
            let save_name = self.save_manager.available_saves[selected].clone();
            let ai_client = self.ai_client.clone().ok_or("AI client not found")?;
            let save_2 = save_name.clone();
            let assistant_id = get_assistant_id(&save_name)?;
            tokio::spawn(async move {
                delete_assistant(&ai_client.client, &assistant_id).await;
            });
            self.save_manager.available_saves.remove(selected);
            self.save_manager.clone().delete_save(&save_2)?;

            // Update the selected state to ensure it remains within bounds
            let new_selected = if selected >= self.save_manager.available_saves.len() {
                self.save_manager.available_saves.len().saturating_sub(1)
            } else {
                selected
            };
            self.load_game_menu_state.select(Some(new_selected));
            Ok(())
        } else {
            Err("No save selected".into())
        }
    }

    fn navigate_load_game_menu(&mut self, direction: isize) {
        let len = self.save_manager.available_saves.len();
        if len == 0 {
            return;
        }
        let current = self.load_game_menu_state.selected().unwrap_or(0);
        let next = if direction > 0 {
            (current + 1) % len
        } else {
            (current + len - 1) % len
        };
        self.load_game_menu_state.select(Some(next));
    }

    pub async fn load_game(&mut self, save_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.save_manager = self.save_manager.clone().load_from_file(save_name)?;

        let mut game_state = self
            .save_manager
            .current_save
            .clone()
            .ok_or("No current game")?;
        // Extract the save name from the path
        game_state.save_name = save_name.to_string();

        self.update_save_name(game_state.save_name.clone()).await;
        if self.ai_client.is_none() {
            self.initialize_ai_client().await?;
        }

        let conversation_state = GameConversationState {
            assistant_id: game_state.assistant_id.clone(),
            thread_id: game_state.thread_id.clone(),
            character_sheet: game_state.main_character_sheet.clone(),
        };

        // Clone the Arc to get a new reference to the AI client
        let ai_client = self.ai_client.as_mut().unwrap().borrow_mut();

        // Use the cloned Arc to call load_conversation
        ai_client.load_conversation(conversation_state).await;

        // Fetch all messages from the thread
        let all_messages = ai_client.fetch_all_messages(&game_state.thread_id).await?;

        // Load message history
        *self.game_content.borrow_mut() = all_messages;

        // Add a system message indicating the game was loaded
        self.add_message(message::Message::new(
            message::MessageType::System,
            format!("Game '{}' loaded successfully!", game_state.save_name),
        ));

        // Store the game state
        self.current_game = Some(Arc::new(Mutex::new(game_state)));

        self.state = AppState::InGame;

        // Calculate total lines after loading the game content
        self.total_lines = self.calculate_total_lines();
        // Scroll to the bottom after updating the scroll
        self.scroll_to_bottom();

        Ok(())
    }
}
