use crate::app::{App, InputMode};
use crate::character::CharacterSheet;
use crate::message::{GameMessage, MessageType, UserMessage};
use crate::ui::utils::spinner_frame;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::*,
};
use ratatui_image::{Resize, StatefulImage};
use std::cell::RefCell;
use std::path::PathBuf;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

thread_local! {
    static CACHED_LAYOUTS: RefCell<Option<(Rect, Vec<Rect>, Vec<Rect>)>> = RefCell::new(None);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HighlightedSection {
    None,
    Backstory,
    InventoryItem(String), // String is the item name
    Contact(String),       // String is the contact name
}

pub fn draw_in_game(f: &mut Frame, app: &mut App) {
    let size = f.area();
    *app.debug_info.borrow_mut() = format!("Terminal size: {}x{}", size.width, size.height);

    if size.width < 20 || size.height < 10 {
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

        let (_, main_chunks, left_chunks) = cache.as_ref().unwrap();
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

                    // Split the game_info_area into two parts: character sheet and details
                    let character_sheet_area = game_info_area;

                    draw_character_sheet(f, sheet, character_sheet_area, &app.highlighted_section);
                    draw_detailed_info(app, f, sheet, left_chunk[0]);
                } else {
                    app.last_known_character_sheet = None;
                    let no_character = Paragraph::new("No character sheet available.")
                        .style(Style::default().fg(Color::Yellow))
                        .alignment(Alignment::Center);
                    f.render_widget(no_character, game_info_area);
                }
            }
            Err(_) => {
                if let Some(last_sheet) = &app.last_known_character_sheet.clone() {
                    let character_sheet_area = game_info_area;
                    let details_area = Rect::new(
                        character_sheet_area.x,
                        character_sheet_area.bottom(),
                        character_sheet_area.width,
                        size.height - character_sheet_area.bottom(),
                    );

                    draw_character_sheet(
                        f,
                        last_sheet,
                        character_sheet_area,
                        &app.highlighted_section,
                    );
                    draw_detailed_info(app, f, last_sheet, details_area);
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

// Function to draw the character sheet.
fn draw_character_sheet(
    f: &mut Frame,
    sheet: &CharacterSheet,
    area: Rect,
    highlighted: &HighlightedSection,
) {
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
    draw_basic_info(f, sheet, chunks[0], highlighted);
    draw_attributes_and_derived(f, sheet, chunks[1], highlighted);
    draw_skills_qualities_and_other(f, sheet, chunks[2], highlighted);
}

pub fn draw_detailed_info(app: &mut App, f: &mut Frame, sheet: &CharacterSheet, area: Rect) {
    // Early return if HighlightedSection::None
    if matches!(&app.highlighted_section, HighlightedSection::None) {
        return;
    }

    let detail_text = match &app.highlighted_section {
        HighlightedSection::None => unreachable!(), // We've already returned in this case
        HighlightedSection::Backstory => sheet.backstory.clone(),
        HighlightedSection::InventoryItem(_) => sheet
            .inventory
            .values()
            .map(|item| format!("{}: {} (x{})", item.name, item.description, item.quantity))
            .collect::<Vec<_>>()
            .join("\n\n"),
        HighlightedSection::Contact(_) => sheet
            .contacts
            .values()
            .map(|contact| {
                format!(
                    "{}: Loyalty {}, Connection {}\n\n{}",
                    contact.name, contact.loyalty, contact.connection, contact.description
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n"),
    };
    // Wrap the text to fit within the area width
    let wrapped_text = textwrap::wrap(&detail_text, area.width as usize - 4); // -4 for margins
    let content_height = wrapped_text.len() as u16 + 2; // +2 for top and bottom margins

    // Calculate the size and position of the floating frame
    let width = (area.width - (f.area().width - 2) / 3).saturating_sub(4); // Minimum width of 20
    let height = content_height.max(f.area().height.saturating_sub(2));
    // let x = area.x + (area.width - width) / 2;
    let x = (f.area().width / 3) + 2;
    let y = 1;

    let details_area = Rect::new(x, y, width, height);

    // Create a block for the floating frame
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .title(match &app.highlighted_section {
            HighlightedSection::Backstory => " Backstory ",
            HighlightedSection::InventoryItem(_) => " Inventory Details ",
            HighlightedSection::Contact(_) => " Contact Details ",
            _ => " Details ",
        })
        .style(Style::default()); // Make the block opaque

    // Render the block
    f.render_widget(Clear, details_area); // Clear the area behind the block
    f.render_widget(block.clone(), details_area);

    // Get the inner area of the block for the content
    let inner_area = block.inner(details_area);

    let detail_paragraph = Paragraph::new(wrapped_text.join("\n"))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    // Render the content inside the block
    let mut stateful_image = StatefulImage::default();
    if let Some(image) = app.image.as_mut() {
        let image_rect = Rect::new(1, 1, (f.area().width + 2) / 3, f.area().height - 2);
        let image_block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", sheet.name));

        let resize: Resize = Resize::Fit(None);
        stateful_image = stateful_image.resize(resize);

        f.render_widget(detail_paragraph, inner_area);
        f.render_widget(Clear, image_rect);
        f.render_widget(image_block.clone(), image_rect);
        f.render_stateful_widget(stateful_image, image_block.inner(image_rect), image);
    } else {
        f.render_widget(detail_paragraph, inner_area);
    }
}

// Display basic information like name, race, and gender.
fn draw_basic_info(
    f: &mut Frame,
    sheet: &CharacterSheet,
    area: Rect,
    highlighted: &HighlightedSection,
) {
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
                .border_style(Style::default().fg(
                    if matches!(highlighted, HighlightedSection::Backstory) {
                        Color::Yellow
                    } else {
                        Color::White
                    },
                ))
                .title(" Basic Information "),
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(basic_info, area);
}
// Display attributes and derived attributes.

fn draw_attributes_and_derived(
    f: &mut Frame,
    sheet: &CharacterSheet,
    area: Rect,
    highlighted: &HighlightedSection,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    draw_attributes(f, sheet, chunks[0], highlighted);
    draw_derived_attributes(f, sheet, chunks[1], highlighted);
}

// Display specific attributes.

fn draw_attributes(
    f: &mut Frame,
    sheet: &CharacterSheet,
    area: Rect,
    _highlighted: &HighlightedSection,
) {
    let attributes = vec![
        ("BODY", sheet.body),
        ("AGILITY", sheet.agility),
        ("REACTION", sheet.reaction),
        ("STRENGTH", sheet.strength),
        ("WILLPOWER", sheet.willpower),
        ("LOGIC", sheet.logic),
        ("INTUITION", sheet.intuition),
        ("CHARISMA", sheet.charisma),
        ("EDGE", sheet.edge),
        ("MAGIC", sheet.magic.unwrap_or(0)),
        ("RESONANCE", sheet.resonance.unwrap_or(0)),
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
        .block(Block::default().borders(Borders::ALL).title(" Attributes "))
        .style(Style::default().fg(Color::White))
        .row_highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .column_spacing(1);

    f.render_widget(table, area);
}

// Display derived attributes like initiative and limits.

fn draw_derived_attributes(
    f: &mut Frame,
    sheet: &CharacterSheet,
    area: Rect,
    _highlighted: &HighlightedSection,
) {
    let derived = [
        format!(
            "Initiative:  {}+{}d6",
            sheet.initiative.0, sheet.initiative.1
        ),
        format!(
            "Limits:  PHY:{} MEN:{} SOC:{}",
            sheet.physical_limit, sheet.mental_limit, sheet.social_limit
        ),
        format!(
            "Monitors:  PHY:{} SOC:{}",
            sheet.physical_monitor, sheet.stun_monitor
        ),
        format!("Essence:  {:.2}", sheet.essence),
        format!("Edge Points:  {}", sheet.edge_points),
        format!("Armor:  {}", sheet.armor),
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
                .title(" Derived Attributes "),
        )
        .style(Style::default().fg(Color::White))
        .row_highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .column_spacing(1);

    f.render_widget(table, area);
}

fn draw_skills_qualities_and_other(
    f: &mut Frame,
    sheet: &CharacterSheet,
    area: Rect,
    highlighted: &HighlightedSection,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Skills
            Constraint::Length(5),  // Qualities
            Constraint::Min(0),     // Other Info
        ])
        .split(area);

    draw_skills(f, sheet, chunks[0], highlighted);
    draw_qualities(f, sheet, chunks[1], highlighted);
    draw_other_info(f, sheet, chunks[2], highlighted);
}

// Specific function to handle the display of skills.

fn draw_skills(
    f: &mut Frame,
    sheet: &CharacterSheet,
    area: Rect,
    _highlighted: &HighlightedSection,
) {
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
    .block(Block::default().borders(Borders::ALL).title(" Skills "))
    .style(Style::default().fg(Color::White))
    .row_highlight_style(Style::default().add_modifier(Modifier::BOLD))
    .column_spacing(1);

    f.render_widget(table, area);
}

// Function to handle the display of qualities.

fn draw_qualities(
    f: &mut Frame,
    sheet: &CharacterSheet,
    area: Rect,
    _highlighted: &HighlightedSection,
) {
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
        .block(Block::default().borders(Borders::ALL).title(" Qualities "))
        .wrap(Wrap { trim: true });
    f.render_widget(qualities_paragraph, area);
}

// Function to display miscellaneous information.

fn draw_other_info(
    f: &mut Frame,
    sheet: &CharacterSheet,
    area: Rect,
    highlighted: &HighlightedSection,
) {
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

    draw_resources(f, sheet, left_chunks[0], highlighted);
    draw_augmentations(f, sheet, right_chunks[0], highlighted);
    draw_contacts(f, sheet, right_chunks[1], highlighted);
    draw_inventory(f, sheet, left_chunks[1], highlighted);
}

fn draw_resources(
    f: &mut Frame,
    sheet: &CharacterSheet,
    area: Rect,
    _highlighted: &HighlightedSection,
) {
    let info = vec![
        format!("Lifestyle: {}", sheet.lifestyle),
        format!("Nuyen: {}", sheet.nuyen),
    ];

    let resources_table = create_table(&info, "Resources");
    f.render_widget(resources_table, area);
}

fn draw_augmentations(
    f: &mut Frame,
    sheet: &CharacterSheet,
    area: Rect,
    _highlighted: &HighlightedSection,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let cyberware_elements: Vec<Line> = sheet
        .cyberware
        .iter()
        .map(|cw| {
            Line::from(Span::styled(
                cw.clone(),
                Style::default().fg(if sheet.cyberware.is_empty() {
                    Color::DarkGray
                } else {
                    Color::White
                }),
            ))
        })
        .collect();

    let bioware_elements: Vec<Line> = sheet
        .bioware
        .iter()
        .map(|bw| {
            Line::from(Span::styled(
                bw.clone(),
                Style::default().fg(if sheet.bioware.is_empty() {
                    Color::DarkGray
                } else {
                    Color::White
                }),
            ))
        })
        .collect();

    let cyberware_paragraph = Paragraph::new(cyberware_elements)
        .block(Block::default().borders(Borders::ALL).title(" Cyberware "))
        .wrap(Wrap { trim: true });

    let bioware_paragraph = Paragraph::new(bioware_elements)
        .block(Block::default().borders(Borders::ALL).title(" Bioware "))
        .wrap(Wrap { trim: true });

    f.render_widget(cyberware_paragraph, chunks[0]);
    f.render_widget(bioware_paragraph, chunks[1]);
}

fn draw_contacts(
    f: &mut Frame,
    sheet: &CharacterSheet,
    area: Rect,
    highlighted: &HighlightedSection,
) {
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
            let style = Style::default().fg(Color::White);
            let cells = vec![
                Cell::from(name.clone()).style(style),
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
    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(
                if matches!(highlighted, HighlightedSection::Contact(_)) {
                    Color::Yellow
                } else {
                    Color::White
                },
            ))
            .title(" Contacts "),
    );

    f.render_widget(table, area);
}

fn draw_inventory(
    f: &mut Frame,
    sheet: &CharacterSheet,
    area: Rect,
    highlighted: &HighlightedSection,
) {
    let inventory_items: Vec<Row> = sheet
        .inventory
        .values()
        .map(|item| {
            let style = Style::default().fg(Color::White);
            Row::new(vec![
                Cell::from(format!("{} (x{})", item.name, item.quantity)).style(style),
            ])
        })
        .collect();

    let widths = vec![Constraint::Percentage(100)];
    let inventory_table = Table::new(inventory_items, widths)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Inventory ")
                .border_style(Style::default().fg(
                    if matches!(highlighted, HighlightedSection::InventoryItem(_)) {
                        Color::Yellow
                    } else {
                        Color::White
                    },
                )),
        )
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
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", title)),
        )
        .style(Style::default().fg(Color::White))
        .row_highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">>")
        .column_spacing(1)
}

pub fn draw_game_content(f: &mut Frame, app: &mut App, area: Rect) {
    let save_name = app
        .save_manager
        .current_save
        .clone()
        .map_or_else(|| String::from("Loading..."), |save| save.save_name);

    let fluff_block = Block::default()
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

pub fn draw_user_input(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
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
