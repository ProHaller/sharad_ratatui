use crate::app::{App, InputMode};
use crate::character::CharacterSheet;
use crate::message::{GameMessage, MessageType, UserMessage};
use crate::ui::utils::spinner_frame;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::*,
    Frame,
};
use std::cell::RefCell;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

thread_local! {
    static CACHED_LAYOUTS: RefCell<Option<(Rect, Vec<Rect>, Vec<Rect>)>> = RefCell::new(None);
}

pub fn draw_in_game(f: &mut Frame, app: &mut App) {
    let size = f.size();
    *app.debug_info.borrow_mut() = format!("Terminal size: {}x{}", size.width, size.height);

    if size.width < 101 || size.height < 50 {
        let warning = Paragraph::new("Terminal too small. Please resize.")
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center);
        f.render_widget(warning, size);
        return;
    }

    let (_main_chunk, left_chunk, game_info_area) = CACHED_LAYOUTS.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.as_ref().map_or(true, |&(area, _, _)| area != size) {
            let main_chunk = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(size);

            let left_chunk = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
                .split(main_chunk[0]);

            let new_cache = (size, main_chunk.to_vec(), left_chunk.to_vec());
            *cache = Some(new_cache);
        }

        let (_, ref main_chunks, ref left_chunks) = cache.as_ref().unwrap();
        (main_chunks.clone(), left_chunks.clone(), main_chunks[1])
    });
    draw_game_content(f, app, left_chunk[0]);

    draw_user_input(f, app, left_chunk[1]);
    app.update_spinner();
    if app.spinner_active {
        let spinner_area = Rect::new(
            left_chunk[0].x,
            left_chunk[0].bottom() - 1,
            left_chunk[0].width,
            1,
        );

        let spinner_text = spinner_frame(&app.spinner);
        let spinner_widget = Paragraph::new(spinner_text)
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);

        f.render_widget(spinner_widget, spinner_area);
    }

    if let Some(game_state) = &app.current_game {
        match game_state.try_lock() {
            Ok(locked_game_state) => {
                if let Some(sheet) = &locked_game_state.character_sheet {
                    // Update the last known character sheet
                    app.last_known_character_sheet = Some(sheet.clone());
                    draw_character_sheet(f, sheet, game_info_area);
                } else {
                    app.last_known_character_sheet = None;
                    let no_character = Paragraph::new("No character sheet available.")
                        .style(Style::default().fg(Color::Yellow))
                        .alignment(Alignment::Center);
                    f.render_widget(no_character, game_info_area);
                }
            }
            Err(_) => {
                // If we can't get the lock, use the last known character sheet
                if let Some(last_sheet) = &app.last_known_character_sheet {
                    draw_character_sheet(f, last_sheet, game_info_area);
                } else {
                    let no_character = Paragraph::new("No character sheet available.")
                        .style(Style::default().fg(Color::Yellow))
                        .alignment(Alignment::Center);
                    f.render_widget(no_character, game_info_area);
                }
            }
        }
    } else {
        app.add_debug_message("No active game".to_string());
        let no_game = Paragraph::new("No active game.")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);
        f.render_widget(no_game, game_info_area);
    }

    if app.settings.debug_mode {
        let debug_area = Rect::new(size.x, size.bottom() - 1, size.width, 1);
        let debug_text =
            Paragraph::new(app.debug_info.borrow().clone()).style(Style::default().fg(Color::Gray));
        f.render_widget(debug_text, debug_area);
    }
}

// Function to draw the character sheet.
fn draw_character_sheet(f: &mut Frame, sheet: &CharacterSheet, area: Rect) {
    // Layout for different sections of the character sheet.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Basic Information
            Constraint::Length(14), // Attributes and Derived Attributes
            Constraint::Min(0),     // Skills, Qualities, and Other Info
        ])
        .split(area);

    // Drawing individual sections of the character sheet.
    draw_basic_info(f, sheet, chunks[0]);
    draw_attributes_and_derived(f, sheet, chunks[1]);
    draw_skills_qualities_and_other(f, sheet, chunks[2]);
}

// Display basic information like name, race, and gender.
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

