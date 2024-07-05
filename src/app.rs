use crate::ai::{AIError, GameAI, GameConversationState};
use crate::ai_response::{create_user_message, GameMessage, UserMessage};
use crate::app_state::AppState;
use crate::cleanup::cleanup;
use crate::game_state::GameState;
use crate::message::{Message, MessageType};
use crate::settings::Settings;
use crate::settings_state::SettingsState;

use async_openai::{config::OpenAIConfig, Client};
use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::widgets::ListState;
use std::fs;
use std::path::Path;
use tokio::sync::mpsc;

pub enum AppCommand {
    LoadGame(String),
    StartNewGame(String),
    ProcessMessage(String),
    ApiKeyValidationResult(bool),
}

pub struct App {
    pub should_quit: bool,
    pub state: AppState,
    pub main_menu_state: ListState,
    pub ai_client: Option<GameAI>,
    pub current_game: Option<GameState>,
    pub settings: Settings,
    pub api_key_input: String,
    pub openai_api_key_valid: bool,
    pub settings_state: SettingsState,
    pub user_input: String,
    pub cursor_position: usize,
    pub debug_info: String,
    clipboard: ClipboardContext,
    command_sender: mpsc::UnboundedSender<AppCommand>,
    pub load_game_menu_state: ListState,
    pub available_saves: Vec<String>,
    ai_sender: mpsc::UnboundedSender<String>,
    pub visible_messages: usize,
    pub game_content: Vec<Message>,
    pub game_content_scroll: usize,
    pub visible_lines: usize,
    pub total_lines: usize,
    pub message_line_counts: Vec<usize>, // Store the number of lines for each message
    pub save_name_input: String,
    pub current_game_response: Option<GameMessage>,
    pub last_user_message: Option<UserMessage>,
}

impl App {
    pub async fn new(
        ai_sender: mpsc::UnboundedSender<String>,
    ) -> (Self, mpsc::UnboundedReceiver<AppCommand>) {
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        let mut main_menu_state = ListState::default();
        main_menu_state.select(Some(0));

        let mut settings = Settings::load_from_file("settings.json").unwrap_or_default();
        let openai_api_key_valid = if let Some(ref api_key) = settings.openai_api_key {
            Settings::validate_api_key(api_key).await
        } else {
            false
        };

        if !openai_api_key_valid {
            settings.openai_api_key = None;
            settings.save_to_file("settings.json").ok();
        }

        let settings_state = SettingsState::from_settings(&settings);

        let ai_client = if let Some(api_key) = &settings.openai_api_key {
            match GameAI::new(api_key.clone()) {
                Ok(client) => Some(client),
                Err(e) => {
                    eprintln!("Failed to initialize AI client: {:?}", e);
                    None
                }
            }
        } else {
            None
        };

        let mut load_game_menu_state = ListState::default();
        load_game_menu_state.select(Some(0));

        let available_saves = Self::scan_save_files();
        (
            Self {
                should_quit: false,
                state: AppState::MainMenu,
                main_menu_state,
                ai_client,
                current_game: None,
                settings,
                api_key_input: String::new(),
                openai_api_key_valid,
                settings_state,
                load_game_menu_state,
                save_name_input: String::new(),
                available_saves,
                game_content: Vec::new(),
                game_content_scroll: 0,
                user_input: String::new(),
                cursor_position: 0,
                debug_info: String::new(),
                visible_messages: 0,
                total_lines: 0,
                visible_lines: 0,
                message_line_counts: Vec::new(),
                clipboard: ClipboardContext::new().expect("Failed to initialize clipboard"),
                command_sender,
                ai_sender,
                current_game_response: None,
                last_user_message: None,
            },
            command_receiver,
        )
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        match self.state {
            AppState::MainMenu => self.handle_main_menu_input(key),
            AppState::InGame => self.handle_in_game_input(key),
            AppState::LoadGame => self.handle_load_game_input(key),
            AppState::CreateImage => self.handle_create_image_input(key),
            AppState::Settings => self.handle_settings_input(key),
            AppState::InputApiKey => self.handle_api_key_input(key),
            AppState::InputSaveName => self.handle_save_name_input(key),
        }
    }

