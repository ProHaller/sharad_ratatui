use crate::app::{App, AppState, GameState};

use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::*,
    Frame,
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

pub fn draw<B: Backend>(f: &mut Frame, app: &App) {
    match app.state {
        AppState::MainMenu => draw_main_menu(f, app),
        AppState::InGame => draw_in_game(f, app),
        AppState::LoadGame => draw_load_game(f, app),
        AppState::CreateImage => draw_create_image(f, app),
        AppState::Settings => draw_settings(f, app),
    }
}

fn draw_main_menu<B: Backend>(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(30), // ASCII art height
                Constraint::Min(10),    // Menu
                Constraint::Length(3),  // Status bar
            ]
            .as_ref(),
        )
        .split(f.size());

    // Render ASCII art
    let ascii_art = Paragraph::new(ASCII_ART)
        .style(Style::default().fg(Color::Green))
        .alignment(Alignment::Center);
    f.render_widget(ascii_art, chunks[0]);

    // Render menu
    let menu_chunk = centered_rect(30, 40, chunks[1]);
    f.render_widget(Clear, menu_chunk);

    let menu_items = vec![
        "Start a new game",
        "Load a game",
        "Create an image",
        "Settings",
        "Exit",
    ];
    let items: Vec<ListItem> = menu_items.iter().map(|i| ListItem::new(*i)).collect();

    let menu = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Menu"))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(menu, menu_chunk, &mut app.main_menu_state.clone());

    // Render status bar
    let status = Paragraph::new("Press q to quit")
        .style(Style::default().fg(Color::LightCyan))
        .block(Block::default().borders(Borders::NONE))
        .alignment(Alignment::Center);
    f.render_widget(status, chunks[2]);
}

fn draw_in_game<B: Backend>(f: &mut Frame, app: &App) {
    // Implement in-game UI
}

fn draw_load_game<B: Backend>(f: &mut Frame, app: &App) {
    // Implement load game UI
}

fn draw_create_image<B: Backend>(f: &mut Frame, app: &App) {
    // Implement create image UI
}

fn draw_settings<B: Backend>(f: &mut Frame, app: &App) {
    // Implement settings UI
}

fn render_menu<B: Backend>(f: &mut Frame, app: &App, area: Rect) {
    let menu_items = [
        "Start a new game",
        "Load a game",
        "Create an image",
        "Settings",
        "Exit",
    ];

    let text: Vec<Line> = menu_items
        .iter()
        .enumerate()
        .map(|(i, &item)| {
            if i == app.main_menu_state.selected_index {
                Line::from(vec![
                    Span::styled(
                        "> ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        item,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(Span::raw(format!("  {}", item)))
            }
        })
        .collect();

    let menu = Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(ratatui::widgets::Borders::NONE));

    f.render_widget(menu, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_character_creation<B: Backend>(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.size());

    let title = Paragraph::new("Character Creation")
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let creation_area = centered_rect(60, 80, chunks[1]);
    f.render_widget(Clear, creation_area);

    let creation_text = vec![
        Line::from("Name: [Enter your character's name]"),
        Line::from(""),
        Line::from("Class:"),
        Line::from("1. Hacker"),
        Line::from("2. Street Samurai"),
        Line::from("3. Netrunner"),
        Line::from(""),
        Line::from("Use arrow keys to navigate, Enter to select"),
    ];

    let creation_widget = Paragraph::new(creation_text)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Create Your Character"),
        );
    f.render_widget(creation_widget, creation_area);

    let status = Paragraph::new("Press Esc to return to main menu")
        .style(Style::default().fg(Color::LightCyan))
        .alignment(Alignment::Center);
    f.render_widget(status, chunks[2]);
}

fn draw_game<B: Backend>(f: &mut Frame, app: &mut App) {
    // To be implemented
}

fn draw_pause_menu<B: Backend>(f: &mut Frame, app: &mut App) {
    // To be implemented
}
