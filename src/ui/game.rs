use crate::app::App;
use crate::message::MessageType;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::*,
    Frame,
};

pub fn draw_in_game(f: &mut Frame, app: &mut App) {
    let size = f.size();
    app.debug_info = format!("Terminal size: {}x{}", size.width, size.height);

    // Check if the terminal size is too small
    if size.width < 20 || size.height < 10 {
        let warning = Paragraph::new("Terminal too small. Please resize.")
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center);
        f.render_widget(warning, size);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        .split(size);

    draw_game_content(f, app, chunks[0]);
    draw_user_input(f, app, chunks[1]);
}

fn draw_game_content(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    let narration_block = Block::default()
        .title("Narration")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let game_info_block = Block::default()
        .title("Game Info")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    f.render_widget(&narration_block, chunks[0]);
    f.render_widget(&game_info_block, chunks[1]);

    let narration_area = narration_block.inner(chunks[0]);
    let game_info_area = game_info_block.inner(chunks[1]);

    let max_width = narration_area.width.saturating_sub(2) as usize;
    let max_height = narration_area.height.saturating_sub(2) as usize;

    let mut all_lines = Vec::new();

    // Calculate all lines and their styles
    for message in &app.game_content {
        let (content, style, alignment) = match message.message_type {
            MessageType::User => {
                if let Some(user_message) = message.parse_user_message() {
                    (
                        format!("Player: {}", user_message.player_action),
                        Style::default().fg(Color::Blue),
                        Alignment::Right,
                    )
                } else {
                    continue;
                }
            }
            MessageType::Game => {
                if let Some(game_message) = message.parse_game_message() {
                    (
                        format!("Narration: {}", game_message.narration),
                        Style::default().fg(Color::Green),
                        Alignment::Left,
                    )
                } else {
                    continue;
                }
            }
            MessageType::System => (
                message.content.clone(),
                Style::default().fg(Color::Yellow),
                Alignment::Center,
            ),
        };

        let wrapped_lines = textwrap::wrap(&content, max_width);
        for line in wrapped_lines {
            all_lines.push((line.to_string(), style, alignment));
        }
    }

    app.total_lines = all_lines.len();
    app.debug_info += &format!(", Total lines: {}", app.total_lines);

    // Render visible lines
    let visible_lines: Vec<Line> = all_lines
        .iter()
        .skip(app.game_content_scroll)
        .take(max_height)
        .map(|(content, style, alignment)| {
            let span = Span::styled(content.clone(), *style);
            match alignment {
                Alignment::Right => Line::from(vec![span]).alignment(Alignment::Right),
                Alignment::Left => Line::from(vec![span]),
                Alignment::Center => Line::from(vec![span]).alignment(Alignment::Center),
            }
        })
        .collect();

    app.debug_info += &format!(", Visible lines: {}", visible_lines.len());

    let content = Paragraph::new(visible_lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: true });

    f.render_widget(content, narration_area);

    // Render game info (reasoning) in the right panel
    let mut game_info_content = Vec::new();
    for message in &app.game_content {
        if let MessageType::Game = message.message_type {
            if let Some(game_message) = message.parse_game_message() {
                game_info_content.push(Line::from(vec![
                    Span::styled("Reasoning: ", Style::default().fg(Color::Cyan)),
                    Span::raw(game_message.reasoning),
                ]));
            }
        }
    }

    let game_info_paragraph = Paragraph::new(game_info_content)
        .wrap(Wrap { trim: true })
        .scroll((app.game_content_scroll as u16, 0));

    f.render_widget(game_info_paragraph, game_info_area);

    // Update app state
    app.visible_lines = max_height;
    app.update_scroll();
    app.update_debug_info();

    // Render debug info
    let debug_area = Rect::new(area.x, area.bottom() - 1, area.width, 1);
    let debug_text = Paragraph::new(app.debug_info.clone()).style(Style::default().fg(Color::Gray));
    f.render_widget(debug_text, debug_area);
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

    f.set_cursor(inner_area.x + cursor_x, inner_area.y + cursor_y);
}
