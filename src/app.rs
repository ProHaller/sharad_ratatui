use crate::ai::{AppError, GameAI, GameConversationState};
use crate::ai_response::{create_user_message, GameMessage, UserMessage};
use crate::app_state::AppState;
use crate::character::CharacterSheet;
use crate::cleanup::cleanup;
use crate::game_state::GameState;
use crate::message::{self, AIMessage, Message, MessageType};
use crate::settings::Settings;
use crate::settings_state::SettingsState;

use chrono::Local;
use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::widgets::ListState;
use ropey::Rope;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
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
    pub user_input: Rope,
    pub cursor_position: usize,
    pub debug_info: String,
    clipboard: ClipboardContext,
    command_sender: mpsc::UnboundedSender<AppCommand>,
    pub load_game_menu_state: ListState,
    pub available_saves: Vec<String>,
    ai_sender: mpsc::UnboundedSender<AIMessage>,
    pub visible_messages: usize,
    pub game_content: Vec<Message>,
    pub game_content_scroll: usize,
    pub visible_lines: usize,
    pub total_lines: usize,
    pub message_line_counts: Vec<usize>, // Store the number of lines for each message
    pub save_name_input: String,
    pub current_game_response: Option<GameMessage>,
    pub last_user_message: Option<UserMessage>,
    pub backspace_counter: bool,
}

impl App {
    pub async fn new(
        ai_sender: mpsc::UnboundedSender<AIMessage>,
    ) -> (Self, mpsc::UnboundedReceiver<AppCommand>) {
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        let mut main_menu_state = ListState::default();
        main_menu_state.select(Some(0));

        let settings = Settings::load_from_file("./data/settings.json").unwrap_or_default();
        let settings_state = SettingsState::from_settings(&settings);

        let mut load_game_menu_state = ListState::default();
        load_game_menu_state.select(Some(0));

        let available_saves = Self::scan_save_files();

        let openai_api_key_valid = if let Some(ref api_key) = settings.openai_api_key {
            Settings::validate_api_key(api_key).await
        } else {
            false
        };

        let app = Self {
            should_quit: false,
            state: AppState::MainMenu,
            main_menu_state,
            ai_client: None, // We'll initialize this later when needed
            current_game: None,
            settings,
            api_key_input: String::new(),
            settings_state,
            load_game_menu_state,
            openai_api_key_valid,
            save_name_input: String::new(),
            available_saves,
            game_content: Vec::new(),
            game_content_scroll: 0,
            user_input: Rope::new(),
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
            backspace_counter: false,
        };

        (app, command_receiver)
    }

    // Add this method to initialize the AI client when needed

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

        self.ai_client = Some(GameAI::new(api_key, debug_callback)?);

