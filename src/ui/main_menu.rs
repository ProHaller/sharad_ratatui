// src/ui/main_menu.rs

// Import required modules and structs from other parts of the application or external crates.
use super::{
    Component,
    api_key_input::ApiKeyInput,
    constants::{ART, TITLE},
    draw::center_rect,
    image_menu::ImageMenu,
    load_menu::LoadMenu,
    main_menu_fix::*,
    save_name_input::SaveName,
    settings_menu::SettingsMenu,
};

use crate::{app::Action, context::Context, message::MessageType};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::*,
};

#[derive(Default, Debug)]
pub struct MainMenu {
    state: ListState,
}

impl Component for MainMenu {
    fn on_key(&mut self, key: KeyEvent, context: Context) -> Option<Action> {
        match key.code {
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                match self.state.selected() {
                    Some(0) => Some(Action::SwitchComponent(Box::new(SaveName::default()))),
                    Some(1) => {
                        // Load Game
                        Some(Action::SwitchComponent(Box::new(LoadMenu::default())))
                    }
                    Some(2) => {
                        if context.openai_api_key_valid {
                            Some(Action::SwitchComponent(Box::new(ImageMenu::default())))
                        } else {
                            Some(Action::SwitchComponent(Box::new(ApiKeyInput::default())))
                        }
                    }
                    Some(3) => Some(Action::SwitchComponent(Box::new(SettingsMenu::default()))),
                    _ => None,
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.navigate_main_menu(-1);
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.navigate_main_menu(1);
                None
            }
            KeyCode::Char(c) if ('1'..='4').contains(&c) => {
                self.select_main_menu_by_char(c);
                None
            }
            KeyCode::Char('q') => {
                crate::cleanup::cleanup();
                std::process::exit(0);
            }
            _ => None,
        }
    }
    fn render(&self, area: Rect, buffer: &mut Buffer, context: &Context) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .flex(ratatui::layout::Flex::Center)
            .constraints(
                [
                    Constraint::Max(1),
                    // TODO: extract the logic in separate functions
                    Constraint::Length(if area.height - 20 > 20 { 20 } else { 0 }),
                    Constraint::Length(if area.height - 7 > 7 { 7 } else { 0 }),
                    Constraint::Max(1),
                    Constraint::Min(10),
                ]
                .as_ref(),
            )
            .split(area);

        // Render individual parts of the main menu using the layout defined above.
        self.render_header(buffer, chunks[0]);
        self.render_art(buffer, chunks[1]);
        self.render_title(buffer, chunks[2]);
        self.render_console(buffer, context, chunks[3]);
        self.render_menu(buffer, context, chunks[4]);
    }
}

impl MainMenu {
    fn navigate_main_menu(&mut self, direction: isize) {
        let i = self.state.selected().unwrap_or(0) as isize;
        let new_i = (i + direction).rem_euclid(4) as usize;
        self.state.select(Some(new_i));
    }
    fn select_main_menu_by_char(&mut self, c: char) {
        let index = (c as usize - 1) % 4;
        self.state.select(Some(index));
    }
    pub fn draw_main_menu(self, buffer: &mut Buffer, area: Rect, context: &Context) {
        // Define layout constraints for different sections of the main menu.
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .flex(ratatui::layout::Flex::Center)
            .constraints(
                [
                    Constraint::Max(1),
                    Constraint::Length(if area.height - 20 > 20 { 20 } else { 0 }),
                    Constraint::Length(if area.height - 7 > 7 { 7 } else { 0 }),
                    Constraint::Max(1),
                    Constraint::Min(10),
                ]
                .as_ref(),
            )
            .split(area);

        // Render individual parts of the main menu using the layout defined above.
        render_header(buffer, chunks[0]);
        render_art(buffer, chunks[1]);
        render_title(buffer, chunks[2]);
        self.render_console(buffer, context, chunks[3]);
        self.render_menu(buffer, context, chunks[4]);
    }

