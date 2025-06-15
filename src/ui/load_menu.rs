// ui/load_menu.rs

use std::path::PathBuf;

use super::{
    Component, ComponentEnum, SaveName, api_key_input::ApiKeyInput, draw::center_rect,
    main_menu_fix::*, widgets::StatefulList,
};
use crate::{
    app::Action,
    context::Context,
    save::{self, get_save_base_dir},
    ui::MainMenu,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::*,
};

#[derive(Debug)]
pub struct LoadMenu {
    state: StatefulList<PathBuf>,
    backspace_counter: bool,
}

impl Component for LoadMenu {
    fn on_key(&mut self, key: KeyEvent, context: &mut Context) -> Option<Action> {
        match key.code {
            KeyCode::Enter if context.save_manager.available_saves.is_empty() => {
                match context.ai_client {
                    Some(_) => Some(Action::SwitchComponent(ComponentEnum::SaveName(
                        SaveName::default(),
                    ))),
                    None => Some(Action::SwitchComponent(ComponentEnum::ApiKeyInput(
                        ApiKeyInput::new(&None),
                    ))),
                }
            }
            KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => {
                self.state.state.selected().map(|selected| {
                    Action::LoadSave(context.save_manager.available_saves[selected].clone())
                })
            }

            KeyCode::Esc | KeyCode::Char('h') => Some(Action::SwitchComponent(
                ComponentEnum::from(MainMenu::default()),
            )),
            KeyCode::Up | KeyCode::Char('k') => {
                self.backspace_counter = false;
                self.state.previous();
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.backspace_counter = false;
                self.state.next();
                None
            }
            KeyCode::Backspace => {
                if self.backspace_counter {
                    if !&context.save_manager.available_saves.is_empty() {
                        context
                            .save_manager
                            .clone()
                            .delete_save(
                                &context.save_manager.available_saves
                                    [self.state.state.selected().unwrap()]
                                .clone(),
                                &context.settings.openai_api_key.clone().unwrap(),
                            )
                            .expect("Expected save deletion");
                    }
                    self.backspace_counter = false;
                    context.save_manager.available_saves = save::SaveManager::scan_save_files();
                    self.state.items = context.save_manager.available_saves.clone();
                    None
                } else {
                    self.backspace_counter = true;
                    None
                }
            }

            KeyCode::Char(c) => {
                if self.state.items.is_empty() {
                    return None;
                };
                if let Some(digit) = c.to_digit(10) {
                    let selected = ((digit as usize).saturating_sub(1)) % self.state.items.len();
                    self.state.state.select(Some(selected));
                    let save_name = context.save_manager.available_saves[selected].clone();
                    Some(Action::LoadSave(save_name))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn render(&mut self, area: Rect, buffer: &mut Buffer, context: &Context) {
        let saves_length = context.save_manager.available_saves.len() as u16;
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .flex(ratatui::layout::Flex::Center)
            .constraints(
                [
                    Constraint::Max(1),
                    Constraint::Length(if area.height - 20 > 20 { 20 } else { 0 }),
                    Constraint::Length(if (area.height - saves_length - 7) > 7 {
                        7
                    } else {
                        0
                    }),
                    Constraint::Length(1),
                    Constraint::Min(saves_length + 2),
                    Constraint::Length(1),
                ]
                .as_ref(),
            )
            .split(area);

        render_header(buffer, chunks[0]);
        render_art(buffer, chunks[1]);
        render_title(buffer, chunks[2]);
        self.render_console(buffer, context, chunks[3]);
        self.render_load_menu(buffer, context, chunks[4]);
        self.render_hints(buffer, chunks[5]);
    }
}
impl Hints for LoadMenu {
    fn display(&self) -> String {
        "Main Menu -> Load Menu:".to_string()
    }

    fn key_hints(&self) -> String {
        "Navigate: ←↓↑→ or hjkl. Go Back to Main Manu: Esc".to_string()
    }
}

impl LoadMenu {
    pub fn default(context: &mut Context) -> Self {
        let mut menu = Self {
            state: StatefulList::with_items(context.save_manager.available_saves.clone()),
            backspace_counter: false,
        };
        menu.state.next();
        menu
    }
    fn render_console(&self, buffer: &mut Buffer, context: &Context, area: Rect) {
        let console_text = if context.save_manager.available_saves.is_empty() {
            format!("No save files found in {}.", get_save_base_dir().display())
        } else {
            "Select a save file to load".to_string()
        };

        let console = Paragraph::new(console_text)
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .border_type(BorderType::Rounded)
                    .borders(Borders::NONE),
            );

        console.render(area, buffer);
    }

    fn render_load_menu(&self, buffer: &mut Buffer, context: &Context, area: Rect) {
        let text: Vec<Line> = if context.save_manager.available_saves.is_empty() {
            vec![
                Line::from(Span::raw("No save files available")),
                Line::from(Span::raw("Press Enter to Start a new game")),
            ]
        } else {
            context
                .save_manager
                .available_saves
                .iter()
                .enumerate()
                .map(|(i, save)| {
                    let save_name = save.file_stem().unwrap().to_string_lossy().to_string();
                    if Some(i) == self.state.state.selected() {
                        Line::from(
                            Span::styled(
                                format!("{}. {}", (i + 1), save_name),
                                if !self.backspace_counter {
                                    Style::default().fg(Color::Yellow)
                                } else {
                                    Style::default().fg(Color::Red).rapid_blink()
                                },
                            )
                            .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        Line::from(Span::raw(format!("{}. {}", (i + 1), save_name)))
                    }
                })
                .collect()
        };
        let max_width = text.iter().max_by_key(|line| line.width()).unwrap().width();

        let outer_block = Block::default()
            .border_type(BorderType::Rounded)
            .borders(Borders::NONE)
            .style(Style::default().fg(Color::DarkGray));

        outer_block.render(area, buffer);

        let centered_area = center_rect(
            area,
            Constraint::Length(max_width as u16),
            Constraint::Length(context.save_manager.available_saves.len() as u16 + 2),
        );

        let menu = Paragraph::new(text)
            .alignment(Alignment::Left)
            .style(Style::default().fg(Color::White));

        // HACK: This should probably be a stateful widget if I can have th two step validation for
        // deletion
        menu.render(centered_area, buffer);
    }
}
