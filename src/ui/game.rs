use super::descriptions::*;
use super::draw::{MIN_HEIGHT, MIN_WIDTH};
use super::{chunk_attributes, draw_character_sheet, get_attributes, get_derived};
use crate::character::CharacterSheet;

use crate::{
    app::{App, InputMode},
    message::{GameMessage, MessageType, UserMessage},
    ui::spinner::spinner_frame,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Flex, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::*,
};
use ratatui_image::StatefulImage;
use std::cell::RefCell;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

type Cache = RefCell<Option<(Rect, Vec<Rect>, Vec<Rect>)>>;
thread_local! {
    static CACHED_LAYOUTS: Cache = const {RefCell::new(None)};
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HighlightedSection {
    None,
    Backstory,
    Attributes(usize),
    Derived(usize),
    Skills,
    Qualities,
    Inventory,
    Contact,
    Cyberware,
    Bioware,
    Resources,
}

pub fn draw_in_game(f: &mut Frame, app: &mut App) {
    let size = f.area();
    *app.debug_info.borrow_mut() = format!("Terminal size: {}x{}", size.width, size.height);

    if size.width < MIN_WIDTH || size.height < MIN_HEIGHT {
        let warning = Paragraph::new("Terminal too small. Please resize.")
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center);
        f.render_widget(warning, size);
        return;
    }

    let (_main_chunk, left_chunk, game_info_area) = CACHED_LAYOUTS.with(|cache: &Cache| {
        let mut cache = cache.borrow_mut();
        if cache.is_none() || cache.as_ref().expect("Expected a valide cache").0 != size {
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

        let (_, main_chunks, left_chunks) = cache.as_ref().expect("Expected a valide cache");
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
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center);

        f.render_widget(spinner_widget, spinner_area);
    }

    if let Some(game_state) = &app.current_game.clone() {
        match game_state.try_lock() {
            Ok(locked_game_state) => {
                if let Some(sheet) = &locked_game_state.main_character_sheet {
                    app.last_known_character_sheet = Some(sheet.clone());

                    draw_character_sheet(f, sheet, game_info_area, &app.highlighted_section);
                    draw_detailed_info(app, f, sheet, left_chunk[0]);
                } else {
                    app.last_known_character_sheet = None;
                    let center_rect = center_vertical(game_info_area, 5);
                    let center_block = Block::bordered();
                    let no_character = Paragraph::new("No character sheet available yet.")
                        .style(Style::default().fg(Color::Yellow))
                        .alignment(Alignment::Center)
                        .block(center_block.padding(Padding {
                            left: 0,
                            right: 0,
                            top: 1,
                            bottom: 0,
                        }));
                    f.render_widget(no_character, center_rect);
                }
            }
            Err(_) => {
                if let Some(last_sheet) = &app.last_known_character_sheet.clone() {
                    draw_character_sheet(f, last_sheet, game_info_area, &app.highlighted_section);
                    draw_detailed_info(app, f, last_sheet, left_chunk[0]);
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

    // Debug mode rendering
    if app.settings.debug_mode {
        let debug_area = Rect::new(size.x, size.bottom() - 1, size.width, 1);
        let debug_text =
            Paragraph::new(app.debug_info.borrow().clone()).style(Style::default().fg(Color::Gray));
        f.render_widget(debug_text, debug_area);
    }
}

pub fn draw_detailed_info(app: &mut App, f: &mut Frame, sheet: &CharacterSheet, area: Rect) {
    // Early return if HighlightedSection::None
    if matches!(&app.highlighted_section, HighlightedSection::None) {
        return;
    }

    let attributes = get_attributes(sheet);
    let detail_text = match &app.highlighted_section {
        HighlightedSection::Backstory => vec![Line::from(vec![Span::raw(&sheet.backstory)])],
        HighlightedSection::Inventory => sheet
            .inventory
            .values()
            .map(|item| {
                Line::from(vec![
                    Span::styled(&item.name, Style::default().fg(Color::Yellow)),
                    Span::raw(format!("(x{}): {} ", &item.quantity, &item.description)),
                ])
            })
            .collect::<Vec<_>>(),
        HighlightedSection::Contact => sheet
            .contacts
            .values()
            .flat_map(|contact| {
                vec![
                    Line::from(vec![Span::styled(
                        &contact.name,
                        Style::default().fg(Color::Yellow),
                    )]),
                    Line::from(vec![
                        Span::styled(
                            format!(" Loyalty: {} ", &contact.loyalty),
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("Connection: {} ", &contact.connection),
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(vec![Span::raw(&contact.description)]),
                ]
            })
            .collect::<Vec<_>>(),
        HighlightedSection::Cyberware => sheet
            .cyberware
            .iter()
            .flat_map(|cw| vec![Line::from(vec![Span::raw(cw)])])
            .collect::<Vec<_>>(),
        HighlightedSection::Bioware => sheet
            .bioware
            .iter()
            .flat_map(|bw| vec![Line::from(vec![Span::raw(bw)])])
            .collect::<Vec<_>>(),
        HighlightedSection::Resources => vec![
            Line::from(vec![
                Span::styled("Nuyen: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("¥{}", sheet.nuyen),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![Span::raw(NUYEN)]),
            Line::from(vec![
                Span::styled("Lifestyle: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    sheet.lifestyle.to_string(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![Span::raw(LIFESTYLE)]),
        ],
        HighlightedSection::Attributes(0) => chunk_attributes(attributes, 0),
        HighlightedSection::Attributes(1) => chunk_attributes(attributes, 1),
        HighlightedSection::Attributes(_) => chunk_attributes(attributes, 2),
        HighlightedSection::Derived(0) => get_derived(&sheet.derived_attributes, 0),
        HighlightedSection::Derived(_) => get_derived(&sheet.derived_attributes, 1),
        HighlightedSection::Skills => vec![Line::from(vec![
            Span::styled("Initiative: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                sheet.derived_attributes.initiative.0.to_string(),
                Style::default().fg(Color::White),
            ),
            Span::styled("+", Style::default().fg(Color::White)),
            Span::styled(
                sheet.derived_attributes.initiative.1.to_string(),
                Style::default().fg(Color::White),
            ),
            Span::styled("D6", Style::default().fg(Color::White)),
        ])],
        HighlightedSection::Qualities => vec![Line::from(vec![
            Span::styled("Initiative: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                sheet.derived_attributes.initiative.0.to_string(),
                Style::default().fg(Color::White),
            ),
            Span::styled("+", Style::default().fg(Color::White)),
            Span::styled(
                sheet.derived_attributes.initiative.1.to_string(),
                Style::default().fg(Color::White),
            ),
            Span::styled("D6", Style::default().fg(Color::White)),
        ])],

        HighlightedSection::None => unreachable!(),
    };

    // Calculate the size and position of the floating frame
    let width = (area.width - (f.area().width - 2) / 3).saturating_sub(4); // Minimum width of 20
    let height = f.area().height.saturating_sub(2);
    // let x = area.x + (area.width - width) / 2;
    let x = (f.area().width / 3) + 2;
    let y = 1;

    let details_area = Rect::new(x, y, width, height);

    // Create a block for the floating frame
    let block = Block::default()
        .border_type(BorderType::Rounded)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .title(match &app.highlighted_section {
            HighlightedSection::Backstory => " Backstory ",
            HighlightedSection::Inventory => " Inventory ",
            HighlightedSection::Contact => " Contact ",
            HighlightedSection::Cyberware => " Cyberware ",
            HighlightedSection::Bioware => " Bioware ",
            HighlightedSection::Attributes(0) => " Attributes 1/3 ",
            HighlightedSection::Attributes(1) => " Attributes 2/3 ",
            HighlightedSection::Attributes(_) => " Attributes 3/3 ",
            HighlightedSection::Derived(0) => " Derived Attributes 1/2",
            HighlightedSection::Derived(_) => " Derived Attributes 2/2",
            HighlightedSection::Skills => " Skills ",
            HighlightedSection::Qualities => " Qualities ",
            HighlightedSection::Resources => " Resources ",
            HighlightedSection::None => unreachable!(),
        })
        .style(Style::default()); // Make the block opaque

    // Render the block
    f.render_widget(Clear, details_area); // Clear the area behind the block
    f.render_widget(block.clone(), details_area);

    // Get the inner area of the block for the content
    let inner_area = block.inner(details_area);

    let detail_paragraph = Paragraph::new(detail_text) // Use
        // the wrapped text as the Paragraph detail_text)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    // Render the content inside the block
    if let Some(image) = app.image.as_mut() {
        let image_rect = Rect::new(1, 1, (f.area().width + 2) / 3, f.area().height - 2);
        let image_block = Block::default()
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL)
            .title(" Portrait ");

        f.render_widget(detail_paragraph, inner_area);
        f.render_widget(Clear, image_rect);
        f.render_widget(&image_block, image_rect);
        // FIX: How to make the first rendering faster? Pre-rendering?
        f.render_stateful_widget(StatefulImage::new(), image_block.inner(image_rect), image);
    } else {
        f.render_widget(detail_paragraph, inner_area);
    }
}

pub fn draw_game_content(f: &mut Frame, app: &mut App, area: Rect) {
    let save_name = app
        .save_manager
        .current_save
        .clone()
        .map_or_else(|| String::from("Loading..."), |save| save.save_name);

    let fluff_block = Block::default()
        .border_type(BorderType::Rounded)
        .title(if save_name.is_empty() {
            " Game will start momentarily ".to_string()
        } else {
            format!(" {} ", save_name)
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

    let all_lines = app
        .cached_game_content
        .as_ref()
        .expect("Expected a valid ref to a cached_game_content");

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
        .block(
            Block::default()
                .border_type(BorderType::Rounded)
                .borders(Borders::NONE),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(content, fluff_area);

    app.visible_lines = max_height;
    app.update_scroll();
    app.update_debug_info();
}

pub fn draw_user_input(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .border_type(BorderType::Rounded)
        .title(match app.input_mode {
            InputMode::Normal => {
                " Press 'e' to edit, 'r' to record, and ' Tab ' to see character sheet details "
            }
            InputMode::Editing => " Editing ",
            InputMode::Recording => " Recording… Press 'Esc' to stop ",
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
        f.set_cursor_position(Position::new(
            inner_area.x + cursor_x as u16,
            inner_area.y + cursor_y as u16,
        ));
    }
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
                            game_message.crunch,
                            game_message.fluff.render()
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
fn center_vertical(area: Rect, height: u16) -> Rect {
    let [area] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(area);
    area
}
