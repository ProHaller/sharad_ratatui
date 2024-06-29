use crate::app::{App, AppState, GameState};

use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
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

pub fn draw(f: &mut Frame, app: &App) {
    match app.state {
        AppState::MainMenu => draw_main_menu(f, app),
        AppState::InGame => draw_in_game(f, app),
        AppState::LoadGame => draw_load_game(f, app),
        AppState::CreateImage => draw_create_image(f, app),
        AppState::Settings => draw_settings(f, app),
    }
}

fn draw_in_game(f: &mut Frame, app: &App) {
    // Your code here
}

fn draw_load_game(f: &mut Frame, app: &App) {
    // Your code here
}

fn draw_create_image(f: &mut Frame, app: &App) {
    // Your code here
}

fn draw_settings(f: &mut Frame, app: &App) {
    // Your code here
}

fn draw_main_menu(f: &mut Frame, app: &App) {
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
    render_menu(f, app, chunks[1]);

    // Render status bar
    let status = Paragraph::new("Press q to quit")
        .style(Style::default().fg(Color::LightCyan))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
    f.render_widget(status, chunks[2]);
}

fn render_menu(f: &mut Frame, app: &App, area: Rect) {
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
            let number = if i == menu_items.len() - 1 {
                "0. ".to_string()
            } else {
                format!("{}. ", i + 1)
            };
            let content = format!("{}", item);
            if i == app.main_menu_state.selected().unwrap_or(0) {
                Line::from(vec![
                    Span::styled(number, Style::default().fg(Color::Yellow)),
                    Span::styled(
                        content,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![Span::raw(number), Span::raw(content)])
            }
        })
        .collect();

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .title("Menu")
        .style(Style::default().fg(Color::White));

    let menu_area = centered_rect(50, 40, area);

    // Render the outer block
    f.render_widget(outer_block, menu_area);

    // Create an inner area with margins
    let inner_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top margin
            Constraint::Min(1),    // Content
            Constraint::Length(1), // Bottom margin
        ])
        .split(menu_area.inner(Margin {
            vertical: 0,
            horizontal: 30,
        }))[1];

    let menu = Paragraph::new(text)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White));

    // Render the menu text in the inner area
    f.render_widget(menu, inner_area);
}

fn draw_character_creation(f: &mut Frame, app: &mut App) {
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

fn draw_game(f: &mut Frame, app: &mut App) {
    // To be implemented
}

fn draw_pause_menu(f: &mut Frame, app: &mut App) {
    // To be implemented
}
