// ui/main_menu.rs

use super::constants::{ART, TITLE};
use super::utils::centered_rect;
use crate::app::App;
use crate::app_state::AppState;
use crate::message::MessageType;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::*,
    Frame,
};

pub fn draw_main_menu(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Max(3),
                Constraint::Max(20),
                Constraint::Max(7),
                Constraint::Min(2),
                Constraint::Min(6),
                Constraint::Max(3),
            ]
            .as_ref(),
        )
        .split(f.size());

    render_header(f, chunks[0]);
    render_art(f, chunks[1]);
    render_title(f, chunks[2]);
    render_console(f, app, chunks[3]);
    render_menu(f, app, chunks[4]);
    render_status(f, app, chunks[5]);
}

pub fn render_header(f: &mut Frame, area: Rect) {
    let header = Paragraph::new(format!("Sharad Ratatui v{}", env!("CARGO_PKG_VERSION")))
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default())
        .alignment(Alignment::Center);
    f.render_widget(header, area);
}

pub fn render_art(f: &mut Frame, area: Rect) {
    let outer_block = Block::default().style(Style::default().fg(Color::DarkGray));
    f.render_widget(outer_block, area);

    let center_x = area.x + (area.width - 80) / 2;
    let center_y = area.y + (area.height - 18) / 2;
    let inner_rect = Rect::new(center_x, center_y, 80, 18);

    let inner_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Green));
    f.render_widget(inner_block, inner_rect);

    let art = Paragraph::new(ART)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));
    f.render_widget(art, inner_rect);
}

pub fn render_title(f: &mut Frame, area: Rect) {
    let outer_block = Block::default().style(Style::default().fg(Color::DarkGray));
    let title_outer_area = centered_rect(100, 100, area);
    f.render_widget(&outer_block, title_outer_area);

    let title_inner_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(0),
            Constraint::Max(9),
            Constraint::Length(0),
        ])
        .split(title_outer_area.inner(Margin {
            vertical: 0,
            horizontal: 1,
        }))[1];

    let title = Paragraph::new(TITLE)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));
    f.render_widget(title, title_inner_area);
}

pub fn render_console(f: &mut Frame, app: &App, area: Rect) {
    let outer_block = Block::default().style(Style::default().fg(Color::DarkGray));
    let console_outer_area = centered_rect(100, 100, area);
    f.render_widget(&outer_block, console_outer_area);

    let console_inner_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(0),
            Constraint::Max(3),
            Constraint::Length(0),
        ])
        .split(console_outer_area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        }))[1];

    let finaly = if let Some(content) = app.game_content.last() {
        if content.message_type == MessageType::System {
            Some(content.content.to_string())
        } else {
            None
        }
    } else {
        None
    };

    let text = Paragraph::new(finaly.unwrap_or("".to_string()))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(text, console_inner_area);
}

pub fn render_menu(f: &mut Frame, app: &App, area: Rect) {
    let menu_items = [
        "Start a new game",
        "Load a game",
        "Create an image",
        "Settings",
    ];

    let text: Vec<Line> = menu_items
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

    let outer_block = Block::default()
        .borders(Borders::NONE)
        // .title("Menu")
        .style(Style::default().fg(Color::DarkGray));

    let menu_area = centered_rect(100, 100, area);
    f.render_widget(outer_block, menu_area);

    let inner_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(menu_area.inner(Margin {
            vertical: 1,
            horizontal: ((area.width as u16 - text[0].width() as u16) / 2 as u16),
        }))[1];

    let menu = Paragraph::new(text)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White));
    f.render_widget(menu, inner_area);
}

pub fn render_status(f: &mut Frame, app: &App, area: Rect) {
    let status_message = match app.state {
        AppState::MainMenu => "Press q to quit",
        AppState::LoadMenu => {
            "Press Enter or number to load save, Backspace twice to delete save, Esc to go back"
        }
        _ => "Press Esc to go back",
    };
    let status = Paragraph::new(status_message)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::NONE))
        .alignment(Alignment::Center);
    f.render_widget(status, area);
}

// Update the draw_main_menu function to pass the app to render_status
