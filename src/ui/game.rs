// ui/game.rs

use crate::app::App;
use crate::message::{Message, MessageType};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::*,
    Frame,
};

pub fn draw_in_game(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        .split(f.size());

    draw_game_content(f, app, chunks[0]);
    draw_user_input(f, app, chunks[1]);
}
// TODO: Make the scrolling start position at the bottom.
fn draw_game_content(f: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title("Game Content")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner_area = block.inner(area);

    f.render_widget(block, area);

    let max_width = inner_area.width as usize - 2;
    let mut all_lines = Vec::new();

    // Calculate all lines and their styles
    for message in &app.game_content {
        let (style, alignment) = match message.message_type {
            MessageType::User => (Style::default().fg(Color::Blue), Alignment::Right),
            MessageType::Game => (Style::default().fg(Color::Green), Alignment::Left),
            MessageType::System => (Style::default().fg(Color::Yellow), Alignment::Center),
        };

        let wrapped_lines = textwrap::wrap(&message.content, max_width);
        for line in wrapped_lines {
            all_lines.push((line.to_string(), style, alignment));
        }
    }

    app.total_lines = all_lines.len();

    // Render visible lines
    let visible_lines: Vec<Line> = all_lines
        .iter()
        .skip(app.game_content_scroll)
        .take(inner_area.height as usize)
        .map(|(content, style, alignment)| {
            let span = Span::styled(content.clone(), *style);
            match alignment {
                Alignment::Right => Line::from(vec![span]).alignment(Alignment::Right),
                Alignment::Left => Line::from(vec![span]),
                Alignment::Center => Line::from(vec![span]).alignment(Alignment::Center),
            }
        })
        .collect();

    let content = Paragraph::new(visible_lines).block(Block::default().borders(Borders::NONE));

    f.render_widget(content, inner_area);

    // Update app state
    app.visible_lines = inner_area.height as usize;
    app.update_scroll();
    app.update_debug_info();

    // // Render debug info
    // let debug_area = Rect::new(area.x, area.bottom() - 1, area.width, 1);
    // let debug_text = Paragraph::new(app.debug_info.clone()).style(Style::default().fg(Color::Gray));
    // f.render_widget(debug_text, debug_area);
}

fn render_messages(game_content: &[Message], width: u16) -> Vec<ListItem> {
    game_content
        .iter()
        .map(|message| {
            let (style, alignment) = match message.message_type {
                MessageType::User => (Style::default().fg(Color::Blue), Alignment::Right),
                MessageType::Game => (Style::default().fg(Color::Green), Alignment::Left),
                MessageType::System => (Style::default().fg(Color::Yellow), Alignment::Center),
            };

            let wrapped_content = textwrap::wrap(&message.content, width as usize - 2)
                .into_iter()
                .map(|s| Line::from(vec![Span::styled(s, style)]))
                .collect::<Vec<Line>>();

            let content = match alignment {
                Alignment::Right => Text::from(wrapped_content).alignment(Alignment::Right),
                Alignment::Left => Text::from(wrapped_content).alignment(Alignment::Left),
                Alignment::Center => Text::from(wrapped_content).alignment(Alignment::Center),
            };

            ListItem::new(content)
        })
        .collect()
}

fn render_game_content(app: &mut App, area: Rect) -> (Block, Vec<ListItem>, Rect) {
    let block = Block::default()
        .title("Game Content")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner_area = block.inner(area);

    // Update the number of visible messages and scroll position
    let new_visible_messages = inner_area.height as usize;
    if app.visible_messages != new_visible_messages {
        app.visible_messages = new_visible_messages;
        app.update_scroll();
    }

    // Collect all messages
    let messages: Vec<ListItem> = app
        .game_content
        .iter()
        .map(|message| {
            let (style, alignment) = match message.message_type {
                MessageType::User => (Style::default().fg(Color::Blue), Alignment::Right),
                MessageType::Game => (Style::default().fg(Color::Green), Alignment::Left),
                MessageType::System => (Style::default().fg(Color::Yellow), Alignment::Center),
            };

            let wrapped_content = textwrap::wrap(&message.content, inner_area.width as usize - 2)
                .into_iter()
                .map(|s| Line::from(vec![Span::styled(s, style)]))
                .collect::<Vec<Line>>();

            let content = match alignment {
                Alignment::Right => Text::from(wrapped_content).alignment(Alignment::Right),
                Alignment::Left => Text::from(wrapped_content).alignment(Alignment::Left),
                Alignment::Center => Text::from(wrapped_content).alignment(Alignment::Center),
            };

            ListItem::new(content)
        })
        .collect();

    (block, messages, inner_area)
}

fn draw_user_input(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title("Your Action")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let wrapped_input = textwrap::wrap(&app.user_input, inner_area.width as usize - 2);
    let input = Paragraph::new(wrapped_input.join("\n"))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    f.render_widget(input, inner_area);

    // Calculate cursor position
    let cursor_position = textwrap::core::display_width(&app.user_input[..app.cursor_position]);
    let (cursor_x, cursor_y) = if cursor_position < inner_area.width as usize - 2 {
        (cursor_position as u16, 0)
    } else {
        let line = cursor_position / (inner_area.width as usize - 2);
        let column = cursor_position % (inner_area.width as usize - 2);
        (column as u16, line as u16)
    };

    // Set cursor

    f.set_cursor(
        inner_area.x + cursor_x as u16,
        inner_area.y + cursor_y as u16,
    );
}