// Display attributes and derived attributes.
fn draw_attributes_and_derived(f: &mut Frame, sheet: &CharacterSheet, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    draw_attributes(f, sheet, chunks[0]);
    draw_derived_attributes(f, sheet, chunks[1]);
}

// Display specific attributes.
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

// Display derived attributes like initiative and limits.
fn draw_derived_attributes(f: &mut Frame, sheet: &CharacterSheet, area: Rect) {
    let derived = [
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

// Display skills, qualities, and other relevant information.
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

// Specific function to handle the display of skills.
fn draw_skills(f: &mut Frame, sheet: &CharacterSheet, area: Rect) {
    let categories = [
        ("Combat", &sheet.skills.combat),
        ("Physical", &sheet.skills.physical),
        ("Social", &sheet.skills.social),
        ("Technical", &sheet.skills.technical),
        ("Knowledge", &sheet.knowledge_skills),
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

// Function to handle the display of qualities.
fn draw_qualities(f: &mut Frame, sheet: &CharacterSheet, area: Rect) {
    let qualities: Vec<Span> = sheet
        .qualities
        .iter()
        .enumerate()
        .map(|(i, q)| {
            let color = if q.positive { Color::Green } else { Color::Red };
            let separator = if i == sheet.qualities.len() - 1 {
                ""
            } else {
                ", "
            };
            Span::styled(
                format!("{}{}", q.name, separator),
                Style::default().fg(color),
            )
        })
        .collect();

    let qualities_paragraph = Paragraph::new(Line::from(qualities))
        .block(Block::default().borders(Borders::ALL).title("Qualities"))
        .wrap(Wrap { trim: true });
    f.render_widget(qualities_paragraph, area);
}

// Function to display miscellaneous information.

fn draw_other_info(f: &mut Frame, sheet: &CharacterSheet, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Left column: Resources and Augmentations
            Constraint::Percentage(60), // Right column: Contacts and Inventory
        ])
        .split(area);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30), // Resources
            Constraint::Percentage(70), // Augmentations
        ])
        .split(chunks[0]);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // Contacts
            Constraint::Percentage(50), // Inventory
        ])
        .split(chunks[1]);

    draw_resources(f, sheet, left_chunks[0]);
    draw_augmentations(f, sheet, right_chunks[0]);
    draw_contacts(f, sheet, right_chunks[1]);
    draw_inventory(f, sheet, left_chunks[1]);
}

fn draw_resources(f: &mut Frame, sheet: &CharacterSheet, area: Rect) {
    let info = vec![
        format!("Lifestyle: {}", sheet.lifestyle),
        format!("Nuyen: {}", sheet.nuyen),
    ];

    let resources_table = create_table(&info, "Resources");
    f.render_widget(resources_table, area);
}

fn draw_augmentations(f: &mut Frame, sheet: &CharacterSheet, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let cyberware_elements: Vec<Line> = sheet
        .cyberware
        .iter()
        .map(|cw| Line::from(Span::styled(cw.clone(), Style::default().fg(Color::White))))
        .collect();

    let bioware_elements: Vec<Line> = sheet
        .bioware
        .iter()
        .map(|bw| Line::from(Span::styled(bw.clone(), Style::default().fg(Color::White))))
        .collect();

    let cyberware_paragraph = Paragraph::new(cyberware_elements)
        .block(Block::default().borders(Borders::ALL).title("Cyberware"))
        .wrap(Wrap { trim: true });

    let bioware_paragraph = Paragraph::new(bioware_elements)
        .block(Block::default().borders(Borders::ALL).title("Bioware"))
        .wrap(Wrap { trim: true });

    f.render_widget(cyberware_paragraph, chunks[0]);
    f.render_widget(bioware_paragraph, chunks[1]);
}

