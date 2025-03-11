// ui/settings.rs

use crate::app::App;
use crate::ui::main_menu::{
    render_art, render_console, render_header, render_status, render_title,
};
use crate::ui::utils::centered_rect;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::*,
};

pub fn draw_settings(f: &mut Frame, app: &mut App) {
    let size = f.area();

    if size.width < 20 || size.height < 10 {
        let warning = Paragraph::new("Terminal too small. Please resize.")
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center);
        f.render_widget(warning, size);
        return;
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Max(3),
                Constraint::Max(20),
                Constraint::Max(7),
                Constraint::Fill(1),
                Constraint::Fill(2),
                Constraint::Max(3),
            ]
            .as_ref(),
        )
        .split(f.area());

    render_header(f, chunks[0]);
    render_art(f, chunks[1]);
    render_title(f, chunks[2]);
    render_console(f, app, chunks[3]);
    render_settings(f, app, chunks[4]);
    render_status(f, app, chunks[5]);
}

pub fn render_settings(f: &mut Frame, app: &App, area: Rect) {
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
                Span::styled(format!("{:<20}", setting), highlight_line_style),
            ];

            if number == 1 {
                // API Key setting
                let api_key_status = if app.settings.openai_api_key.is_some() {
                    Span::styled("[Valid]", Style::default().fg(Color::Green))
                } else {
                    Span::styled("[Not Valid]", Style::default().fg(Color::Red))
                };
                spans.push(api_key_status);
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
        .borders(Borders::NONE)
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
            horizontal: (area.width - text[0].width() as u16) / 2,
        }))[1];

    let settings_widget = Paragraph::new(text)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White));

    f.render_widget(settings_widget, inner_area);
}
