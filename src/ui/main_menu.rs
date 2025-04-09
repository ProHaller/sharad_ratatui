// src/ui/main_menu.rs

// Import required modules and structs from other parts of the application or external crates.
use super::{
    Component, api_key_input::ApiKeyInput, draw::center_rect, image_menu::ImageMenu,
    load_menu::LoadMenu, main_menu_fix::*, save_name_input::SaveName, settings_menu::SettingsMenu,
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
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => self.switch_component(context),
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
                self.switch_component(context)
            }
            KeyCode::Char('q') => Some(Action::Quit),
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
                    Constraint::Length(if area.height.saturating_sub(20) > 20 {
                        20
                    } else {
                        0
                    }),
                    Constraint::Length(if area.height.saturating_sub(7) > 7 {
                        7
                    } else {
                        0
                    }),
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
}

impl MainMenu {
    fn new() -> Self {
        Self {
            state: ListState::default(),
        }
    }
    fn navigate_main_menu(&mut self, direction: isize) {
        let i = self.state.selected().unwrap_or(0) as isize;
        let new_i = (i + direction).rem_euclid(4) as usize;
        self.state.select(Some(new_i));
    }
    fn select_main_menu_by_char(&mut self, c: char) {
        let index = (c as usize - 1) % 4;
        self.state.select(Some(index));
    }

    // Function to render the console section of the menu.
    fn render_console(&self, buffer: &mut Buffer, context: &Context, area: Rect) {
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
    fn render_menu(&self, buffer: &mut Buffer, context: &Context, area: Rect) {
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

    fn switch_component(&mut self, context: Context<'_>) -> Option<Action> {
        match self.state.selected() {
            Some(0) => Some(Action::SwitchComponent(Box::new(SaveName::default()))),
            Some(1) => {
                // Load Game
                Some(Action::SwitchComponent(Box::new(LoadMenu::default(
                    context,
                ))))
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
}

// Function to render the status bar at the bottom of the menu.
// TODO: Should make this into key_hint implementation.
//
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
