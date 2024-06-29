use async_openai::Client;
use crossterm::event::KeyEvent;
use ratatui::widgets::ListState;
use serde::{Deserialize, Serialize};

pub enum AppState {
    MainMenu,
    InGame,
    LoadGame,
    CreateImage,
    Settings,
}

#[derive(Serialize, Deserialize)]
pub struct GameState {
    pub assistant_id: String,
    pub thread_id: String,
    // Add other game state fields as needed
}

pub struct App {
    pub should_quit: bool,
    pub state: AppState,
    pub main_menu_state: ListState,
    pub openai_client: Client,
    pub current_game: Option<GameState>,
    pub settings: Settings,
}

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub language: String,
    pub openai_api_key: String,
    pub audio_output_enabled: bool,
    pub audio_input_enabled: bool,
    pub debug_mode: bool,
}

impl App {
    pub fn new() -> Self {
        let mut main_menu_state = ListState::default();
        main_menu_state.select(Some(0));

        Self {
            should_quit: false,
            state: AppState::MainMenu,
            main_menu_state,
            openai_client: Client::new(),
            current_game: None,
            settings: Settings::default(),
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        match self.state {
            AppState::MainMenu => self.handle_main_menu_input(key),
            AppState::InGame => self.handle_in_game_input(key),
            AppState::LoadGame => self.handle_load_game_input(key),
            AppState::CreateImage => self.handle_create_image_input(key),
            AppState::Settings => self.handle_settings_input(key),
        }
    }

    fn handle_main_menu_input(&mut self, key: KeyEvent) {
        match key.code {
            crossterm::event::KeyCode::Up => {
                let i = self.main_menu_state.selected().unwrap_or(0);
                self.main_menu_state
                    .select(Some(if i == 0 { 4 } else { i - 1 }));
            }
            crossterm::event::KeyCode::Down => {
                let i = self.main_menu_state.selected().unwrap_or(0);
                self.main_menu_state.select(Some((i + 1) % 5));
            }
            crossterm::event::KeyCode::Enter => {
                match self.main_menu_state.selected() {
                    Some(0) => self.state = AppState::InGame, // Start new game
                    Some(1) => self.state = AppState::LoadGame,
                    Some(2) => self.state = AppState::CreateImage,
                    Some(3) => self.state = AppState::Settings,
                    Some(4) => self.should_quit = true, // Exit
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn handle_in_game_input(&mut self, key: KeyEvent) {
        // Implement in-game input handling
        unimplemented!("handle_in_game_input");
    }

    fn handle_load_game_input(&mut self, key: KeyEvent) {
        // Implement load game input handling
        unimplemented!("handle_load_game_input");
    }

    fn handle_create_image_input(&mut self, key: KeyEvent) {
        // Implement image creation input handling
        unimplemented!("handle_create_image_input");
    }

    fn handle_settings_input(&mut self, key: KeyEvent) {
        // Implement settings input handling
        unimplemented!("handle_settings_input");
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: "English".to_string(),
            openai_api_key: String::new(),
            audio_output_enabled: true,
            audio_input_enabled: true,
            debug_mode: false,
        }
    }
}
