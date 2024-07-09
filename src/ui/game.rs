use crate::app::App;
use crate::character::CharacterSheet;
use crate::message::{GameMessage, MessageType, UserMessage};
use hyphenation::{Language, Load, Standard};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::*,
    Frame,
};
use textwrap::{wrap, Options, WordSplitter};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

// In ui/game.rs

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

    // First, split the screen vertically into two parts: left (70%) and right (30%)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(size);

    // For the left part, split it vertically into game content (80%) and user input (20%)
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        .split(main_chunks[0]);

    draw_game_content(f, app, left_chunks[0]);
    draw_user_input(f, app, left_chunks[1]);

    // The right part is entirely for the game info (character sheet)
    let game_info_area = main_chunks[1];

    // Draw game info (character sheet)
    if let Some(game_state) = &app.current_game {
        if let Some(sheet) = &game_state.character_sheet {
            draw_character_sheet(f, sheet, game_info_area);
        } else {
            let no_character = Paragraph::new("No character sheet available.")
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

    // Add this at the end of the function to display debug info
    if app.settings.debug_mode {
        let debug_area = Rect::new(size.x, size.bottom() - 1, size.width, 1);
        let debug_text =
            Paragraph::new(app.debug_info.clone()).style(Style::default().fg(Color::Gray));
        f.render_widget(debug_text, debug_area);
    }
}

fn draw_character_sheet(f: &mut Frame, sheet: &CharacterSheet, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Basic Information
            Constraint::Length(14), // Attributes and Derived Attributes
            Constraint::Min(0),     // Skills, Qualities, and Other Info
        ])
        .split(area);

    draw_basic_info(f, sheet, chunks[0]);
    draw_attributes_and_derived(f, sheet, chunks[1]);
    draw_skills_qualities_and_other(f, sheet, chunks[2]);
}

fn draw_basic_info(f: &mut Frame, sheet: &CharacterSheet, area: Rect) {
    let info = vec![
        Span::styled(
            "Name: ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(&sheet.name),
        Span::raw(" | "),
        Span::styled(
            "Race: ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("{}", sheet.race)),
        Span::raw(" | "),
        Span::styled(
            "Gender: ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(&sheet.gender),
    ];
    let basic_info = Paragraph::new(Line::from(info))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Basic Information"),
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(basic_info, area);
}

fn draw_attributes_and_derived(f: &mut Frame, sheet: &CharacterSheet, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    draw_attributes(f, sheet, chunks[0]);
    draw_derived_attributes(f, sheet, chunks[1]);
}

fn draw_attributes(f: &mut Frame, sheet: &CharacterSheet, area: Rect) {
    let attributes = vec![
        ("BOD", sheet.body),
        ("AGI", sheet.agility),
        ("REA", sheet.reaction),
        ("STR", sheet.strength),
        ("WIL", sheet.willpower),
        ("LOG", sheet.logic),
        ("INT", sheet.intuition),
        ("CHA", sheet.charisma),
        ("EDG", sheet.edge),
        ("MAG", sheet.magic.unwrap_or(0)),
        ("RES", sheet.resonance.unwrap_or(0)),
    ];

    let rows: Vec<Row> = attributes
        .chunks(4)
        .map(|chunk| {
            Row::new(chunk.iter().map(|(name, value)| {
                Cell::from(Span::styled(
                    format!("{}: {}", name, value),
                    Style::default().fg(Color::Green),
                ))
            }))
        })
        .collect();

    let table = Table::new(rows, vec![Constraint::Percentage(25); 4])
        .block(Block::default().borders(Borders::ALL).title("Attributes"))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .column_spacing(1);

    f.render_widget(table, area);
}

fn draw_derived_attributes(f: &mut Frame, sheet: &CharacterSheet, area: Rect) {
    let derived = vec![
        format!(
            "Initiative: {}+{}d6",
            sheet.initiative.0, sheet.initiative.1
        ),
        format!(
            "Limits: PHY:{} MEN:{} SOC:{}",
            sheet.physical_limit, sheet.mental_limit, sheet.social_limit
        ),
        format!(
            "Monitors: PHY:{} SOC:{}",
            sheet.physical_monitor, sheet.stun_monitor
        ),
        format!("Essence: {:.2}", sheet.essence),
        format!("Edge Points: {}", sheet.edge_points),
        format!("Armor: {}", sheet.armor),
    ];

    let rows: Vec<Row> = derived
        .chunks(2)
        .map(|chunk| {
            Row::new(
                chunk
                    .iter()
                    .map(|attr| Cell::from(Span::styled(attr, Style::default().fg(Color::Cyan)))),
            )
        })
        .collect();

    let table = Table::new(rows, vec![Constraint::Percentage(50); 2])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Derived Attributes"),
        )
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .column_spacing(1);

    f.render_widget(table, area);
}

fn draw_skills_qualities_and_other(f: &mut Frame, sheet: &CharacterSheet, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Skills
            Constraint::Length(5),  // Qualities
            Constraint::Min(0),     // Other Info
        ])
        .split(area);

    draw_skills(f, sheet, chunks[0]);
    draw_qualities(f, sheet, chunks[1]);
    draw_other_info(f, sheet, chunks[2]);
}