    // Function to render the header section of the menu.
    pub fn render_header(&self, buffer: &mut Buffer, area: Rect) {
        let header = Paragraph::new(format!("Sharad Ratatui v{}", env!("CARGO_PKG_VERSION")))
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().border_type(BorderType::Rounded))
            .alignment(Alignment::Center);
        header.render(area, buffer);
    }

    // Function to render the art section of the menu.
    pub fn render_art(&self, buffer: &mut Buffer, area: Rect) {
        let outer_block = Block::default()
            .border_type(BorderType::Rounded)
            .style(Style::default().fg(Color::DarkGray));
        outer_block.render(area, buffer);

        let inner_rect = center_rect(area, Constraint::Length(80), Constraint::Length(18));

        let inner_block = Block::default()
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Green));
        inner_block.render(inner_rect, buffer);

        let art = Paragraph::new(ART)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Green));
        art.render(inner_rect, buffer);
    }

    // Function to render the title section of the menu.
    pub fn render_title(&self, buffer: &mut Buffer, area: Rect) {
        let outer_block = Block::default()
            .border_type(BorderType::Rounded)
            .style(Style::default().fg(Color::DarkGray));
        let title_area = center_rect(area, Constraint::Length(38), Constraint::Length(8));
        outer_block.render(title_area, buffer);

        let title = Paragraph::new(TITLE)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Green));
        title.render(title_area, buffer);
    }

    // Function to render the console section of the menu.
    pub fn render_console(&self, buffer: &mut Buffer, context: &Context, area: Rect) {
        let outer_block = Block::default()
            .border_type(BorderType::Rounded)
            .style(Style::default().fg(Color::DarkGray));
        let console_area = center_rect(area, Constraint::Percentage(90), Constraint::Length(2));
        outer_block.render(console_area, buffer);

        let console_message: Option<Paragraph> = context
            .console_messages
            .borrow()
            .last()
            .filter(|content| content.message_type == MessageType::System)
            .map(|content| {
                Paragraph::new(content.content.to_string())
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(Color::Yellow))
            });

        if let Some(message) = console_message {
            message.render(console_area, buffer);
        }
    }

    // Function to render the interactive menu section of the main menu.
    pub fn render_menu(&self, buffer: &mut Buffer, context: &Context, area: Rect) {
        // Define menu items to be displayed.
        let menu_items = [
            "Start a new game",
            "Load a game",
            "Create an image",
            "Settings",
        ];

        // Map menu items to text lines, applying different styles to the selected item.
        // TODO: Make it into a constant
        let menu_lines: Vec<Line> = menu_items
            .iter()
            .enumerate()
            .map(|(i, &item)| {
                let number = format!("{}. ", i + 1);
                let content = item;
                if i == self.state.selected().unwrap_or(0) {
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
            Constraint::Length(context.save_manager.available_saves.len() as u16 + 2),
        );

        let menu = Paragraph::new(menu_lines)
            .alignment(Alignment::Left)
            .style(Style::default().fg(Color::White));

        menu.render(centered_area, buffer);
    }
}

// Function to render the status bar at the bottom of the menu.
// TODO: Should make this into key_hint implementation.
// pub fn render_status(buffer: &mut Buffer, context: &Context, area: Rect) {
//     // Define the status message based on the current application state.
//     let status_message = match context.state {
//         AppState::MainMenu => "Press q to quit",
//         AppState::LoadMenu => {
//             "Press Enter or number to load save, Backspace twice to delete save, Esc to go back"
//         }
//         _ => "Press Esc to go back",
//     };
//     let status = Paragraph::new(status_message)
//         .style(Style::default().fg(Color::DarkGray))
//         .block(
//             Block::default()
//                 .border_type(BorderType::Rounded)
//                 .borders(Borders::NONE),
//         )
//         .alignment(Alignment::Center);
//     status.render(area, buffer);
// }
