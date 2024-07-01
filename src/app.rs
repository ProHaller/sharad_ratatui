use crate::ai::{AIError, GameAI, GameConversationState};
use crate::app_state::AppState;
use crate::cleanup::cleanup;
use crate::game_state::GameState;
use crate::message::{Message, MessageType};
use crate::settings::Settings;
use crate::settings_state::SettingsState;

use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::widgets::ListState;
use std::fs;
use std::path::Path;
use tokio::sync::mpsc;

pub enum AppCommand {
    LoadGame(String),
    StartNewGame,
}

pub struct App {
    pub should_quit: bool,
    pub state: AppState,
    pub main_menu_state: ListState,
    pub ai_client: Option<GameAI>,
    pub current_game: Option<GameState>,
    pub settings: Settings,
    pub settings_state: SettingsState,
    pub api_key_input: String,
    pub game_content: Vec<Message>,
    pub game_content_scroll: usize,
    pub user_input: String,
    pub cursor_position: usize,
    pub debug_info: String,
    pub visible_messages: usize,
    clipboard: ClipboardContext,
    message_sender: mpsc::UnboundedSender<String>,
    command_sender: mpsc::UnboundedSender<AppCommand>,
    pub load_game_menu_state: ListState,
    pub available_saves: Vec<String>,
}

impl App {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<AppCommand>) {
        let (message_sender, message_receiver) = mpsc::unbounded_channel();
        let (command_sender, command_receiver) = mpsc::unbounded_channel();
        let mut main_menu_state = ListState::default();
        main_menu_state.select(Some(0));

        let settings = Settings::load_from_file("settings.json").unwrap_or_default();
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
                settings_state,
                api_key_input: String::new(),
                load_game_menu_state,
                available_saves,
                game_content: Vec::new(),
                game_content_scroll: 0,
                user_input: String::new(),
                cursor_position: 0,
                debug_info: String::new(),
                visible_messages: 0,
                clipboard: ClipboardContext::new().expect("Failed to initialize clipboard"),
                message_sender,
                command_sender,
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
                        0 => (current_option + 1) % 3,   // Language (3 options)
                        2 | 3 | 4 => 1 - current_option, // Toggle settings (2 options)
                        _ => current_option,
                    };
                    self.settings_state.selected_options[current_setting] = new_option;
                    self.apply_settings();
                }
            }
            KeyCode::Esc => {
                self.state = AppState::MainMenu;
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
                        if let Err(e) = self.command_sender.send(AppCommand::StartNewGame) {
                            self.add_system_message(format!(
                                "Failed to send start new game command: {:?}",
                                e
                            ));
                        }
                    }
                    Some(1) => {
                        // Load Game
                        self.state = AppState::LoadGame;
                        self.available_saves = Self::scan_save_files();
                        self.load_game_menu_state.select(Some(0));
                    }
                    _ => {}
                }
            }
            KeyCode::Up => self.navigate_main_menu(-1),
            KeyCode::Down => self.navigate_main_menu(1),
            KeyCode::Char(c) if ('0'..='4').contains(&c) => self.select_main_menu_by_char(c),
            KeyCode::Esc => {
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
            Some(0) => self.state = AppState::InGame,
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
            }
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    // Add a new line in the TextArea
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
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_position < self.user_input.len() {
                    self.cursor_position += 1;
                }
            }
            KeyCode::Char(c) => {
                self.user_input.insert(self.cursor_position, c);
                self.cursor_position += 1;
            }
            _ => {}
        }
    }

    pub fn submit_user_input(&mut self) {
        let input = self.user_input.trim().to_string();
        if !input.is_empty() {
            self.add_user_message(input.clone());

            // Send the message through the channel
            if let Err(e) = self.message_sender.send(input) {
                self.add_system_message(format!("Error sending message: {:?}", e));
            }

            // Clear the user input
            self.user_input.clear();
            self.cursor_position = 0;
        }
    }

    pub fn update_scroll(&mut self) {
        if self.game_content.len() > self.visible_messages {
            self.game_content_scroll = self.game_content.len() - self.visible_messages;
        } else {
            self.game_content_scroll = 0;
        }
    }

    fn scroll_to_bottom(&mut self) {
        self.update_scroll();
    }

    pub fn add_user_message(&mut self, content: String) {
        self.game_content.push(Message {
            content,
            message_type: MessageType::User,
        });
        self.scroll_to_bottom();
    }

    pub fn add_game_message(&mut self, content: String) {
        self.game_content.push(Message {
            content,
            message_type: MessageType::Game,
        });
        self.scroll_to_bottom();
    }

    pub fn add_system_message(&mut self, content: String) {
        self.game_content.push(Message {
            content,
            message_type: MessageType::System,
        });
        self.scroll_to_bottom();
    }

    pub fn check_api_key(&mut self) {
        if self.settings.openai_api_key.is_none() {
            self.state = AppState::InputApiKey;
        }
    }

    // Add a new method to handle periodic updates
    pub fn on_tick(&mut self) {
        // Implement any logic that needs to run periodically
        // For example, you could update game state, process AI responses, etc.
    }
    #[allow(dead_code)]
    pub async fn start_new_conversation(&mut self, assistant_id: &str) -> Result<(), AIError> {
        if let Some(ai) = &mut self.ai_client {
            let initial_state = GameConversationState {
                assistant_id: assistant_id.to_string(),
                thread_id: String::new(), // This will be set by start_new_conversation
                player_health: 100,       // Set initial health
                player_gold: 0,           // Set initial gold
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

    #[allow(dead_code)]
    pub async fn send_message(&mut self, message: &str) -> Result<String, AIError> {
        if let Some(ai) = &mut self.ai_client {
            let response = ai.send_message(message).await?;
            Ok(response)
        } else {
            Err(AIError::ConversationNotInitialized)
        }
    }

    pub async fn start_new_game(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
                    player_health: 100,
                    player_gold: 0,
                },
            )
            .await?;
        } else {
            return Err("AI client not initialized".into());
        }

        self.state = AppState::InGame;
        self.add_system_message("New game started!".to_string());
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
                            self.add_system_message(format!(
                                "Failed to send load game command: {:?}",
                                e
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

        if let Some(ai) = &mut self.ai_client {
            ai.load_conversation(GameConversationState {
                assistant_id: game_state.assistant_id.clone(),
                thread_id: game_state.thread_id.clone(),
                player_health: 100,
                player_gold: 0,
            })
            .await;
        } else {
            return Err("AI client not initialized".into());
        }

        self.current_game = Some(game_state);
        self.state = AppState::InGame;
        self.add_system_message("Game loaded successfully!".to_string());
        Ok(())
    }

    fn handle_create_image_input(&mut self, key: KeyEvent) {
        // Implement image creation input handling
        cleanup();
        unimplemented!("handle_create_image_input");
    }

    fn handle_api_key_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                if !self.api_key_input.is_empty() {
                    self.settings.openai_api_key = Some(self.api_key_input.clone());
                    match GameAI::new(self.api_key_input.clone()) {
                        Ok(client) => {
                            self.ai_client = Some(client);
                            self.api_key_input.clear();
                            self.state = AppState::Settings;
                            if let Err(e) = self.settings.save_to_file("settings.json") {
                                eprintln!("Failed to save settings: {:?}", e);
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to initialize AI client: {:?}", e);
                            self.ai_client = None;
                        }
                    }
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
}