    pub fn handle_api_key_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                if !self.api_key_input.is_empty() {
                    let api_key = self.api_key_input.clone();
                    self.settings.openai_api_key = Some(api_key.clone());
                    self.api_key_input.clear();

                    let sender = self.command_sender.clone();
                    tokio::spawn(async move {
                        let is_valid = Settings::validate_api_key(&api_key).await;
                        let _ = sender.send(AppCommand::ApiKeyValidationResult(is_valid));
                    });

                    self.state = AppState::Settings;
                }
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'v' {
                    // Handle paste
                    if let Ok(contents) = self.clipboard.get_contents() {
                        self.api_key_input.push_str(&contents);
                    }
                } else {
                    self.api_key_input.push(c);
                }
            }
            KeyCode::Backspace => {
                self.api_key_input.pop();
            }
            KeyCode::Esc => {
                self.api_key_input.clear();
                self.state = AppState::Settings;
            }
            _ => {}
        }
    }

    pub fn handle_api_key_validation_result(&mut self, is_valid: bool) {
        if !is_valid {
            self.settings.openai_api_key = None;
            self.add_message(Message::new(
                MessageType::System,
                "Invalid API key entered. Please try again.".to_string(),
            ));
        } else {
            self.openai_api_key_valid = true;
        }
        if let Err(e) = self.settings.save_to_file("settings.json") {
            eprintln!("Failed to save settings: {:?}", e);
        }
    }

    fn handle_settings_input(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
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
                    if digit < 5 {
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
                                2..=4 => 1 - current_option,   // Toggle settings (2 options)
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
        if let Err(e) = self.settings.save_to_file("settings.json") {
            eprintln!("Failed to save settings: {:?}", e);
        }
    }

    fn handle_main_menu_input(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        match key.code {
            KeyCode::Enter => {
                match self.main_menu_state.selected() {
                    Some(0) => {
                        // Start New Game
                        self.state = AppState::InputSaveName;
                        self.save_name_input.clear(); // Clear any previous input
                    }
                    Some(1) => {
                        // Load Game
                        self.state = AppState::LoadGame;
                        self.available_saves = Self::scan_save_files();
                        self.load_game_menu_state.select(Some(0));
                    }
                    Some(2) => {
                        self.state = AppState::CreateImage;
                    }
                    Some(3) => {
                        self.state = AppState::Settings;
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

    fn navigate_main_menu(&mut self, direction: isize) {
        let i = self.main_menu_state.selected().unwrap_or(0) as isize;
        let new_i = (i + direction).rem_euclid(4) as usize;
        self.main_menu_state.select(Some(new_i));
    }

    fn select_main_menu_option(&mut self) {
        match self.main_menu_state.selected() {
            Some(0) => self.state = AppState::InputSaveName,
            Some(1) => self.state = AppState::LoadGame,
            Some(2) => self.state = AppState::CreateImage,
            Some(3) => self.state = AppState::Settings,
            _ => {}
        }
    }

    fn select_main_menu_by_char(&mut self, c: char) {
        let index = (c as usize - 1) % 4;
        self.main_menu_state.select(Some(index));
        self.select_main_menu_option();
    }

    fn handle_in_game_input(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        match key.code {
            KeyCode::Esc => {
                self.state = AppState::MainMenu;
                self.add_message(Message::new(
                    MessageType::System,
                    "Game paused. Returned to main menu.".to_string(),
                ))
            }
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    // Add a new line in the input
                    self.user_input.insert(self.cursor_position, '\n');
                    self.cursor_position += 1;
                } else {
                    // Submit the user input
                    self.submit_user_input();
                }
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.user_input.remove(self.cursor_position - 1);
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Delete => {
                if self.cursor_position < self.user_input.len() {
                    self.user_input.remove(self.cursor_position);
                }
            }
            KeyCode::Left => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    // Move cursor to the start of the previous word
                    while self.cursor_position > 0
                        && !self
                            .user_input
                            .chars()
                            .nth(self.cursor_position - 1)
                            .unwrap()
                            .is_alphanumeric()
                    {
                        self.cursor_position -= 1;
                    }
                    while self.cursor_position > 0
                        && self
                            .user_input
                            .chars()
                            .nth(self.cursor_position - 1)
                            .unwrap()
                            .is_alphanumeric()
                    {
                        self.cursor_position -= 1;
                    }
                } else if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Right => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    // Move cursor to the start of the next word
                    while self.cursor_position < self.user_input.len()
                        && self
                            .user_input
                            .chars()
                            .nth(self.cursor_position)
                            .unwrap()
                            .is_alphanumeric()
                    {
                        self.cursor_position += 1;
                    }
                    while self.cursor_position < self.user_input.len()
                        && !self
                            .user_input
                            .chars()
                            .nth(self.cursor_position)
                            .unwrap()
                            .is_alphanumeric()
                    {
                        self.cursor_position += 1;
                    }
                } else if self.cursor_position < self.user_input.len() {
                    self.cursor_position += 1;
                }
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match c {
                        'a' => self.cursor_position = 0,
                        'e' => self.cursor_position = self.user_input.len(),
                        'k' => self.user_input.truncate(self.cursor_position),
                        'u' => {
                            self.user_input = self.user_input.split_off(self.cursor_position);
                            self.cursor_position = 0;
                        }
                        _ => {}
                    }
                } else {
                    self.user_input.insert(self.cursor_position, c);
                    self.cursor_position += 1;
                }
            }
            KeyCode::PageUp => {
                for _ in 0..self.visible_lines {
                    self.scroll_up();
                }
            }
            KeyCode::Up => self.scroll_up(),
            KeyCode::PageDown => {
                for _ in 0..self.visible_lines {
                    self.scroll_down();
                }
            }
            KeyCode::Down => self.scroll_down(),
            KeyCode::Home => {
                self.game_content_scroll = 0;
            }
            KeyCode::End => {
                self.game_content_scroll = self.total_lines.saturating_sub(self.visible_lines);
            }
            _ => {}
        }
    }

    pub fn submit_user_input(&mut self) {
        let input = self.user_input.trim().to_string();
        if !input.is_empty() {
            self.add_message(Message::new(MessageType::System, input.clone()));

            // Send a command to process the message
            if let Err(e) = self.command_sender.send(AppCommand::ProcessMessage(input)) {
                self.add_message(Message::new(
                    MessageType::System,
                    format!("Error sending message command: {:?}", e),
                ));
            } else {
                // Add a "thinking" message to indicate that the AI is processing
                self.add_message(Message::new(
                    MessageType::System,
                    "AI is thinking...".to_string(),
                ));
            }

            // Clear the user input
            self.user_input.clear();
            self.cursor_position = 0;
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

    pub fn update_scroll(&mut self) {
        let max_scroll = self.total_lines.saturating_sub(self.visible_lines);
        self.game_content_scroll = self.game_content_scroll.min(max_scroll);
    }

    pub fn update_debug_info(&mut self) {
        self.debug_info = format!(
            "Scroll: {}/{}, Visible Lines: {}, Total Lines: {}, Messages: {}",
            self.game_content_scroll,
            self.total_lines.saturating_sub(self.visible_lines),
            self.visible_lines,
            self.total_lines,
            self.game_content.len()
        );
    }

    pub fn add_message(&mut self, message: Message) {
        self.game_content.push(message);
        self.scroll_to_bottom();
    }

    pub async fn send_message(
        &mut self,
        message: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let user_message = create_user_message(&message);
        let formatted_message = serde_json::to_string(&user_message)?;

        self.add_message(Message::new(MessageType::User, formatted_message.clone()));

        if let Some(ai) = &mut self.ai_client {
            match ai.send_message(&formatted_message).await {
                Ok(response) => {
                    self.add_message(Message::new(MessageType::Game, response));
                    Ok(())
                }
                Err(e) => {
                    let error_msg = format!("Error from AI: {:?}", e);
                    self.add_message(Message::new(MessageType::System, error_msg.clone()));
                    Err(error_msg.into())
                }
            }
        } else {
            let error_msg = "AI client not initialized".to_string();
            self.add_message(Message::new(MessageType::System, error_msg.clone()));
            Err(error_msg.into())
        }
    }

    pub fn handle_ai_response(&mut self, response: String) {
        if let Ok(game_message) = serde_json::from_str::<GameMessage>(&response) {
            self.current_game_response = Some(game_message);
            self.add_message(Message::new(MessageType::Game, response));
        } else {
            self.add_message(Message::new(
                MessageType::System,
                format!("Failed to parse AI response: {}", response),
            ));
        }
    }

    fn scroll_to_bottom(&mut self) {
        self.game_content_scroll = self.game_content.len().saturating_sub(self.visible_lines);
        self.update_scroll();
    }

    pub async fn check_openai_api_key(&mut self) {
        if let Some(api_key) = &self.settings.openai_api_key {
            let client = Client::with_config(OpenAIConfig::new().with_api_key(api_key));
            let is_valid = client.models().list().await.is_ok();

            if !is_valid {
                self.settings.openai_api_key = None;
                if let Err(e) = self.settings.save_to_file("settings.json") {
                    eprintln!(
                        "Failed to save settings after removing invalid API key: {:?}",
                        e
                    );
                }
            }
        }
    }

    // Add a new method to handle periodic updates
    pub fn on_tick(&mut self) {}

    #[allow(dead_code)]
    pub async fn start_new_conversation(&mut self, assistant_id: &str) -> Result<(), AIError> {
        if let Some(ai) = &mut self.ai_client {
            let initial_state = GameConversationState {
                assistant_id: assistant_id.to_string(),
                thread_id: String::new(),
            };
            ai.start_new_conversation(assistant_id, initial_state)
                .await?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn continue_conversation(
        &mut self,
        conversation_state: GameConversationState,
    ) -> Result<(), AIError> {
        if let Some(ai) = &mut self.ai_client {
            ai.load_conversation(conversation_state).await;
        }
        Ok(())
    }

    // Update this method or create a new one to handle incoming AI responses

    pub async fn start_new_game(
        &mut self,
        save_name: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.ai_client.is_none() {
            self.initialize_ai_client().await?;
        }

        if let Some(ai) = &mut self.ai_client {
            let assistant_id = "asst_4kaphuqlAkwnsbBrf482Z6dR"; // Set your assistant_id here
            ai.start_new_conversation(
                assistant_id,
                GameConversationState {
                    assistant_id: assistant_id.to_string(),
                    thread_id: String::new(),
                },
            )
            .await?;

            // Save the game immediately after starting
            self.save_game(&save_name)?;
        } else {
            return Err("AI client not initialized".into());
        }

        self.state = AppState::InGame;
        self.add_message(Message::new(
            MessageType::System,
            format!("New game '{}' started!", save_name),
        ));

        // Send an initial message to the AI to start the game
        self.send_message("Start the game in json".to_string())
            .await?;

        Ok(())
    }

    pub fn save_game(&self, save_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let game_state = GameState {
            assistant_id: self
                .ai_client
                .as_ref()
                .unwrap()
                .conversation_state
                .as_ref()
                .unwrap()
                .assistant_id
                .clone(),
            thread_id: self
                .ai_client
                .as_ref()
                .unwrap()
                .conversation_state
                .as_ref()
                .unwrap()
                .thread_id
                .clone(),
            message_history: self.game_content.clone(),
        };

        let save_path = format!("./data/save/{}.json", save_name);
        game_state.save_to_file(&save_path)?;
        Ok(())
    }

    async fn initialize_ai_client(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(api_key) = &self.settings.openai_api_key {
            self.ai_client = Some(GameAI::new(api_key.clone())?);
            Ok(())
        } else {
            Err("OpenAI API key not set".into())
        }
    }

    pub fn scan_save_files() -> Vec<String> {
        let save_dir = Path::new("./data/save");
        if !save_dir.exists() {
            return Vec::new();
        }

        fs::read_dir(save_dir)
            .unwrap()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.is_file() && path.extension()? == "json" {
                    path.file_name()?.to_str().map(String::from)
                } else {
                    None
                }
            })
            .collect()
    }

    fn handle_load_game_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                if let Some(selected) = self.load_game_menu_state.selected() {
                    if selected < self.available_saves.len() {
                        let save_path = format!("./data/save/{}", self.available_saves[selected]);
                        if let Err(e) = self.command_sender.send(AppCommand::LoadGame(save_path)) {
                            self.add_message(Message::new(
                                MessageType::System,
                                format!("Failed to send load game command: {:?}", e),
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
            KeyCode::Up => self.navigate_load_game_menu(-1),
            KeyCode::Down => self.navigate_load_game_menu(1),

            KeyCode::Char(c) => {
                if let Some(digit) = c.to_digit(10) {
                    let selected = (digit as usize - 1) % self.available_saves.len();
                    self.load_game_menu_state.select(Some(selected));
                    let save_path = format!("./data/save/{}", self.available_saves[selected]);
                    if let Err(e) = self.command_sender.send(AppCommand::LoadGame(save_path)) {
                        self.add_message(Message::new(
                            MessageType::System,
                            format!("Failed to send load game command: {:?}", e),
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

    fn navigate_load_game_menu(&mut self, direction: isize) {
        let len = self.available_saves.len();
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

    pub async fn load_game(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let game_state = GameState::load_from_file(path)?;

        if self.ai_client.is_none() {
            self.initialize_ai_client().await?;
        }

        let ai = self.ai_client.as_mut().ok_or("AI client not initialized")?;

        ai.load_conversation(GameConversationState {
            assistant_id: game_state.assistant_id.clone(),
            thread_id: game_state.thread_id.clone(),
        })
        .await;

        // Fetch all messages from the thread
        let all_messages = ai.fetch_all_messages(&game_state.thread_id).await?;

        // Load message history
        self.game_content = all_messages;

        // Add a system message indicating the game was loaded
        self.add_message(Message::new(
            MessageType::System,
            "Game loaded successfully!".to_string(),
        ));

        // Store the game state
        self.current_game = Some(game_state);

        self.state = AppState::InGame;
        self.update_scroll(); // This will set the scroll position to show the most recent messages
        Ok(())
    }

    fn handle_save_name_input(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        match key.code {
            KeyCode::Enter => {
                if !self.save_name_input.is_empty() {
                    // Start a new game with the given save name
                    if let Err(e) = self
                        .command_sender
                        .send(AppCommand::StartNewGame(self.save_name_input.clone()))
                    {
                        self.add_message(Message::new(
                            MessageType::System,
                            format!("Failed to send start new game command: {:?}", e),
                        ));
                    }
                    self.save_name_input.clear();
                    self.state = AppState::InGame;
                }
            }
            KeyCode::Char(c) => {
                self.save_name_input.push(c);
            }
            KeyCode::Backspace => {
                self.save_name_input.pop();
            }
            KeyCode::Esc => {
                self.save_name_input.clear();
                self.state = AppState::MainMenu;
            }
            _ => {}
        }
    }
    fn handle_create_image_input(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        match key.code {
            KeyCode::Esc => {
                self.state = AppState::MainMenu;
            }
            _ => {}
        }
    }
}