fn draw_contacts(f: &mut Frame, sheet: &CharacterSheet, area: Rect) {
    let header_cells = ["Name", "Loyalty", "Connection"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow)));
    let header = Row::new(header_cells)
        .style(Style::default())
        .height(1)
        .bottom_margin(0);

    let rows: Vec<Row> = sheet
        .contacts
        .iter()
        .map(|(name, contact)| {
            let cells = vec![
                Cell::from(if name.split_whitespace().next().unwrap().len() > 3 {
                    name.split_whitespace().next().unwrap()
                } else {
                    name
                }),
                Cell::from(contact.loyalty.to_string()),
                Cell::from(contact.connection.to_string()),
            ];
            Row::new(cells).height(1).bottom_margin(0)
        })
        .collect();

    let widths = vec![
        Constraint::Percentage(30),
        Constraint::Percentage(30),
        Constraint::Percentage(40),
    ];
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("Contacts"));

    f.render_widget(table, area);
}

fn draw_inventory(f: &mut Frame, sheet: &CharacterSheet, area: Rect) {
    let inventory_items: Vec<Row> = sheet
        .inventory
        .values()
        .map(|item| {
            Row::new(vec![Cell::from(format!(
                "{} (x{})",
                item.name, item.quantity,
            ))])
        })
        .collect();

    let widths = vec![Constraint::Percentage(100)];
    let inventory_table = Table::new(inventory_items, widths)
        .block(Block::default().borders(Borders::ALL).title("Inventory"))
        .widths([Constraint::Percentage(100)])
        .column_spacing(1);

    f.render_widget(inventory_table, area);
}

// Helper function to create a styled table from given information.
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
    let save_name = app
        .current_save_name
        .try_read()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| String::from("Loading..."));

    let fluff_block = Block::default()
        .title(if save_name.is_empty() {
            "Game will start momentarily".to_string()
        } else {
            save_name
        })
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    f.render_widget(&fluff_block, area);

    let fluff_area = fluff_block.inner(area);

    let max_width = fluff_area.width.saturating_sub(2) as usize;
    let max_height = fluff_area.height.saturating_sub(2) as usize;

    if app.cached_game_content.is_none()
        || app.cached_content_len != app.game_content.borrow().len()
    {
        app.update_cached_content(max_width);
    }

    let all_lines = app.cached_game_content.as_ref().unwrap();

    app.total_lines = all_lines.len();
    *app.debug_info.borrow_mut() += &format!(", Total lines: {}", app.total_lines);

    let visible_lines: Vec<Line> = all_lines
        .iter()
        .skip(app.game_content_scroll)
        .take(max_height)
        .map(|(line, alignment)| {
            let mut new_line = line.clone();
            new_line.alignment = Some(*alignment);
            new_line
        })
        .collect();

    *app.debug_info.borrow_mut() += &format!(", Visible lines: {}", visible_lines.len());

    let content = Paragraph::new(visible_lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: true });

    f.render_widget(content, fluff_area);

    app.visible_lines = max_height;
    app.update_scroll();
    app.update_debug_info();
}

