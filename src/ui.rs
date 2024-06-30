use crate::app::{App, AppState, MessageType};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::Margin,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::*,
    Frame,
};

// ASCII_ART constant remains unchanged

const ART: &str = r#"
    _____   .                 A            .              .   .       .      
    o o o\            .     _/_\_                                  |\        
   ------\\      .       __//...\\__                .              ||\   .   
   __ A . |\         .  <----------→     .                  .      ||||      
 HH|\. .|||                \\\|///                 ___|_           ||||      
 ||| | . \\\     A    .      |.|                  /|  .|    .      /||\      
   | | .  |||   / \          |.|     .           | | ..|          /.||.\     
 ..| | . . \\\ ||**|         |.|   _A_     ___   | | ..|         || |\ .|    
 ..| | , ,  |||||**|         |.|  /| |   /|   |  |.| ..|         || |*|*|    
 ..|.| . . . \\\|**|.  ____  |.| | | |  | |***|  |.| ..|  _____  || |*|\|\   
 ..|.| . . .  |||**| /|.. .| |.| |*|*|  | |*  | ___| ..|/|  .  | || |*| |\\  
 -----------,. \\\*|| |.. .|//|\\|*|*_____| **||| ||  .| | ..  |/|| |*| |\\  
 Sharad game \  ||||| |..  // A \\*/| . ..| * ||| || ..| |  .  ||||,|*| | \  
  By Roland  |\. \\\| |.. // /|\ \\ | . ..|** ||| || ..| | . . ||||.|*| |\\  
   and the    \\  ||| |, ||.| | | ||| . ..| * ||| ||  .| | ..  ||||.|*| |||| 
 Haller Family||  ||| |, ||.| | | ||| . ..| * ||| ||  .| | ..  ||||.|*| |||| 
"#;

const TITLE: &str = r#"_____ _                         _
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
        AppState::InputApiKey => draw_api_key_input(f, app),
    }
}

fn draw_in_game(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(80), // Game Content
            Constraint::Percentage(20), // User Input
        ])
        .split(f.size());

    draw_game_content(f, app, chunks[0]);
    draw_user_input(f, app, chunks[1]);
}

fn render_game_content(app: &App, area: Rect) -> (Block, Vec<ListItem>, Rect) {
    let block = Block::default()
        .title("Game Content")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner_area = block.inner(area);

    // Calculate the visible range based on the scroll offset
    let start_index = app.game_content_scroll;
    let end_index = (start_index + inner_area.height as usize).min(app.game_content.len());

    // Collect the visible messages
    let messages: Vec<ListItem> = app.game_content[start_index..end_index]
        .iter()
        .map(|message| {
            let style = match message.message_type {
                MessageType::User => Style::default().fg(Color::Yellow),
                MessageType::Game => Style::default().fg(Color::Green),
            };

            let alignment = match message.message_type {
                MessageType::User => Alignment::Right,
                MessageType::Game => Alignment::Left,
            };

            let content = match alignment {
                Alignment::Right => {
                    let padding = " "
                        .repeat((inner_area.width as usize).saturating_sub(message.content.len()));
                    Line::from(vec![
                        Span::raw(padding),
                        Span::styled(message.content.clone(), style),
                    ])
                }
                _ => Line::from(Span::styled(message.content.clone(), style)),
            };

            ListItem::new(content)
        })
        .collect();

    (block, messages, inner_area)
}

fn draw_game_content(f: &mut Frame, app: &App, area: Rect) {
    let (block, messages, inner_area) = render_game_content(app, area);

    f.render_widget(block, area);

    let messages = List::new(messages).block(Block::default().borders(Borders::NONE));
    f.render_widget(messages, inner_area);
}

