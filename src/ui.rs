use crate::app::{App, AppState, GameState}; // Importing necessary modules from the app crate

use ratatui::{
    // Importing various modules from the ratatui library
    backend::Backend, // Backend module for handling backend-specific functionality
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect}, // Layout modules for arranging UI components
    style::{Color, Modifier, Style}, // Style modules for customizing text and UI appearance
    text::{Line, Span},              // Text modules for handling lines and spans of text
    widgets::*,                      // Importing all widgets
    Frame,                           // Frame module for drawing the UI
};

// ASCII_ART constant remains unchanged

const ASCII_ART: &str = r#"


     ----------------------------------------------------------------------------- 
    |    _____   .                 A            .              .   .       .      |
    |    o o o\            .     _/_\_                                  |\        |
    |   ------\\      .       __//...\\__                .              ||\   .   |
    |   __ A . |\         .  <----------→     .                  .      ||||      |
    | HH|\. .|||                \\\|///                 ___|_           ||||      |
    | ||| | . \\\     A    .      |.|                  /|  .|    .      /||\      |
    |   | | .  |||   / \          |.|     .           | | ..|          /.||.\     |
    | ..| | . . \\\ ||**|         |.|   _A_     ___   | | ..|         || |\ .|    |
    | ..| | , ,  |||||**|         |.|  /| |   /|   |  |.| ..|         || |*|*|    |
    | ..|.| . . . \\\|**|.  ____  |.| | | |  | |***|  |.| ..|  _____  || |*|\|\   |
    | ..|.| . . .  |||**| /|.. .| |.| |*|*|  | |*  | ___| ..|/|  .  | || |*| |\\  |
    | -----------,. \\\*|| |.. .|//|\\|*|*_____| **||| ||  .| | ..  |/|| |*| |\\  |
    | Sharad game \  ||||| |..  // A \\*/| . ..| * ||| || ..| |  .  ||||,|*| | \  |
    |  By Roland  |\. \\\| |.. // /|\ \\ | . ..|** ||| || ..| | . . ||||.|*| |\\  |
    |   and the    \\  ||| |, ||.| | | ||| . ..| * ||| ||  .| | ..  ||||.|*| |||| |
     ----------------------------------------------------------------------------- 

  _____ _                         _
 / ____| |                       | |