        Ok(())
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        match self.state {
            AppState::MainMenu => self.handle_main_menu_input(key),
            AppState::InGame => self.handle_in_game_input(key),
            AppState::LoadMenu => self.handle_load_game_input(key),
            AppState::CreateImage => self.handle_create_image_input(key),
            AppState::SettingsMenu => self.handle_settings_input(key),
            AppState::InputApiKey => self.handle_api_key_input(key),
            AppState::InputSaveName => self.handle_save_name_input(key),
        }
    }

    pub fn handle_api_key_input(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
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

                    self.state = AppState::SettingsMenu;
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
                self.state = AppState::SettingsMenu;
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
        if let Err(e) = self.settings.save_to_file("./data/settings.json") {
            self.add_debug_message(format!("Failed to save settings: {:?}", e));
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
                        self.state = AppState::LoadMenu;
                        self.available_saves = Self::scan_save_files();
                        self.load_game_menu_state.select(Some(0));
                    }
                    Some(2) => {
                        self.state = AppState::CreateImage;
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

    fn navigate_main_menu(&mut self, direction: isize) {
        let i = self.main_menu_state.selected().unwrap_or(0) as isize;
        let new_i = (i + direction).rem_euclid(4) as usize;
        self.main_menu_state.select(Some(new_i));
    }

    fn select_main_menu_option(&mut self) {
        match self.main_menu_state.selected() {
            Some(0) => self.state = AppState::InputSaveName,
            Some(1) => self.state = AppState::LoadMenu,
            Some(2) => self.state = AppState::CreateImage,
            Some(3) => self.state = AppState::SettingsMenu,
            _ => {}
        }
    }

    fn select_main_menu_by_char(&mut self, c: char) {
        let index = (c as usize - 1) % 4;
        self.main_menu_state.select(Some(index));
        self.select_main_menu_option();
    }

    pub fn handle_in_game_input(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        match key.code {
            KeyCode::Esc => {
                self.state = AppState::MainMenu;
                self.available_saves = Self::scan_save_files();
                self.add_message(Message::new(
                    MessageType::System,
                    "Game paused. Returned to main menu.".to_string(),
                ))
            }
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    // Add a new line in the input
                    self.user_input.insert(self.cursor_position, "\n");
                    self.cursor_position += 1;
                } else {
                    self.submit_user_input();
                }
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.user_input
                        .remove(self.cursor_position - 1..self.cursor_position);
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Delete => {
                if self.cursor_position < self.user_input.len_chars() {
                    self.user_input
                        .remove(self.cursor_position..self.cursor_position + 1);
                }
            }
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_position < self.user_input.len_chars() {
                    self.cursor_position += 1;
                }
            }
            KeyCode::Char(c) => {
                self.user_input.insert_char(self.cursor_position, c);
                self.cursor_position += 1;
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
        let input = self.user_input.to_string().trim().to_string();
        if !input.is_empty() {
            self.add_message(Message::new(MessageType::User, input.clone()));

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
            self.user_input = Rope::new();
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

    pub fn add_debug_message(&mut self, message: String) {
        if self.settings.debug_mode {
            self.debug_info = message.clone();
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open("sharad_debug.log")
            {
                let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
                let _ = writeln!(file, "[{}] {}", timestamp, message);
            }
        }
    }

    pub fn update_debug_info(&mut self) {
        if self.settings.debug_mode {
            self.debug_info = format!(
                "Scroll: {}/{}, Visible Lines: {}, Total Lines: {}, Messages: {}",
                self.game_content_scroll,
                self.total_lines.saturating_sub(self.visible_lines),
                self.visible_lines,
                self.total_lines,
                self.game_content.len()
            );
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.game_content.push(message.clone());
        self.add_debug_message(format!("pushed message to game_content: {:?}", message));
        self.total_lines = self
            .game_content
            .iter()
            .map(|message| {
                let wrapped_lines = textwrap::wrap(&message.content, self.visible_lines);
                wrapped_lines.len()
            })
            .sum();
        self.update_scroll();
    }

    pub async fn send_message(&mut self, message: String) -> Result<(), AppError> {
        let user_message = create_user_message(&message);
        let formatted_message = serde_json::to_string(&user_message)?;

        match (&mut self.ai_client, &mut self.current_game) {
            (Some(ai), Some(game_state)) => {
                let game_message = ai.send_message(&formatted_message, game_state).await?;

                self.add_debug_message(format!(
                    "Received game message from AI: {:?}",
                    game_message
                ));

                let game_message_json = serde_json::to_string(&game_message)?;
                self.add_message(Message::new(MessageType::Game, game_message_json));

                if let Some(character_sheet) = game_message.character_sheet {
                    self.update_character_sheet(character_sheet);
                }

                self.save_current_game()?;

                Ok(())
            }
            (None, _) => Err(AppError::AIClientNotInitialized),
            (_, None) => Err(AppError::NoCurrentGame),
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
        if self.ai_client.is_none() {
            self.initialize_ai_client().await?;
        }

        let assistant_id = "asst_4kaphuqlAkwnsbBrf482Z6dR"; // Set your assistant_id here

        if let Some(ai) = &mut self.ai_client {
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
            let thread_id = ai.conversation_state.as_ref().unwrap().thread_id.clone();

            // Create a new GameState
            let new_game_state = GameState {
                assistant_id: assistant_id.to_string(),
                thread_id,
                character_sheet: None,
                characters: Vec::new(),
                message_history: Vec::new(),
                save_name: save_name.clone(),
            };

            // Set the current_game
            self.current_game = Some(new_game_state);

            // Now save the initial state
            self.save_game(&save_name)?;

            self.state = AppState::InGame;
            self.add_message(Message::new(
                MessageType::System,
                format!("New game '{}' started!", save_name),
            ));

            self.send_message("Start the game. When necessary, please create a character sheet by calling the `create_character_sheet` function with the necessary details.".to_string())
            .await?;

            Ok(())
        } else {
            Err("AI client not initialized".into())
        }
    }

    pub fn update_character_sheet(&mut self, mut character_sheet: CharacterSheet) {
        if let Some(game_state) = &mut self.current_game {
            // Update the main character sheet
            game_state.character_sheet = Some(character_sheet.clone());

            // Update the character in the characters vector
            if let Some(existing_character) = game_state
                .characters
                .iter_mut()
                .find(|c| c.name == character_sheet.name)
            {
                *existing_character = character_sheet;
            } else {
                game_state.characters.push(character_sheet);
            }

            // Save the updated game state
            if let Err(e) = self.save_current_game() {
                self.add_message(Message::new(
                    MessageType::System,
                    format!("Failed to save game after character sheet update: {:?}", e),
                ));
            } else {
                self.add_debug_message("Game saved after character sheet update".to_string());
            }
        } else {
            self.add_debug_message("No current game state to update character sheet".to_string());
        }
    }

    pub fn save_current_game(&mut self) -> Result<(), AppError> {
        if let Some(game_state) = &self.current_game {
            let save_name = &game_state.save_name;
            self.save_game(save_name)?;
            self.add_debug_message(format!("Game saved with name: {}", save_name));
            Ok(())
        } else {
            self.add_debug_message("No current game to save".to_string());
            Err(AppError::NoCurrentGame)
        }
    }

    pub fn save_game(&self, save_name: &str) -> Result<(), AppError> {
        if let Some(game_state) = &self.current_game {
            let save_dir = "./data/save";
            if !Path::new(save_dir).exists() {
                fs::create_dir_all(save_dir).map_err(AppError::IO)?;
            }

            let save_path = format!("{}/{}.json", save_dir, save_name);
            fs::File::create(&save_path).map_err(AppError::IO)?;
            game_state
                .save_to_file(&save_path)
                .map_err(|e| AppError::IO(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
            Ok(())
        } else {
            Err(AppError::NoCurrentGame)
        }
    }

    // Update the handle_ai_response method

    pub fn handle_ai_response(&mut self, response: String) {
        self.add_debug_message(format!("Received AI response: {}", response));

        // Remove the "AI is thinking..." message if it exists
        if let Some(last_message) = self.game_content.last() {
            if last_message.content == "AI is thinking..."
                && last_message.message_type == MessageType::System
            {
                self.game_content.pop();
            }
        }

        // Attempt to parse the AI response as a GameMessage
        match serde_json::from_str::<GameMessage>(&response) {
            Ok(game_message) => {
                // Add the reasoning and narration separately
                self.add_message(Message::new(
                    MessageType::Game,
                    format!("Reasoning: {}", game_message.reasoning),
                ));
                self.add_message(Message::new(
                    MessageType::Game,
                    format!("Narration: {}", game_message.narration),
                ));

                // Add debug message for AI reasoning if debug mode is on
                if self.settings.debug_mode {
                    self.add_debug_message(format!("AI Reasoning: {}", game_message.reasoning));
                }

                // Update character sheet if present
                if let Some(character_sheet) = game_message.character_sheet {
                    self.update_character_sheet(character_sheet);
                }

                // Save the game after processing the response
                if let Err(e) = self.save_current_game() {
                    self.add_message(Message::new(
                        MessageType::System,
                        format!("Failed to save game after AI response: {:?}", e),
                    ));
                }
            }
            Err(e) => {
                // If parsing fails, display the raw response
                self.add_message(Message::new(
                    MessageType::System,
                    format!(
                        "Failed to parse AI response: {}. Raw response: {}",
                        e, response
                    ),
                ));
            }
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
                    if !self.available_saves.is_empty() {
                        self.delete_save();
                    }
                    self.backspace_counter = false;
                } else {
                    self.backspace_counter = true;
                }
            }

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

    fn delete_save(&mut self) {
        if let Some(selected) = self.load_game_menu_state.selected() {
            let save_path = format!("./data/save/{}", self.available_saves[selected]);
            match fs::remove_file(save_path) {
                Ok(_) => {
                    self.add_message(Message::new(
                        MessageType::System,
                        format!(
                            "Successfully deleted save file: {}",
                            self.available_saves[selected]
                        ),
                    ));
                    self.available_saves.remove(selected);

                    // Update the selected state to ensure it remains within bounds
                    let new_selected = if selected >= self.available_saves.len() {
                        self.available_saves.len().saturating_sub(1)
                    } else {
                        selected
                    };
                    self.load_game_menu_state.select(Some(new_selected));
                }
                Err(e) => {
                    self.add_message(Message::new(
                        MessageType::System,
                        format!("Failed to delete save file: {:?}", e),
                    ));
                }
            }
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
        let mut game_state = GameState::load_from_file(path)?;
        self.add_debug_message(format!("Game state loade: {:?}", game_state));

        // Extract the save name from the path
        let save_name = Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        game_state.save_name = save_name;

        if self.ai_client.is_none() {
            self.initialize_ai_client().await?;
        }

        let ai = self.ai_client.as_mut().ok_or("AI client not initialized")?;

        ai.load_conversation(GameConversationState {
            assistant_id: game_state.assistant_id.clone(),
            thread_id: game_state.thread_id.clone(),
            character_sheet: game_state.character_sheet.clone(),
        })
        .await;

        // Fetch all messages from the thread
        let all_messages = ai.fetch_all_messages(&game_state.thread_id).await?;

        // Load message history
        self.game_content = all_messages;

        // Add a system message indicating the game was loaded
        self.add_message(Message::new(
            MessageType::System,
            format!("Game '{}' loaded successfully!", game_state.save_name),
        ));

        // Store the game state
        self.current_game = Some(game_state);

        self.state = AppState::InGame;

        // Calculate total lines after loading the game content
        self.total_lines = self
            .game_content
            .iter()
            .map(|message| {
                let wrapped_lines = textwrap::wrap(&message.content, self.visible_lines);
                wrapped_lines.len()
            })
            .sum();

        // Scroll to the bottom after updating the scroll
        self.scroll_to_bottom();

        Ok(())
    }

    pub fn scroll_to_bottom(&mut self) {
        self.game_content_scroll = self.total_lines.saturating_sub(self.visible_lines);
        self.update_scroll();
    }

    fn handle_save_name_input(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        match key.code {
            KeyCode::Enter => {
                if !self.save_name_input.is_empty() {
                    // Clear the game content
                    self.game_content.clear();
                    self.current_game = None;
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

// TODO: Double display of user message
// TODO: double display of User message
// TODO: Character sheet refreshment
// TODO: L+