fn draw_skills(f: &mut Frame, sheet: &CharacterSheet, area: Rect) {
    let categories = [
        ("Combat", &sheet.skills.combat),
        ("Physical", &sheet.skills.physical),
        ("Social", &sheet.skills.social),
        ("Technical", &sheet.skills.technical),
    ];

    let rows: Vec<Row> = categories
        .iter()
        .map(|(category, skills)| {
            let skills_str = skills
                .iter()
                .map(|(skill, rating)| format!("{}:{}", skill, rating))
                .collect::<Vec<_>>()
                .join(", ");
            Row::new(vec![
                Cell::from(Span::styled(
                    *category,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(skills_str),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        vec![Constraint::Percentage(20), Constraint::Percentage(80)],
    )
    .block(Block::default().borders(Borders::ALL).title("Skills"))
    .style(Style::default().fg(Color::White))
    .highlight_style(Style::default().add_modifier(Modifier::BOLD))
    .column_spacing(1);

    f.render_widget(table, area);
}

fn draw_qualities(f: &mut Frame, sheet: &CharacterSheet, area: Rect) {
    let qualities: Vec<Span> = sheet
        .qualities
        .iter()
        .map(|q| {
            let color = if q.positive { Color::Green } else { Color::Red };
            Span::styled(format!("{}, ", q.name), Style::default().fg(color))
        })
        .collect();

    let qualities_paragraph = Paragraph::new(Line::from(qualities))
        .block(Block::default().borders(Borders::ALL).title("Qualities"))
        .wrap(Wrap { trim: true });
    f.render_widget(qualities_paragraph, area);
}

fn draw_other_info(f: &mut Frame, sheet: &CharacterSheet, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let left_info = vec![
        format!("Nuyen: {}", sheet.nuyen),
        format!("Lifestyle: {}", sheet.lifestyle),
        format!("Contacts: {}", sheet.contacts.len()),
    ];

    let right_info = vec![
        format!("Cyberware: {}", sheet.cyberware.len()),
        format!("Bioware: {}", sheet.bioware.len()),
        format!("Inventory: {}", sheet.inventory.len()),
    ];

    let left_table = create_table(&left_info, "Resources & Contacts");
    let right_table = create_table(&right_info, "Augmentations & Inventory");

    f.render_widget(left_table, chunks[0]);
    f.render_widget(right_table, chunks[1]);
}

fn create_table<'a>(info: &'a [String], title: &'a str) -> Table<'a> {
    let rows: Vec<Row> = info
        .iter()
        .map(|item| {
            Row::new(vec![Cell::from(Span::styled(
                item.to_string(),
                Style::default().fg(Color::White),
            ))])
        })
        .collect();

    let widths = vec![Constraint::Percentage(100)];

    Table::new(rows, widths)
        .block(Block::default().borders(Borders::ALL).title(title))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">>")
        .column_spacing(1)
}

pub fn draw_game_content(f: &mut Frame, app: &mut App, area: Rect) {
    let narration_block = Block::default()
        .title("Narration")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    f.render_widget(&narration_block, area);

    let narration_area = narration_block.inner(area);

    let max_width = narration_area.width.saturating_sub(2) as usize;
    let max_height = narration_area.height.saturating_sub(2) as usize;

    let mut all_lines = Vec::new();

    for message in &app.game_content {
        let (content, base_style, alignment) = match message.message_type {
            MessageType::Game => {
                if let Ok(game_message) = serde_json::from_str::<GameMessage>(&message.content) {
                    (
                        format!(
                            "Reasoning:\n{}\n\nNarration:\n{}",
                            game_message.reasoning, game_message.narration
                        ),
                        Style::default().fg(Color::Green),
                        Alignment::Left,
                    )
                } else {
                    (
                        message.content.clone(),
                        Style::default().fg(Color::Green),
                        Alignment::Left,
                    )
                }
            }
            MessageType::User => {
                if let Ok(user_message) = serde_json::from_str::<UserMessage>(&message.content) {
                    (
                        format!("\nPlayer action:\n{}", user_message.player_action),
                        Style::default().fg(Color::Blue),
                        Alignment::Right,
                    )
                } else {
                    (
                        message.content.clone(),
                        Style::default().fg(Color::Blue),
                        Alignment::Right,
                    )
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
            let parsed_line = parse_markdown(line.to_string(), base_style);
            all_lines.push((parsed_line, alignment));
        }
    }

    app.total_lines = all_lines.len();
    app.debug_info += &format!(", Total lines: {}", app.total_lines);

    let visible_lines: Vec<Line> = all_lines
        .iter()
        .skip(app.game_content_scroll)
        .take(max_height)
        .map(|(line, alignment)| match alignment {
            Alignment::Right => line.clone().alignment(Alignment::Right),
            Alignment::Left => line.clone(),
            Alignment::Center => line.clone().alignment(Alignment::Center),
        })
        .collect();

    app.debug_info += &format!(", Visible lines: {}", visible_lines.len());

    let content = Paragraph::new(visible_lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: true });

    f.render_widget(content, narration_area);

    app.visible_lines = max_height;
    app.update_scroll();
    app.update_debug_info();
}

fn draw_user_input(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title("Your Action")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let max_width = inner_area.width as usize - 2;

    // Use Rope for handling the text buffer
    let rope = &app.user_input;
    let text = rope.to_string();

    // Load hyphenation dictionary
    let dictionary = Standard::from_embedded(Language::EnglishUS).unwrap();

    // Configure textwrap options
    let options = Options::new(max_width)
        .word_splitter(WordSplitter::Hyphenation(dictionary))
        .break_words(true);

    // Wrap the input text
    let wrapped_lines: Vec<String> = wrap(&text, &options)
        .into_iter()
        .map(|s| s.into_owned())
        .collect();

    // Calculate cursor position
    let mut cursor_x = 0;
    let mut cursor_y = 0;
    let mut chars_processed = 0;

    for (line_idx, line) in wrapped_lines.iter().enumerate() {
        let line_graphemes: Vec<&str> = line.graphemes(true).collect();
        let line_width: usize = line_graphemes.iter().map(|g| g.width()).sum();

        if chars_processed + line_graphemes.len() >= app.cursor_position {
            cursor_y = line_idx;
            let prefix_graphemes = &line_graphemes[..app.cursor_position - chars_processed];
            cursor_x = prefix_graphemes.iter().map(|g| g.width()).sum();

            // Count trailing spaces
            let trailing_spaces = text[chars_processed + prefix_graphemes.len()..]
                .chars()
                .take_while(|&c| c == ' ')
                .count();
            cursor_x += trailing_spaces;

            break;
        }

        chars_processed += line_graphemes.len();
        if chars_processed < text.len() && text.as_bytes()[chars_processed] == b'\n' {
            chars_processed += 1;
        }
    }

    // Handle cursor at the end of the text
    if app.cursor_position == text.len() {
        cursor_y = wrapped_lines.len() - 1;
        cursor_x = wrapped_lines
            .last()
            .map(|line| line.graphemes(true).map(|g| g.width()).sum())
            .unwrap_or(0);

        // Add trailing spaces at the end of the text
        let trailing_spaces = text.chars().rev().take_while(|&c| c == ' ').count();
        cursor_x += trailing_spaces;
    }

    let joined_lines = wrapped_lines.join("\n");

    let input = Paragraph::new(joined_lines)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });

    f.render_widget(input, inner_area);

    // Adjust cursor position if it's beyond the visible area
    let visible_lines = inner_area.height as usize - 1;
    if cursor_y >= visible_lines {
        cursor_y = visible_lines - 1;
    }

    // Ensure cursor_x doesn't exceed the max width
    cursor_x = cursor_x.min(max_width);

    // Set cursor
    f.set_cursor(
        inner_area.x + cursor_x as u16,
        inner_area.y + cursor_y as u16,
    );
}

fn parse_markdown<'a>(line: String, base_style: Style) -> Line<'a> {
    let mut spans = Vec::new();
    let mut current_text = String::new();
    let mut in_bold = false;

    for word in line.split_whitespace() {
        if word.starts_with("###") {
            if !current_text.is_empty() {
                spans.push(Span::styled(current_text.clone(), base_style));
                current_text.clear();
            }
            let header_text = word[3..].to_uppercase();
            spans.push(Span::styled(
                header_text,
                base_style.add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            ));
        } else if word.starts_with("**") && word.ends_with("**") && word.len() > 4 {
            if !current_text.is_empty() {
                spans.push(Span::styled(current_text.clone(), base_style));
                current_text.clear();
            }
            let bold_text = word[2..word.len() - 2].to_string();
            spans.push(Span::styled(
                bold_text,
                base_style.add_modifier(Modifier::BOLD),
            ));
        } else if word.contains("**") {
            let parts: Vec<&str> = word.split("**").collect();
            for (i, part) in parts.iter().enumerate() {
                if !part.is_empty() {
                    if in_bold {
                        spans.push(Span::styled(
                            part.to_string(),
                            base_style.add_modifier(Modifier::BOLD),
                        ));
                    } else {
                        current_text.push_str(part);
                    }
                }
                if i < parts.len() - 1 {
                    in_bold = !in_bold;
                }
            }
        } else if in_bold {
            spans.push(Span::styled(
                word.to_string(),
                base_style.add_modifier(Modifier::BOLD),
            ));
        } else {
            current_text.push_str(word);
            current_text.push(' ');
        }
    }

    if !current_text.is_empty() {
        spans.push(Span::styled(current_text.trim().to_string(), base_style));
    }

    Line::from(spans)
}
