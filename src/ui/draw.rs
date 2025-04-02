// ui/draw.rs

use std::time::Duration;

use crate::{
    app::App,
    error::{ErrorMessage, ShadowrunError},
};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Clear, List, ListItem},
};

// Constants for minimum terminal size.
pub const MIN_WIDTH: u16 = 40;
pub const MIN_HEIGHT: u16 = 20;

pub fn center_rect(original_area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [horizontal_area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(original_area);
    let [vertical_area] = Layout::vertical([vertical])
        .flex(Flex::Center)
        .areas(horizontal_area);
    vertical_area
}

// pub fn draw(f: &mut Frame, app: &mut App) {
//     match app.state {
//         AppState::MainMenu => main_menu::draw_main_menu(f, app),
//         AppState::InGame => game::draw_in_game(f, app),
//         AppState::LoadMenu => load_menu::draw_load_game(f, app),
//         AppState::CreateImage => image_menu::draw_create_image(f, app),
//         AppState::SettingsMenu => settings_menu::draw_settings(f, app),
//         AppState::InputApiKey => api_key_input::draw_api_key_input(f, app),
//         AppState::InputSaveName => save_name_input::draw_save_name_input(f, app),
//     }
//     let area = f.area();
//
//     // Create a layout with space for error messages at the top
//     let chunks = Layout::default()
//         .direction(Direction::Vertical)
//         .constraints(
//             [
//                 Constraint::Length((app.error_messages.len() + 2) as u16),
//                 Constraint::Min(0),
//             ]
//             .as_ref(),
//         )
//         .split(area);
//
//     // Draw error messages
//     draw_error_messages(f, app, chunks[0]);
// }

fn draw_error_messages(f: &mut Frame, app: &App, area: Rect) {
    let max_age = Duration::from_secs(5);
    // Collect all error messages that are less than 5 seconds old
    let recent_error_messages: Vec<&ErrorMessage> = app
        .error_messages
        .iter()
        .filter(|error_message| error_message.timestamp.elapsed() < max_age)
        .collect();

    if !recent_error_messages.is_empty() {
        // Create a list of ListItem from recent error messages
        let error_items: Vec<ListItem> = recent_error_messages
            .iter()
            .map(|error_message| {
                let error_text = match &error_message.error {
                    ShadowrunError::Network(msg) => Span::styled(
                        format!("Network Error: {}", msg),
                        Style::default().fg(Color::Red),
                    ),
                    ShadowrunError::Audio(msg) => Span::styled(
                        format!("Audio Error: {}", msg),
                        Style::default().fg(Color::Red),
                    ),
                    ShadowrunError::IO(msg) => Span::styled(
                        format!("IO Error: {}", msg),
                        Style::default().fg(Color::Red),
                    ),
                    ShadowrunError::OpenAI(msg) => Span::styled(
                        format!("OpenAI Error: {}", msg),
                        Style::default().fg(Color::Red),
                    ),
                    ShadowrunError::Serialization(msg) => Span::styled(
                        format!("Serialization Error: {}", msg),
                        Style::default().fg(Color::Red),
                    ),
                    ShadowrunError::Unknown(msg) => Span::styled(
                        format!("Unknown Error: {}", msg),
                        Style::default().fg(Color::Red),
                    ),
                    ShadowrunError::Game(msg) => Span::styled(
                        format!("Game Error: {}", msg),
                        Style::default().fg(Color::Red),
                    ),
                    ShadowrunError::AI(msg) => Span::styled(
                        format!("AI Error: {}", msg),
                        Style::default().fg(Color::Red),
                    ),
                    ShadowrunError::Image(msg) => Span::styled(
                        format!("Image Error: {}", msg),
                        Style::default().fg(Color::Red),
                    ),
                };
                ListItem::new(error_text)
            })
            .collect();

        // Create a List widget to display all error messages
        let error_list = List::new(error_items).block(
            Block::default()
                .border_type(BorderType::Rounded)
                .borders(Borders::ALL)
                .title(" Error: ")
                .border_style(Style::default().fg(Color::Red)),
        );

        f.render_widget(Clear, area); // Clear the area behind the block
        f.render_widget(error_list, area);
    }
}
