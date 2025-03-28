// src/ui/main_menu.rs

// Import required modules and structs from other parts of the application or external crates.
use super::{
    constants::{ART, TITLE},
    draw::{MIN_HEIGHT, MIN_WIDTH, center_rect},
};
use crate::{
    app::{App, AppState},
    message::MessageType,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::*,
};

// Function to draw the main menu interface.
pub fn draw_main_menu(f: &mut Frame, app: &App) {
    let size = f.area();

    if size.width < MIN_WIDTH || size.height < MIN_HEIGHT {
        let warning = Paragraph::new("Terminal too small. Please resize.")
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center);
        f.render_widget(warning, size);
        return;
    }
    // Define layout constraints for different sections of the main menu.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Max(1),
                Constraint::Min(if size.height - 20 > 20 { 20 } else { 0 }),
                Constraint::Min(if size.height - 7 > 7 { 7 } else { 0 }),
                Constraint::Max(1),
                Constraint::Min(10),
                Constraint::Max(1),
            ]
            .as_ref(),
        )
        .split(f.area());

    // Render individual parts of the main menu using the layout defined above.
    render_header(f, chunks[0]);
    if (size.height - 20) > 20 {
        render_art(f, chunks[1]);
    }
    render_title(f, chunks[2]);
    render_console(f, app, chunks[3]);
    render_menu(f, app, chunks[4]);
    render_status(f, app, chunks[5]);
}

// Function to render the header section of the menu.
pub fn render_header(f: &mut Frame, area: Rect) {
    let header = Paragraph::new(format!("Sharad Ratatui v{}", env!("CARGO_PKG_VERSION")))
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().border_type(BorderType::Rounded))
        .alignment(Alignment::Center);
    f.render_widget(header, area);
}

// Function to render the art section of the menu.
pub fn render_art(f: &mut Frame, area: Rect) {
    let outer_block = Block::default()
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(outer_block, area);

    let inner_rect = center_rect(area, Constraint::Length(80), Constraint::Length(18));

    let inner_block = Block::default()
        .border_type(BorderType::Rounded)
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Green));
    f.render_widget(inner_block, inner_rect);

    let art = Paragraph::new(ART)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));
    f.render_widget(art, inner_rect);
}

// Function to render the title section of the menu.
pub fn render_title(f: &mut Frame, area: Rect) {
    let outer_block = Block::default()
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::DarkGray));
    let title_area = center_rect(area, Constraint::Length(38), Constraint::Length(8));
    f.render_widget(&outer_block, title_area);

    let title = Paragraph::new(TITLE)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));
    f.render_widget(title, title_area);
}

// Function to render the console section of the menu.
pub fn render_console(f: &mut Frame, app: &App, area: Rect) {
    let outer_block = Block::default()
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::DarkGray));
    let console_area = center_rect(area, Constraint::Percentage(90), Constraint::Length(2));
    f.render_widget(&outer_block, console_area);

    let console_message = app
        .game_content
        .borrow()
        .last()
        .filter(|content| content.message_type == MessageType::System)
        .map(|content| {
            Paragraph::new(content.content.to_string())
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Yellow))
        });

    if let Some(message) = console_message {
        f.render_widget(message, console_area);
    }
}

// Function to render the interactive menu section of the main menu.
pub fn render_menu(f: &mut Frame, app: &App, area: Rect) {
    // Define menu items to be displayed.
    let menu_items = [
        "Start a new game",
        "Load a game",
        "Create an image",
        "Settings",
    ];

    // Map menu items to text lines, applying different styles to the selected item.
    let menu_lines: Vec<Line> = menu_items
        .iter()
        .enumerate()
        .map(|(i, &item)| {
            let number = format!("{}. ", i + 1);
            let content = item;
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

    let max_width = menu_lines.iter().map(|l| l.width()).max().unwrap_or(0) as u16;
    let centered_area = center_rect(
        area,
        Constraint::Length(max_width),
        Constraint::Length(app.save_manager.available_saves.len() as u16 + 2),
    );

    let menu = Paragraph::new(menu_lines)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White));

    // HACK: This should be a stateful widget.
    f.render_widget(menu, centered_area);
}

// Function to render the status bar at the bottom of the menu.
pub fn render_status(f: &mut Frame, app: &App, area: Rect) {
    // Define the status message based on the current application state.
    let status_message = match app.state {
        AppState::MainMenu => "Press q to quit",
        AppState::LoadMenu => {
            "Press Enter or number to load save, Backspace twice to delete save, Esc to go back"
        }
        _ => "Press Esc to go back",
    };
    let status = Paragraph::new(status_message)
        .style(Style::default().fg(Color::DarkGray))
        .block(
            Block::default()
                .border_type(BorderType::Rounded)
                .borders(Borders::NONE),
        )
        .alignment(Alignment::Center);
    f.render_widget(status, area);
}

// Note: The proposed update to pass the 'app' to 'render_status' is already implemented in 'draw_main_menu'.
