// ui/settings_menu.rs

use crate::{
    app::Action, context::Context, save::get_game_data_dir, settings::Language,
    settings_state::SettingsState, ui::draw::center_rect,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    prelude::Buffer,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::*,
};

use super::{Component, ComponentEnum, MainMenu, api_key_input::ApiKeyInput, main_menu_fix::*};

#[derive(Debug)]
pub struct SettingsMenu {
    pub state: SettingsState,
}

impl Component for SettingsMenu {
    fn on_key(&mut self, key: KeyEvent, context: &mut Context) -> Option<Action> {
        let action: Option<Action> = match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.selected_setting = if self.state.selected_setting == 0 {
                    self.state.selected_options.len() - 1
                } else {
                    self.state.selected_setting - 1
                };
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.selected_setting =
                    if self.state.selected_setting >= self.state.selected_options.len() - 1 {
                        0
                    } else {
                        self.state.selected_setting + 1
                    };
                None
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.change_settings(-1);
                None
            }
            KeyCode::Right | KeyCode::Enter | KeyCode::Char('l') => {
                if self.state.selected_setting == 1 {
                    Some(Action::SwitchComponent(ComponentEnum::from(
                        ApiKeyInput::new(&context.settings.openai_api_key),
                    )))
                } else {
                    self.change_settings(1);
                    None
                }
            }
            KeyCode::Esc => Some(Action::SwitchComponent(ComponentEnum::from(
                MainMenu::default(),
            ))),
            KeyCode::Char(c) => {
                if let Some(digit) = c.to_digit(10) {
                    self.state.selected_setting =
                        ((digit as usize).saturating_sub(1)) % self.state.selected_options.len();
                    match self.state.selected_setting {
                        1 => Some(Action::SwitchComponent(ComponentEnum::from(
                            ApiKeyInput::new(&context.settings.openai_api_key),
                        ))),
                        _ => {
                            self.change_settings(1);
                            None
                        }
                    }
                } else {
                    None
                }
            }
            _ => None,
        };
        self.apply_settings(context);
        action
    }

    fn render(&mut self, area: Rect, buffer: &mut Buffer, context: &Context) {
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

        render_header(buffer, chunks[0]);
        render_art(buffer, chunks[1]);
        render_title(buffer, chunks[2]);
        self.render_console(buffer, context, chunks[3]);
        self.render_settings(buffer, context, chunks[4]);
    }
}

impl SettingsMenu {
    pub fn new(context: &mut Context) -> Self {
        Self {
            state: SettingsState::from_settings(context.settings),
        }
    }

    fn render_settings(&self, buffer: &mut Buffer, context: &Context, area: Rect) {
        // TODO: Make this dynamic based on settings content.
        let settings = [
            ("Language", vec!["English", "Français", "日本語", "Türkçe"]),
            ("AI API Key", vec![]),
            ("OpenAI Model", vec!["gpt-4o-mini", "gpt-4o", "o1-mini"]),
            ("Voice Output", vec!["On", "Off"]),
            ("Voice Input", vec!["On", "Off"]),
            ("Debug Mode", vec!["Off", "On"]),
        ];

        let text: Vec<Line> = settings
            .iter()
            .enumerate()
            .map(|(number, (setting, options))| {
                let is_selected_setting = number == self.state.selected_setting;

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
                    Span::styled(format!("{:<20}", setting), highlight_line_style),
                ];

                if number == 1 {
                    // API Key setting
                    let api_key_status = if context.settings.openai_api_key.is_some() {
                        Span::styled("[Valid]", Style::default().fg(Color::Green))
                    } else {
                        Span::styled("[Not Valid]", Style::default().fg(Color::Red))
                    };
                    spans.push(api_key_status);
                } else {
                    let selected_option = self.state.selected_options[number];
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
            .border_type(BorderType::Rounded)
            .borders(Borders::NONE)
            .style(Style::default().fg(Color::DarkGray));

        let settings_area = center_rect(
            area,
            Constraint::Percentage(100),
            Constraint::Percentage(100),
        );
        outer_block.render(settings_area, buffer);

        let inner_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(settings_area.inner(Margin {
                vertical: 1,
                horizontal: (area.width - text[0].width() as u16) / 2,
            }))[1];

        let settings_widget = Paragraph::new(text)
            .alignment(Alignment::Left)
            .style(Style::default().fg(Color::White));

        settings_widget.render(inner_area, buffer);
    }

    fn render_console(&self, buffer: &mut Buffer, _context: &Context, area: Rect) {
        let console_text = format!(
            "The Settings are saved at: {:#?}/settings.json",
            get_game_data_dir()
        );

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

    pub fn apply_settings(&mut self, context: &mut Context) {
        // Apply changes from settings_state to settings
        context.settings.language = match self.state.selected_options[0] {
            0 => Language::English,
            1 => Language::French,
            2 => Language::Japanese,
            3 => Language::Turkish,
            _ => context.settings.language.clone(),
        };
        context.settings.model = match self.state.selected_options[2] {
            0 => "gpt-4o-mini".to_string(),
            1 => "gpt-4o".to_string(),
            2 => "o1-mini".to_string(),
            _ => context.settings.model.clone(),
        };
        context.settings.audio_output_enabled = self.state.selected_options[3] == 0;
        context.settings.audio_input_enabled = self.state.selected_options[4] == 0;
        context.settings.debug_mode = self.state.selected_options[5] == 1;

        // Save settings to file
        let home_dir = dir::home_dir().expect("Failed to get home directory");
        let path = home_dir.join("sharad").join("data").join("settings.json");
        if let Err(e) = context.settings.save_to_file(path) {
            eprintln!("Failed to save settings: {:#?}", e);
        }
    }

    fn change_settings(&mut self, change: isize) {
        let current_setting = self.state.selected_setting;
        match (current_setting, change) {
            (0, change) => {
                if self.state.selected_options[current_setting] == 0 {
                    self.state.selected_options[current_setting] = (4 + change) as usize % 4;
                } else {
                    self.state.selected_options[current_setting] =
                        (self.state.selected_options[current_setting] as isize + change) as usize
                            % 4
                }
            }
            (2, change) => {
                if self.state.selected_options[current_setting] == 0 {
                    self.state.selected_options[current_setting] = (3 + change) as usize % 3;
                } else {
                    self.state.selected_options[current_setting] =
                        (self.state.selected_options[current_setting] as isize + change) as usize
                            % 3
                }
            }
            (_current, _change) => {
                self.state.selected_options[current_setting] =
                    1 - self.state.selected_options[current_setting];
            }
        }
    }
}
