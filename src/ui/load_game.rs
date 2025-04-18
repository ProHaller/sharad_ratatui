// ui/load_game.rs

use super::main_menu::{render_art, render_header, render_status, render_title};
use super::utils::centered_rect;
use crate::app::App;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::*,
};

pub fn draw_load_game(f: &mut Frame, app: &App) {
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
                Constraint::Max(3),
                Constraint::Min(app.save_manager.available_saves.len() as u16),
                Constraint::Max(3),
            ]
            .as_ref(),
        )
        .split(f.area());

    render_header(f, chunks[0]);
    render_art(f, chunks[1]);
    render_title(f, chunks[2]);
    render_console(f, app, chunks[3]);
    render_load_game_menu(f, app, chunks[4]);
    render_status(f, app, chunks[5]);
}

fn render_console(f: &mut Frame, app: &App, area: Rect) {
    let console_text = if app.save_manager.available_saves.is_empty() {
        "No save files found in ./data/save/"
    } else {
        "Select a save file to load"
    };

    let console = Paragraph::new(console_text)
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));

    f.render_widget(console, area);
}

fn render_load_game_menu(f: &mut Frame, app: &App, area: Rect) {
    let text: Vec<Line> = if app.save_manager.available_saves.is_empty() {
        vec![Line::from(Span::raw("No save files available"))]
    } else {
        app.save_manager
            .available_saves
            .iter()
            .enumerate()
            .map(|(i, save)| {
                if Some(i) == app.load_game_menu_state.selected() {
                    Line::from(Span::styled(
                        format!("{}. {}", (i + 1), save),
                        Style::default()
                            .fg(if !app.backspace_counter {
                                Color::Yellow
                            } else {
                                Color::Red
                            })
                            .add_modifier(Modifier::BOLD),
                    ))
                } else {
                    Line::from(Span::raw(format!("{}. {}", (i + 1), save)))
                }
            })
            .collect()
    };

    let outer_block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default().fg(Color::DarkGray));

    let menu_area = centered_rect(100, 100, area);
    f.render_widget(outer_block, menu_area);

    let inner_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .split(menu_area.inner(Margin {
            vertical: 1,
            horizontal: (area.width - text.iter().map(|t| t.width() as u16).max().unwrap()) / 2,
        }))[1];

    let menu = Paragraph::new(text)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White));

    f.render_widget(menu, inner_area);
}
