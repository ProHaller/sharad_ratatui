use crate::ai::{AIError, GameAI, GameConversationState};
use crate::cleanup::cleanup;
use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, ListState, Paragraph};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use tokio::sync::mpsc;
use tui_textarea::TextArea;

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
    System,
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
    pub fn new() -> (Self, mpsc::UnboundedReceiver<String>) {
        let (message_sender, message_receiver) = mpsc::unbounded_channel();
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
                game_content: Vec::new(),
                game_content_scroll: 0,
                user_input: String::new(),
                cursor_position: 0,
                debug_info: String::new(),
                visible_messages: 0,
                clipboard: ClipboardContext::new().expect("Failed to initialize clipboard"),
                message_sender,
            },
            message_receiver,
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load() -> io::Result<Self> {
        // For now, just load from a file
        Self::load_from_file("settings.json")
    }

    pub fn save(&self) -> io::Result<()> {
        // For now, just save to a file
        self.save_to_file("settings.json")
    }
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

fn clear_textarea(textarea: &mut TextArea) {
    while !textarea.is_empty() {
        textarea.move_cursor(tui_textarea::CursorMove::Top);
        textarea.delete_line_by_end();
    }
}
