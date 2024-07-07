// ui/draw.rs

use crate::app::App;
use crate::app_state::AppState;

use ratatui::Frame;

use super::{api_key_input, create_image, game, load_game, main_menu, save_name_input, settings};

pub fn draw(f: &mut Frame, app: &mut App) {
    match app.state {
        AppState::MainMenu => main_menu::draw_main_menu(f, app),
        AppState::InGame => game::draw_in_game(f, app),
        AppState::LoadMenu => load_game::draw_load_game(f, app),
        AppState::CreateImage => create_image::draw_create_image(f, app),
        AppState::SettingsMenu => settings::draw_settings(f, app),
        AppState::InputApiKey => api_key_input::draw_api_key_input(f, app),
        AppState::InputSaveName => save_name_input::draw_save_name_input(f, app),
    }
}