fn draw_user_input(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title("Your Action")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let input = Paragraph::new(app.user_input.as_str())
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    f.render_widget(input, inner_area);

    // Calculate cursor position
    let lines: Vec<&str> = app.user_input.split('\n').collect();
    let (cursor_x, cursor_y) = lines.iter().enumerate().fold((0, 0), |(_, y), (i, line)| {
        if app.cursor_position > line.len() + y {
            (0, y + line.len() + 1)
        } else {
            (app.cursor_position - y, y)
        }
    });

    // Set cursor
    f.set_cursor(
        inner_area.x + cursor_x as u16,
        inner_area.y + cursor_y as u16,
    )
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
                Constraint::Max(3),  // Fixed height for ASCII art
                Constraint::Max(20), // Fixed height for ASCII art
                Constraint::Max(7),  // Fixed height for title art
                Constraint::Min(2),  // Minimum height for console
                Constraint::Min(6),  // Minimum height for menu
                Constraint::Max(3),  // Fixed height for status bar
            ]
            .as_ref(),
        )
        .split(f.size());

    let header = Paragraph::new(format!("Sharad Ratatui v{}", env!("CARGO_PKG_VERSION")))
        .style(Style::default().fg(Color::DarkGray)) // Style the status bar text with light cyan color
        .block(Block::default()) // Add borders to the status bar
        .alignment(Alignment::Center); // Center align the status bar text
    f.render_widget(header, chunks[0]); // Render the status bar in the third chunk
                                        //
    render_art(f, chunks[1]);
    render_title(f, chunks[2]);
    // Render status bar
    render_console(f, chunks[3]);
    // Render menu
    render_menu(f, app, chunks[4]); // Render the menu in the second chunk

    // Render status bar
    let status = Paragraph::new("Press q to quit")
        .style(Style::default().fg(Color::LightCyan)) // Style the status bar text with light cyan color
        .block(Block::default().borders(Borders::ALL)) // Add borders to the status bar
        .alignment(Alignment::Center); // Center align the status bar text
    f.render_widget(status, chunks[5]); // Render the status bar in the third chunk
}

fn render_menu(f: &mut Frame, app: &App, area: Rect) {
    // Function to render the menu
    let menu_items = [
        "Start a new game", // First menu item
        "Load a game",      // Second menu item
        "Create an image",  // Third menu item
        "Settings",         // Fourth menu item
    ];

    let text: Vec<Line> = menu_items
        .iter()
        .enumerate()
        .map(|(i, &item)| {
            let number = format!("{}. ", i + 1);
            let content = item;
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
        .style(Style::default().fg(Color::DarkGray)); // Style the outer block

    let menu_area = centered_rect(100, 100, area); // Create a centered rectangle for the menu

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
            vertical: 1, // No vertical margin
            horizontal: ((area.width as u16 - text[0].width() as u16) / 2 as u16), // Horizontal margin of 27 units
        }))[1];

    let menu = Paragraph::new(text)
        .alignment(Alignment::Left) // Left align the menu text
        .style(Style::default().fg(Color::White)); // Style the menu text

    // Render the menu text in the inner area
    f.render_widget(menu, inner_area);
}

fn render_art(f: &mut Frame, area: Rect) {
    let outer_block = Block::default().style(Style::default().fg(Color::DarkGray));

    f.render_widget(outer_block, area);

    // Calculate the center position
    let center_x = area.x + (area.width - 80) / 2;
    let center_y = area.y + (area.height - 18) / 2;

    // Create the inner rect with fixed dimensions (77x15) centered within the outer area
    let inner_rect = Rect::new(center_x, center_y, 80, 18);

    let inner_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Green));

    f.render_widget(inner_block, inner_rect);

    let art = Paragraph::new(Text::raw(ART))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));

    f.render_widget(art, inner_rect);
}

fn render_title(f: &mut Frame, area: Rect) {
    let outer_block = Block::default().style(Style::default().fg(Color::DarkGray));

    let title_outer_area = centered_rect(100, 100, area); // Create a centered rectangle for the title
    f.render_widget(&outer_block, title_outer_area);

    // Create an inner area with margins for the title
    let title_inner_area = Layout::default()
        .direction(Direction::Vertical) // Arrange elements vertically inside the title area
        .constraints([
            Constraint::Length(0), // Fixed height for top margin
            Constraint::Max(9),    // Minimum height for content
            Constraint::Length(0), // Fixed height for bottom margin
        ])
        .split(title_outer_area.inner(Margin {
            vertical: 0, // No vertical margin
            horizontal: 1,
        }))[1];

    let title = Paragraph::new(TITLE)
        .alignment(Alignment::Center) // Left align the title text
        .style(Style::default().fg(Color::Green)); // Style the title text

    // Render the title text in the inner area
    f.render_widget(title, title_inner_area);
}

