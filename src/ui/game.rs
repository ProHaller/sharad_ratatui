use crate::app::App;
use crate::character::CharacterSheet;
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

    draw_game_content(f, app, chunks[0]);
    draw_user_input(f, app, chunks[1]);
}

fn draw_character_sheet(f: &mut Frame, sheet: &CharacterSheet, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Name, Race, Gender
            Constraint::Length(10), // Attributes
            Constraint::Length(2),  // Skills header
            Constraint::Min(0),     // Skills
            Constraint::Length(3),  // Qualities
        ])
        .split(area);

    // Basic Info
    let basic_info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Name: ", Style::default().fg(Color::Yellow)),
            Span::raw(&sheet.name),
        ]),
        Line::from(vec![
            Span::styled("Race: ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{:?}", sheet.race)),
        ]),
        Line::from(vec![
            Span::styled("Gender: ", Style::default().fg(Color::Yellow)),
            Span::raw(&sheet.gender),
        ]),
    ]);
    f.render_widget(basic_info, chunks[0]);

    // Attributes
    let attributes = vec![
        ("Body", sheet.body),
        ("Agility", sheet.agility),
        ("Reaction", sheet.reaction),
        ("Strength", sheet.strength),
        ("Willpower", sheet.willpower),
        ("Logic", sheet.logic),
        ("Intuition", sheet.intuition),
        ("Charisma", sheet.charisma),
        ("Edge", sheet.edge),
        ("Magic", sheet.magic.unwrap_or(0)),
        ("Resonance", sheet.resonance.unwrap_or(0)),
    ];

    let attribute_rows: Vec<Row> = attributes
        .iter()
        .map(|(name, value)| {
            Row::new(vec![
                Cell::from(*name),
                Cell::from(value.to_string()).style(Style::default().fg(Color::Yellow)),
            ])
        })
        .collect();

    let attribute_widths = vec![Constraint::Percentage(100), Constraint::Percentage(100)];

    let attribute_table = Table::new(attribute_rows, attribute_widths)
        .header(Row::new(vec!["Attribute", "Value"]).style(Style::default().fg(Color::Cyan)))
        .widths(&[Constraint::Percentage(50), Constraint::Percentage(50)])
        .column_spacing(1);

    f.render_widget(attribute_table, chunks[1]);

    // Skills header
    let skills_header = Paragraph::new("Skills")
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);
    f.render_widget(skills_header, chunks[2]);

    // Skills
    let mut skill_lines = Vec::new();
    for (category, skills) in [
        ("Combat", &sheet.skills.combat),
        ("Physical", &sheet.skills.physical),
        ("Social", &sheet.skills.social),
        ("Technical", &sheet.skills.technical),
    ] {
        skill_lines.push(Line::from(Span::styled(
            category,
            Style::default().fg(Color::Yellow),
        )));
        for (skill, rating) in skills {
            skill_lines.push(Line::from(vec![
                Span::raw(format!("  {}: ", skill)),
                Span::styled(rating.to_string(), Style::default().fg(Color::Green)),
            ]));
        }
    }

    let skills_paragraph = Paragraph::new(skill_lines).wrap(Wrap { trim: true });
    f.render_widget(skills_paragraph, chunks[3]);

    // Qualities
    let qualities: Vec<Line> = sheet
        .qualities
        .iter()
        .map(|q| {
            Line::from(vec![
                Span::styled(
                    if q.positive { "+" } else { "-" },
                    Style::default().fg(if q.positive { Color::Green } else { Color::Red }),
                ),
                Span::raw(format!(" {}", q.name)),
            ])
        })
        .collect();

    let qualities_paragraph = Paragraph::new(qualities).wrap(Wrap { trim: true });
    f.render_widget(qualities_paragraph, chunks[4]);
}

pub fn draw_game_content(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    let narration_block = Block::default()
        .title("Narration")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let game_info_block = Block::default()
        .title("Character Sheet")
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

    if let Some(game_state) = &app.current_game {
        if let Some(sheet) = &game_state.character_sheet {
            draw_character_sheet(f, sheet, game_info_area);
        } else {
            let no_character = Paragraph::new("No character sheet available.".to_string())
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center);
            f.render_widget(no_character, game_info_area);
        }
    } else {
        app.add_debug_message("No active game".to_string());
        let no_game = Paragraph::new("No active game.")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);
        f.render_widget(no_game, game_info_area);
    }

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