pub fn parse_game_content(app: &App, max_width: usize) -> Vec<(Line<'static>, Alignment)> {
    let mut all_lines = Vec::new();

    for message in app.game_content.borrow().iter() {
        let (content, base_style, alignment) = match message.message_type {
            MessageType::Game => {
                if let Ok(game_message) = serde_json::from_str::<GameMessage>(&message.content) {
                    (
                        format!(
                            "crunch:\n{}\n\nfluff:\n{}",
                            game_message.crunch, game_message.fluff
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
                        Style::default().fg(Color::Cyan),
                        Alignment::Right,
                    )
                } else {
                    (
                        message.content.clone(),
                        Style::default().fg(Color::Cyan),
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

    all_lines
}

pub fn draw_user_input(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(match app.input_mode {
            InputMode::Normal => " Press 'e' to edit or 'r' to record ",
            InputMode::Editing => " Editing ",
            InputMode::Recording => " Recording… ",
        })
        .borders(Borders::ALL)
        .border_style(Style::default().fg(match app.input_mode {
            InputMode::Normal => Color::DarkGray,
            InputMode::Editing => Color::White,
            InputMode::Recording => Color::Red,
        }));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let max_width = inner_area.width as usize - 2;

    let text = app.user_input.value();

    // Wrap the text manually, considering grapheme clusters and their widths
    let mut wrapped_lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for grapheme in text.graphemes(true) {
        let grapheme_width = grapheme.width();
        if current_width + grapheme_width > max_width {
            wrapped_lines.push(current_line);
            current_line = String::new();
            current_width = 0;
        }
        current_line.push_str(grapheme);
        current_width += grapheme_width;
    }
    if !current_line.is_empty() {
        wrapped_lines.push(current_line);
    }

    // Calculate cursor position
    let cursor_position = app.user_input.visual_cursor();
    let mut cursor_x = 0;
    let mut cursor_y = 0;
    let mut total_width = 0;

    for (line_idx, line) in wrapped_lines.iter().enumerate() {
        let line_width: usize = line.width();
        if total_width + line_width >= cursor_position {
            cursor_y = line_idx;
            cursor_x = cursor_position - total_width;
            break;
        }
        total_width += line_width;
        cursor_y = line_idx + 1;
    }

    // Ensure cursor_x doesn't exceed the line width
    if cursor_y < wrapped_lines.len() {
        cursor_x = cursor_x.min(wrapped_lines[cursor_y].width());
    }

    let joined_lines = wrapped_lines.join("\n");

    let input = Paragraph::new(joined_lines)
        .style(Style::default().fg(match app.input_mode {
            InputMode::Normal => Color::DarkGray,
            InputMode::Editing => Color::Yellow,
            InputMode::Recording => Color::Red,
        }))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });

    f.render_widget(input, inner_area);

    // Adjust cursor position if it's beyond the visible area
    let visible_lines = inner_area.height.saturating_sub(1) as usize;
    if cursor_y >= visible_lines {
        cursor_y = visible_lines.saturating_sub(1);
    }

    // Set cursor
    if let InputMode::Editing = app.input_mode {
        f.set_cursor(
            inner_area.x + cursor_x as u16,
            inner_area.y + cursor_y as u16,
        );
    }
}

// Function to parse markdown-like text to formatted spans.

fn parse_markdown(line: String, base_style: Style) -> Line<'static> {
    let mut spans = Vec::new();
    let mut current_text = String::new();
    let mut in_bold = false;
    let mut in_list = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '*' {
            if chars.peek() == Some(&'*') {
                chars.next(); // consume the second '*'
                if in_bold {
                    if !current_text.is_empty() {
                        spans.push(Span::styled(
                            current_text.clone(),
                            base_style.add_modifier(Modifier::BOLD),
                        ));
                        current_text.clear();
                    }
                } else if !current_text.is_empty() {
                    spans.push(Span::styled(current_text.clone(), base_style));
                    current_text.clear();
                }
                in_bold = !in_bold;
            } else {
                current_text.push(ch);
            }
        } else if ch == '#' {
            let mut header_level = 1;
            while chars.peek() == Some(&'#') {
                header_level += 1;
                chars.next(); // consume additional '#'
            }
            if header_level == 3 {
                if !current_text.is_empty() {
                    spans.push(Span::styled(current_text.clone(), base_style));
                    current_text.clear();
                }
                while chars.peek() == Some(&' ') {
                    chars.next(); // consume spaces after ###
                }
                let header_text: String = chars.by_ref().take_while(|&c| c != ' ').collect();
                spans.push(Span::styled(
                    header_text.to_uppercase(),
                    base_style.add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ));
            } else {
                current_text.push('#');
                for _ in 1..header_level {
                    current_text.push('#');
                }
            }
        } else if ch == '-' && chars.peek() == Some(&' ') {
            if !current_text.is_empty() {
                spans.push(Span::styled(current_text.clone(), base_style));
                current_text.clear();
            }
            in_list = true;
            current_text.push(ch);
        } else if ch == '\n' {
            if in_list {
                spans.push(Span::styled(current_text.clone(), base_style));
                current_text.clear();
                in_list = false;
            }
            current_text.push(ch);
        } else {
            current_text.push(ch);
        }
    }

    if !current_text.is_empty() {
        if in_bold {
            spans.push(Span::styled(
                current_text,
                base_style.add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(current_text, base_style));
        }
    }

    Line::from(spans)
}