fn render_console(f: &mut Frame, area: Rect) {
    let outer_block = Block::default().style(Style::default().fg(Color::DarkGray));

    let title_outer_area = centered_rect(100, 100, area); // Create a centered rectangle for the title
    f.render_widget(&outer_block, title_outer_area);

    // Create an inner area with margins for the title
    let title_inner_area = Layout::default()
        .direction(Direction::Vertical) // Arrange elements vertically inside the title area
        .constraints([
            Constraint::Length(0), // Fixed height for top margin
            Constraint::Max(3),    // Minimum height for content
            Constraint::Length(0), // Fixed height for bottom margin
        ])
        .split(title_outer_area.inner(Margin {
            vertical: 1, // No vertical margin
            horizontal: 1,
        }))[1];

    let text = Paragraph::new("Welcome to Sharad Ratatui!")
        .alignment(Alignment::Center) // Left align the title text
        .style(Style::default().fg(Color::Green)); // Style the title text

    // Render the title text in the inner area
    f.render_widget(text, title_inner_area);
}

fn draw_settings(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical) // Arrange elements vertically
        .constraints(
            [
                Constraint::Max(3),  // Fixed height for ASCII art
                Constraint::Max(20), // Fixed height for ASCII art
                Constraint::Max(7),  // Fixed height for title art
                Constraint::Min(2),  // Minimum height for console
                Constraint::Min(6),  // Minimum height for menu
                Constraint::Max(3),  // Fixed height for status bar
            ]
            .as_ref(),
        )
        .split(f.size());

    let header = Paragraph::new(format!("Sharad Ratatui v{}", env!("CARGO_PKG_VERSION")))
        .style(Style::default().fg(Color::DarkGray)) // Style the status bar text with light cyan color
        .block(Block::default()) // Add borders to the status bar
        .alignment(Alignment::Center); // Center align the status bar text
    f.render_widget(header, chunks[0]); // Render the status bar in the third chunk
                                        //
    render_art(f, chunks[1]);
    render_title(f, chunks[2]);
    // Render status bar
    render_console(f, chunks[3]);
    // Render menu

    render_settings(f, app, chunks[4]);

    let status = Paragraph::new("Press Esc to return to main menu")
        .style(Style::default().fg(Color::LightCyan))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
    f.render_widget(status, chunks[5]);
}

fn render_settings(f: &mut Frame, app: &App, area: Rect) {
    let settings = [
        ("Language", vec!["English", "Français", "日本語"]),
        ("OpenAI API Key", vec![]),
        ("Voice Output", vec!["On", "Off"]),
        ("Voice Input", vec!["On", "Off"]),
        ("Debug Mode", vec!["Off", "On"]),
    ];

    let text: Vec<Line> = settings
        .iter()
        .enumerate()
        .map(|(number, (setting, options))| {
            let is_selected_setting = number == app.settings_state.selected_setting;

            let highlight_line_style = if is_selected_setting {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let mut spans = vec![
                Span::styled(
                    format!("{}. ", number + 1),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(format!("{:<15}", setting), highlight_line_style),
            ];

            if number == 1 {
                // API Key setting
                let api_key_status = if app.settings.openai_api_key.is_none() {
                    spans.push(Span::styled("[not valid]", Style::default().fg(Color::Red)));
                } else {
                    spans.push(Span::styled("[valid]", Style::default().fg(Color::Green)));
                };
            } else {
                let selected_option = app.settings_state.selected_options[number];
                spans.extend(options.iter().enumerate().map(|(option_number, option)| {
                    let is_selected_option = option_number == selected_option;
                    let option_style = if is_selected_option {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    Span::styled(format!("[{}] ", option), option_style)
                }));
            }

            Line::from(spans)
        })
        .collect();

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .title("Settings")
        .style(Style::default().fg(Color::DarkGray));

    let settings_area = centered_rect(100, 100, area);
    f.render_widget(outer_block, settings_area);

    let inner_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(settings_area.inner(Margin {
            vertical: 1,
            horizontal: (area.width as u16 - text[0].width() as u16) / 2,
        }))[1];

    let settings_widget = Paragraph::new(text)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White));

    f.render_widget(settings_widget, inner_area);
}

fn draw_api_key_input(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Min(1),
            ]
            .as_ref(),
        )
        .split(f.size());

    let title = Paragraph::new("Enter OpenAI API Key")
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    let input = Paragraph::new(app.api_key_input.as_str())
        .style(Style::default().fg(Color::Red))
        .block(Block::default().borders(Borders::ALL).title("API Key"));
    f.render_widget(input, chunks[1]);

    let instructions = Paragraph::new("Press Enter to confirm, Esc to cancel")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    f.render_widget(instructions, chunks[2]);

    let paste_info = Paragraph::new("Use Ctrl+V to paste")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    f.render_widget(paste_info, chunks[3]);
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
