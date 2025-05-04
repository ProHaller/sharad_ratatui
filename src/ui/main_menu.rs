// src/ui/main_menu.rs

// Import required modules and structs from other parts of the application or external crates.
use super::{
    Component, ComponentEnum, api_key_input::ApiKeyInput, draw::center_rect, image_menu::ImageMenu,
    load_menu::LoadMenu, main_menu_fix::*, save_name_input::SaveName, settings_menu::SettingsMenu,
    widgets::StatefulList,
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

const MAIN_MENU: [&str; 4] = [
    "Start a new game",
    "Load a game",
    "Create an image",
    "Settings",
];

#[derive(Debug)]
pub struct MainMenu {
    state: StatefulList<&'static str>,
}

impl Default for MainMenu {
    fn default() -> Self {
        let mut menu = Self {
            state: StatefulList::with_items(Vec::from(MAIN_MENU)),
        };
        menu.state.state.select(Some(0));
        menu
    }
}

impl Component for MainMenu {
    fn on_key(&mut self, key: KeyEvent, context: &mut Context) -> Option<Action> {
        match key.code {
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => self.switch_component(context),
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.previous();
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.next();
                None
            }
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char(c) => {
                if let Some(digit) = c.to_digit(10) {
                    let selected = ((digit as usize).saturating_sub(1)) % self.state.items.len();
                    self.state.state.select(Some(selected));
                    self.switch_component(context)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    fn render(&mut self, area: Rect, buffer: &mut Buffer, context: &Context) {
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
                    Constraint::Length(1),
                    Constraint::Min(MAIN_MENU.len() as u16 + 2),
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
    // Function to render the console section of the menu.
    fn render_console(&self, buffer: &mut Buffer, context: &Context, area: Rect) {
        let outer_block = Block::default()
            .border_type(BorderType::Rounded)
            .style(Style::default().fg(Color::DarkGray));
        let console_area = center_rect(area, Constraint::Percentage(90), Constraint::Length(2));
        outer_block.render(console_area, buffer);

        let console_message: Option<Paragraph> = context
            .messages
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
    fn render_menu(&self, buffer: &mut Buffer, _context: &Context, area: Rect) {
        // Define menu items to be displayed.
        let menu_items = MAIN_MENU;

        // Map menu items to text lines, applying different styles to the selected item.
        // TODO: Make it into a constant
        let menu_lines: Vec<Line> = menu_items
            .iter()
            .enumerate()
            .map(|(i, &item)| {
                let number = format!("{}. ", i + 1);
                let content = item;
                if i == self.state.state.selected().unwrap_or(0) {
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
            Constraint::Length(MAIN_MENU.len() as u16 + 2),
        );

        let menu = Paragraph::new(menu_lines)
            .alignment(Alignment::Left)
            .style(Style::default().fg(Color::White));

        menu.render(centered_area, buffer);
    }

    pub fn switch_component(&mut self, context: &mut Context<'_>) -> Option<Action> {
        match self.state.state.selected() {
            Some(0) => {
                if context.ai_client.is_some() {
                    Some(Action::SwitchComponent(
                        ComponentEnum::from(SaveName::new()),
                    ))
                } else {
                    Some(Action::SwitchComponent(ComponentEnum::from(
                        ApiKeyInput::new(&context.settings.openai_api_key),
                    )))
                }
            }
            Some(1) => {
                // Load Game
                Some(Action::SwitchComponent(ComponentEnum::from(
                    LoadMenu::default(context),
                )))
            }
            Some(2) => {
                if context.ai_client.is_some() {
                    Some(Action::SwitchComponent(ComponentEnum::from(
                        ImageMenu::new(context.image_sender.clone()),
                    )))
                } else {
                    Some(Action::SwitchComponent(ComponentEnum::from(
                        ApiKeyInput::new(&context.settings.openai_api_key),
                    )))
                }
            }
            Some(3) => Some(Action::SwitchComponent(ComponentEnum::from(
                SettingsMenu::new(context),
            ))),
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