| (___ | |__   __ _ _ __ __ _  __| |
 \___ \| '_ \ / _` | '__/ _` |/ _` |
 ____) | | | | (_| | | | (_| | (_| |
|_____/|_| |_|\__,_|_|  \__,_|\__,_|
"#;

pub fn draw(f: &mut Frame, app: &App) {
    // Main draw function that decides which screen to draw based on the app state
    match app.state {
        AppState::MainMenu => draw_main_menu(f, app), // Draw main menu
        AppState::InGame => draw_in_game(f, app),     // Draw in-game screen
        AppState::LoadGame => draw_load_game(f, app), // Draw load game screen
        AppState::CreateImage => draw_create_image(f, app), // Draw create image screen
        AppState::Settings => draw_settings(f, app),  // Draw settings screen
    }
}

fn draw_in_game(f: &mut Frame, app: &App) {
    // Placeholder for drawing the in-game screen
}

fn draw_load_game(f: &mut Frame, app: &App) {
    // Placeholder for drawing the load game screen
}

fn draw_create_image(f: &mut Frame, app: &App) {
    // Placeholder for drawing the create image screen
}

fn draw_main_menu(f: &mut Frame, app: &App) {
    // Function to draw the main menu
    let chunks = Layout::default()
        .direction(Direction::Vertical) // Arrange elements vertically
        .constraints(
            [
                Constraint::Length(30), // Fixed height for ASCII art
                Constraint::Min(10),    // Minimum height for menu
                Constraint::Length(3),  // Fixed height for status bar
            ]
            .as_ref(),
        )
        .split(f.size());

    // Render ASCII art
    let ascii_art = Paragraph::new(ASCII_ART)
        .style(Style::default().fg(Color::Green)) // Style the ASCII art with green color
        .alignment(Alignment::Center); // Center align the ASCII art
    f.render_widget(ascii_art, chunks[0]); // Render the ASCII art in the first chunk

    // Render menu
    render_menu(f, app, chunks[1]); // Render the menu in the second chunk

    // Render status bar
    let status = Paragraph::new("Press q to quit")
        .style(Style::default().fg(Color::LightCyan)) // Style the status bar text with light cyan color
        .block(Block::default().borders(Borders::ALL)) // Add borders to the status bar
        .alignment(Alignment::Center); // Center align the status bar text
    f.render_widget(status, chunks[2]); // Render the status bar in the third chunk
}

fn render_menu(f: &mut Frame, app: &App, area: Rect) {
    // Function to render the menu
    let menu_items = [
        "Start a new game", // First menu item
        "Load a game",      // Second menu item
        "Create an image",  // Third menu item
        "Settings",         // Fourth menu item
        "Exit",             // Fifth menu item
    ];

    let text: Vec<Line> = menu_items
        .iter()
        .enumerate()
        .map(|(i, &item)| {
            // Map each menu item to a line of text
            let number = if i == menu_items.len() - 1 {
                "0. ".to_string() // Format the last item number as 0
            } else {
                format!("{}. ", i + 1) // Format other item numbers starting from 1
            };
            let content = format!("{}", item); // Convert the item to a string
            if i == app.main_menu_state.selected().unwrap_or(0) {
                Line::from(vec![
                    Span::styled(number, Style::default().fg(Color::Yellow)), // Highlight the selected item number
                    Span::styled(
                        content,
                        Style::default()
                            .fg(Color::Yellow) // Highlight the selected item text
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![Span::raw(number), Span::raw(content)]) // Raw display for non-selected items
            }
        })
        .collect();

    let outer_block = Block::default()
        .borders(Borders::ALL) // Add borders to the outer block
        .title("Menu") // Title the outer block
        .style(Style::default().fg(Color::White)); // Style the outer block

    let menu_area = centered_rect(50, 40, area); // Create a centered rectangle for the menu

    // Render the outer block
    f.render_widget(outer_block, menu_area);

    // Create an inner area with margins
    let inner_area = Layout::default()
        .direction(Direction::Vertical) // Arrange elements vertically inside the menu
        .constraints([
            Constraint::Length(1), // Fixed height for top margin
            Constraint::Min(1),    // Minimum height for content
            Constraint::Length(1), // Fixed height for bottom margin
        ])
        .split(menu_area.inner(Margin {
            vertical: 0,    // No vertical margin
            horizontal: 27, // Horizontal margin of 27 units
        }))[1];

    let menu = Paragraph::new(text)
        .alignment(Alignment::Left) // Left align the menu text
        .style(Style::default().fg(Color::White)); // Style the menu text

    // Render the menu text in the inner area
    f.render_widget(menu, inner_area);
}

fn draw_settings(f: &mut Frame, app: &App) {
    // Function to draw the settings screen
    let chunks = Layout::default()
        .direction(Direction::Vertical) // Arrange elements vertically
        .constraints(
            [
                Constraint::Length(28), // Fixed height for ASCII art
                Constraint::Min(7),     // Minimum height for settings
                Constraint::Length(3),  // Fixed height for status bar
            ]
            .as_ref(),
        )
        .split(f.size());

    // Render ASCII art
    let ascii_art = Paragraph::new(ASCII_ART)
        .style(Style::default().fg(Color::Green)) // Style the ASCII art with green color
        .alignment(Alignment::Center); // Center align the ASCII art
    f.render_widget(ascii_art, chunks[0]); // Render the ASCII art in the first chunk

    render_settings(f, app, chunks[1]); // Render the settings content in the second chunk

    let status = Paragraph::new("Press Esc to return to main menu")
        .style(Style::default().fg(Color::LightCyan)) // Style the status bar text with light cyan color
        .block(Block::default().borders(Borders::ALL)) // Add borders to the status bar
        .alignment(Alignment::Center); // Center align the status bar text
    f.render_widget(status, chunks[2]); // Render the status bar in the third chunk
}

fn render_settings(f: &mut Frame, app: &App, area: Rect) {
    // Function to render the settings content
    let settings = [
        ("Language", vec!["English", "Français", "日本語"]), // Language setting with options
        ("API Key", vec!["Set", "Not Set"]),                 // API Key setting with options
        ("Voice Output", vec!["On", "Off"]),                 // Voice output setting with options
        ("Voice Input", vec!["On", "Off"]),                  // Voice input setting with options
        ("Debug Mode", vec!["Off", "On"]),                   // Debug mode setting with options
    ];

    let text: Vec<Line> = settings
        .iter()
        .enumerate()
        .map(|(i, (setting, options))| {
            // Map each setting to a line of text
            let mut spans = vec![
                Span::styled(format!("{}. ", i + 1), Style::default().fg(Color::Yellow)), // Highlight the setting number
                Span::styled(
                    format!("{:<15}", setting), // Format the setting name with fixed width
                    Style::default().fg(Color::White), // Style the setting name
                ),
            ];

            let selected_option = match i {
                0 => match app.settings.language.as_str() {
                    "en" => 0, // English option
                    "fr" => 1, // French option
                    "ja" => 2, // Japanese option
                    _ => 0,    // Default to English
                },
                1 => {
                    if app.settings.openai_api_key.is_empty() {
                        1 // API key not set
                    } else {
                        0 // API key set
                    }
                }
                2 => {
                    if app.settings.audio_output_enabled {
                        0 // Voice output enabled
                    } else {
                        1 // Voice output disabled
                    }
                }
                3 => {
                    if app.settings.audio_input_enabled {
                        0 // Voice input enabled
                    } else {
                        1 // Voice input disabled
                    }
                }
                4 => {
                    if app.settings.debug_mode {
                        1 // Debug mode enabled
                    } else {
                        0 // Debug mode disabled
                    }
                }
                _ => 0,
            };

            spans.extend(options.iter().enumerate().map(|(j, option)| {
                // Map each option to a span of text
                if j == selected_option {
                    Span::styled(
                        format!("[{}] ", option), // Highlight the selected option
                        Style::default()
                            .fg(Color::Green) // Green color for selected option
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled(format!("{} ", option), Style::default().fg(Color::Gray))
                    // Gray color for non-selected options
                }
            }));

            Line::from(spans) // Create a line from the spans
        })
        .collect();

    let outer_block = Block::default()
        .borders(Borders::ALL) // Add borders to the outer block
        .title("Settings") // Title the outer block
        .style(Style::default().fg(Color::White)); // Style the outer block

    let settings_area = centered_rect(50, 40, area); // Create a centered rectangle for the settings

    // Render the outer block
    f.render_widget(outer_block, settings_area);

    // Create an inner area with margins
    let inner_area = Layout::default()
        .direction(Direction::Vertical) // Arrange elements vertically inside the settings
        .constraints([
            Constraint::Length(1), // Fixed height for top margin
            Constraint::Min(1),    // Minimum height for content
            Constraint::Length(1), // Fixed height for bottom margin
        ])
        .split(settings_area.inner(Margin {
            vertical: 1,   // Vertical margin of 1 unit
            horizontal: 2, // Horizontal margin of 2 units
        }))[1];

    let settings_widget = Paragraph::new(text)
        .alignment(Alignment::Left) // Left align the settings text
        .style(Style::default().fg(Color::White)); // Style the settings text

    // Render the settings text in the inner area
    f.render_widget(settings_widget, inner_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // Function to create a centered rectangle
    let popup_layout = Layout::default()
        .direction(Direction::Vertical) // Arrange elements vertically
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2), // Top margin as a percentage of total height
            Constraint::Percentage(percent_y), // Content height as a percentage of total height
            Constraint::Percentage((100 - percent_y) / 2), // Bottom margin as a percentage of total height
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal) // Arrange elements horizontally
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2), // Left margin as a percentage of total width
            Constraint::Percentage(percent_x), // Content width as a percentage of total width
            Constraint::Percentage((100 - percent_x) / 2), // Right margin as a percentage of total width
        ])
        .split(popup_layout[1])[1] // Split and get the center rectangle
}

fn draw_game(f: &mut Frame, app: &mut App) {
    // Placeholder for drawing the game screen
}

fn draw_pause_menu(f: &mut Frame, app: &mut App) {
    // Placeholder for drawing the pause menu screen
}
