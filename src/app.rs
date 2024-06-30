use crate::ai::ConversationState;
use crate::ai::AI;
use crate::cleanup::cleanup;
use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::widgets::ListState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};

#[derive(PartialEq)]
pub enum AppState {
    MainMenu,
    InGame,
    LoadGame,
    CreateImage,
    Settings,
    InputApiKey,
}

#[derive(Serialize, Deserialize)]
pub struct GameState {
    pub assistant_id: String,
    pub thread_id: String,
    pub player_health: u8,
    pub player_gold: u32,
}

#[derive(Clone, PartialEq)]
pub enum MessageType {
    User,
    Game,
}

#[derive(Clone)]
pub struct Message {
    pub content: String,
    pub message_type: MessageType,
}

pub struct App {
    pub should_quit: bool,
    pub state: AppState,
    pub main_menu_state: ListState,
    pub ai_client: Option<AI>,
    pub current_game: Option<GameState>,
    pub settings: Settings,
    pub settings_state: SettingsState,
    pub api_key_input: String,
    pub game_content: Vec<Message>,
    pub game_content_scroll: usize,
    pub user_input: String,
    pub cursor_position: usize,
    pub debug_info: String,
    clipboard: ClipboardContext,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    pub language: String,
    pub openai_api_key: Option<String>,
    pub audio_output_enabled: bool,
    pub audio_input_enabled: bool,
    pub debug_mode: bool,
}

#[derive(Serialize, Deserialize)]
pub struct SettingsState {
    pub selected_setting: usize,
    pub selected_options: Vec<usize>,
}

impl SettingsState {
    pub fn from_settings(settings: &Settings) -> Self {
        SettingsState {
            selected_setting: 0,
            selected_options: vec![
                match settings.language.as_str() {
                    "English" => 0,
                    "Français" => 1,
                    "日本語" => 2,
                    _ => 0,
                },
                0, // API Key (always 0 as it's not a toggle)
                if settings.audio_output_enabled { 0 } else { 1 },
                if settings.audio_input_enabled { 0 } else { 1 },
                if settings.debug_mode { 1 } else { 0 },
            ],
        }
    }
}

impl App {
    pub fn new() -> Self {
        let mut main_menu_state = ListState::default();
        main_menu_state.select(Some(0));

        let settings = Settings::load_from_file("settings.json").unwrap_or_default();
        let settings_state = SettingsState::from_settings(&settings);

        let ai_client = if let Some(api_key) = &settings.openai_api_key {
            match AI::new(api_key.clone()) {
                Ok(client) => Some(client),
                Err(e) => {
                    eprintln!("Failed to initialize AI client: {:?}", e);
                    None
                }
            }
        } else {
            None
        };

        Self {
            should_quit: false,
            state: AppState::MainMenu,
            main_menu_state,
            ai_client,
            current_game: None,
            settings,
            settings_state,
            api_key_input: String::new(),
            game_content: Vec::new(),
            game_content_scroll: 0,
            user_input: String::new(),
            cursor_position: 0,
            debug_info: String::new(),
            clipboard: ClipboardContext::new().expect("Failed to initialize clipboard"),
        }
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
            KeyCode::Up => self.navigate_main_menu(-1),
            KeyCode::Down => self.navigate_main_menu(1),
            KeyCode::Enter => self.select_main_menu_option(),
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
        match key.code {
            KeyCode::Up => {
                if self.game_content_scroll > 0 {
                    self.game_content_scroll -= 1;
                }
            }
            KeyCode::Down => {
                if self.game_content_scroll < self.game_content.len().saturating_sub(1) {
                    self.game_content_scroll += 1;
                }
            }
            KeyCode::Enter => {
                self.submit_user_input();
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'v' {
                    // Handle paste
                    if let Ok(contents) = self.clipboard.get_contents() {
                        self.user_input.insert_str(self.cursor_position, &contents);
                        self.cursor_position += contents.len();
                    }
                } else {
                    self.user_input.insert(self.cursor_position, c);
                    self.cursor_position += 1;
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
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_position < self.user_input.len() {
                    self.cursor_position += 1;
                }
            }
            KeyCode::Home => {
                self.cursor_position = 0;
            }
            KeyCode::End => {
                self.cursor_position = self.user_input.len();
            }
            KeyCode::Esc => {
                self.state = AppState::MainMenu;
            }
            _ => {}
        }
    }

    pub fn submit_user_input(&mut self) {
        if !self.user_input.trim().is_empty() {
            // Add user input to game content
            self.game_content.push(Message {
                content: self.user_input.clone(),
                message_type: MessageType::User,
            });

            // Here, you would typically send the user input to the AI and get a response
            // For now, we'll just add a placeholder response
            self.game_content.push(Message {
                content: "AI response placeholder".to_string(),
                message_type: MessageType::Game,
            });

            self.user_input.clear();
            self.cursor_position = 0;

            // Automatically scroll to the bottom
            self.game_content_scroll = self.game_content.len().saturating_sub(1);
        }
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
    pub async fn start_new_conversation(
        &mut self,
        assistant_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ai) = &mut self.ai_client {
            ai.start_new_conversation(assistant_id).await?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn continue_conversation(
        &mut self,
        conversation_state: ConversationState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ai) = &mut self.ai_client {
            ai.continue_conversation(conversation_state).await;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn send_message(&self, message: &str) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(ai) = &self.ai_client {
            let response = ai.send_message(message).await?;
            Ok(response)
        } else {
            Err("AI client not initialized".into())
        }
    }

    fn handle_load_game_input(&mut self, key: KeyEvent) {
        // Implement load game input handling
        cleanup();
        unimplemented!("handle_load_game_input");
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
                    match AI::new(self.api_key_input.clone()) {
                        Ok(client) => self.ai_client = Some(client),
                        Err(e) => {
                            eprintln!("Failed to initialize AI client: {:?}", e);
                            self.ai_client = None;
                        }
                    }
                    self.api_key_input.clear();
                    self.state = AppState::Settings;
                    self.settings
                        .save_to_file("settings.json")
                        .unwrap_or_else(|e| {
                            eprintln!("Failed to save settings: {:?}", e);
                        });
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

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: "English".to_string(),
            openai_api_key: None,
            audio_output_enabled: true,
            audio_input_enabled: true,
            debug_mode: false,
        }
    }
}

impl Settings {
    pub fn load_from_file(path: &str) -> io::Result<Self> {
        let data = fs::read_to_string(path)?;
        let settings = serde_json::from_str(&data)?;
        Ok(settings)
    }

    pub fn save_to_file(&self, path: &str) -> io::Result<()> {
        let data = serde_json::to_string_pretty(self)?;
        let mut file = fs::File::create(path)?;
        file.write_all(data.as_bytes())?;
        Ok(())
    }
}
