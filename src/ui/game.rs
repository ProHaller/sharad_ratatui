use crate::app::App;
use crate::message::MessageType;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::*,
    Frame,
};
use textwrap::{core::display_width, wrap};
use unicode_segmentation::UnicodeSegmentation;

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

    if app.is_character_creation {
        draw_character_creation(f, app, chunks[0]);
    } else {
        draw_game_content(f, app, chunks[0]);
    }
    draw_user_input(f, app, chunks[1]);
}

fn draw_character_creation(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    draw_character_sheet(f, app, chunks[0]);
}

fn draw_character_sheet(f: &mut Frame, app: &App, area: Rect) {
    if let Some(sheet) = &app.character_sheet {
        let attributes = vec![
            ("Name", sheet.name.clone()),
            ("Race", format!("{:?}", sheet.race)),
            ("Gender", sheet.gender.clone()),
            ("Body", sheet.body.to_string()),
            ("Agility", sheet.agility.to_string()),
            ("Reaction", sheet.reaction.to_string()),
            ("Strength", sheet.strength.to_string()),
            ("Willpower", sheet.willpower.to_string()),
            ("Logic", sheet.logic.to_string()),
            ("Intuition", sheet.intuition.to_string()),
            ("Charisma", sheet.charisma.to_string()),
            ("Edge", sheet.edge.to_string()),
            ("Magic", sheet.magic.unwrap_or(0).to_string()),
            ("Resonance", sheet.resonance.unwrap_or(0).to_string()),
        ];

        let header = Row::new(vec!["Attribute", "Value"]).style(Style::default().fg(Color::Cyan));

        let rows: Vec<Row> = attributes
            .into_iter()
            .map(|(name, value)| {
                Row::new(vec![
                    Cell::from(name),
                    Cell::from(value).style(Style::default().fg(Color::Yellow)),
                ])
            })
            .collect();

        let widths = vec![Constraint::Percentage(50), Constraint::Percentage(50)];

        let table = Table::new(rows, widths).header(header).block(
            Block::default()
                .title("Character Sheet")
                .borders(Borders::ALL),
        );

        f.render_widget(table, area);
    }
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

    for message in &app.game_content {
        let (content, style, alignment) = match message.message_type {
            MessageType::User => {
                if let Some(user_message) = message.parse_user_message() {
                    (
                        user_message.player_action,
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
                        game_message.narration,
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
    app.visible_lines = max_height;
    app.update_scroll();
    app.update_debug_info();

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

    let max_width = inner_area.width as usize - 2;

    // Wrap the input text
    let wrapped_input = wrap(&app.user_input, max_width);

    let input = Paragraph::new(wrapped_input.join("\n"))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    f.render_widget(input, inner_area);

    // Calculate cursor position
    let mut current_line = 0;
    let mut current_column = 0;
    let mut last_word_start = 0;

    for (chars_processed, (_i, grapheme)) in app.user_input.graphemes(true).enumerate().enumerate()
    {
        if chars_processed == app.cursor_position {
            break;
        }

        let grapheme_width = display_width(grapheme);

        if grapheme == " " {
            last_word_start = current_column + grapheme_width.saturating_sub(1);
        }

        if current_column + grapheme_width > max_width {
            current_line += 1;
            if last_word_start > 0 {
                current_column -= last_word_start;
            } else {
                current_column = 0;
            }
            last_word_start = 0;
        } else {
            current_column += grapheme_width;
        }
    }

    // Set cursor
    f.set_cursor(
        inner_area.x + current_column as u16,
        inner_area.y + current_line as u16,
    );
}
