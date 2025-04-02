// ui/settings_menu.rs

use crate::{
    app::Action,
    context::Context,
    settings::{Language, Settings},
    settings_state::SettingsState,
    ui::draw::center_rect,
};
use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    prelude::Buffer,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::*,
};

use super::{Component, main_menu_fix::*};

#[derive(Debug, Default)]
pub struct SettingsMenu {
    state: SettingsState,
    settings: Settings,
}

impl Component for SettingsMenu {
    fn on_key(&mut self, key: KeyEvent, context: Context) -> Option<Action> {
        todo!();
        // TODO: adapt this to on_key for SettingsMenu

        // fn handle_settings_input(&mut self, key: KeyEvent) {
        //     match key.code {
        //         KeyCode::Up | KeyCode::Char('k') => {
        //             self.settings_state.selected_setting =
        //                 (self.settings_state.selected_setting + 5) % 6; // Wrap around 6 settings
        //         }
        //         KeyCode::Down | KeyCode::Char('j') => {
        //             self.settings_state.selected_setting =
        //                 (self.settings_state.selected_setting + 1) % 6; // TODO: Make this into a stateful list
        //         }
        //         KeyCode::Left | KeyCode::Char('h') => {
        //             let current_setting = self.settings_state.selected_setting;
        //             if current_setting == 0 {
        //                 // Language setting
        //                 let current_language =
        //                     self.settings_state.selected_options[current_setting];
        //                 self.settings_state.selected_options[current_setting] =
        //                     (current_language + 3) % 4;
        //             } else if current_setting == 2 {
        //                 // Model setting
        //                 let current_model = self.settings_state.selected_options[current_setting];
        //                 self.settings_state.selected_options[current_setting] =
        //                     (current_model + 2) % 3;
        //             } else if current_setting != 1 {
        //                 // Not API Key setting
        //                 self.settings_state.selected_options[current_setting] =
        //                     1 - self.settings_state.selected_options[current_setting];
        //             }
        //             self.apply_settings();
        //         }
        //         KeyCode::Right | KeyCode::Char('l') => {
        //             let current_setting = self.settings_state.selected_setting;
        //             if current_setting == 0 {
        //                 // Language setting
        //                 let current_option = self.settings_state.selected_options[current_setting];
        //                 self.settings_state.selected_options[current_setting] =
        //                     (current_option + 1) % 4;
        //             } else if current_setting == 1 {
        //                 // API Key setting
        //                 self.state = AppState::InputApiKey;
        //             } else if current_setting == 2 {
        //                 // Model setting
        //                 let current_option = self.settings_state.selected_options[current_setting];
        //                 self.settings_state.selected_options[current_setting] =
        //                     (current_option + 1) % 3;
        //             } else if current_setting != 1 {
        //                 // Not API Key setting
        //                 self.settings_state.selected_options[current_setting] =
        //                     1 - self.settings_state.selected_options[current_setting];
        //             }
        //             self.apply_settings();
        //         }
        //         KeyCode::Enter => {
        //             let current_setting = self.settings_state.selected_setting;
        //             if current_setting == 1 {
        //                 // API Key setting
        //                 self.state = AppState::InputApiKey;
        //             } else {
        //                 self.state = AppState::MainMenu;
        //                 self.apply_settings();
        //             }
        //         }
        //         KeyCode::Esc => {
        //             self.state = AppState::MainMenu;
        //         }
        //         KeyCode::Char(c) => {
        //             if let Some(digit) = c.to_digit(10) {
        //                 if digit <= 6 {
        //                     self.settings_state.selected_setting = (digit - 1) as usize;
        //                     let current_setting = self.settings_state.selected_setting;
        //                     if current_setting == 1 {
        //                         // API Key setting
        //                         self.state = AppState::InputApiKey;
        //                     } else {
        //                         let current_option =
        //                             self.settings_state.selected_options[current_setting];
        //                         let new_option = match current_setting {
        //                             0 => (current_option + 1) % 4, // Language (3 options)
        //                             2..=6 => 1 - current_option,   // Toggle settings (2 options)
        //                             _ => current_option,
        //                         };
        //                         self.settings_state.selected_options[current_setting] = new_option;
        //                     }
        //                     self.apply_settings();
        //                 }
        //             }
        //         }
        //         _ => {}
        //     }
        // }
    }

    fn render(&self, area: Rect, buffer: &mut Buffer, context: &Context) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .flex(ratatui::layout::Flex::Center)
            .constraints(
                [
                    Constraint::Max(1),
                    Constraint::Min(if area.height - 20 > 20 { 20 } else { 0 }),
                    Constraint::Min(if area.height - 7 > 7 { 7 } else { 0 }),
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
    fn render_console(&self, buffer: &mut Buffer, context: &Context, area: Rect) {
        let console_text = format!("This should be dynamically filled",);

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

    pub fn apply_settings(&mut self) {
        // Apply changes from settings_state to settings
        self.settings.language = match self.state.selected_options[0] {
            0 => Language::English,
            1 => Language::French,
            2 => Language::Japanese,
            3 => Language::Turkish,
            _ => self.settings.language.clone(),
        };
        self.settings.model = match self.state.selected_options[2] {
            0 => "gpt-4o-mini".to_string(),
            1 => "gpt-4o".to_string(),
            2 => "o1-mini".to_string(),
            _ => self.settings.model.clone(),
        };
        self.settings.audio_output_enabled = self.state.selected_options[3] == 0;
        self.settings.audio_input_enabled = self.state.selected_options[4] == 0;
        self.settings.debug_mode = self.state.selected_options[5] == 1;

        // Save settings to file
        let home_dir = dir::home_dir().expect("Failed to get home directory");
        let path = home_dir.join("sharad").join("data").join("settings.json");
        if let Err(e) = self.settings.save_to_file(path) {
            eprintln!("Failed to save settings: {:#?}", e);
        }
    }
}
