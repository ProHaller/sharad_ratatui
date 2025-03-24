use super::super::utils::{MIN_HEIGHT, MIN_WIDTH};
use crate::app::{App, InputMode};
use crate::character::{CharacterSheet, DerivedAttributes};
use crate::descriptions::*;
use crate::message::{GameMessage, MessageType, UserMessage};
use crate::ui::utils::spinner_frame;
use ratatui::layout::Flex;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::*,
};
use ratatui_image::StatefulImage;
use std::cell::RefCell;
use std::cmp::min;
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
        if cache.is_none() || cache.as_ref().unwrap().0 != size {
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
        let image_block = Block::default().borders(Borders::ALL).title(" Portrait ");

        f.render_widget(detail_paragraph, inner_area);
        f.render_widget(Clear, image_rect);
        f.render_widget(&image_block, image_rect);
        // FIX: How to make the first rendering faster? Pre-rendering?
        f.render_stateful_widget(StatefulImage::new(), image_block.inner(image_rect), image);
    } else {
        f.render_widget(detail_paragraph, inner_area);
    }
}

fn chunk_attributes(attributes: Vec<(&str, u8)>, chunk_nb: u8) -> Vec<Line<'_>> {
    let line_chunks = attributes
        .chunks(4)
        .map(|chunk| {
            chunk
                .iter()
                .flat_map(|attr| {
                    vec![
                        Line::from(vec![
                            Span::styled(
                                format!("{}: ", attr.0),
                                Style::default().fg(Color::Yellow),
                            ),
                            Span::styled(
                                format!("{}", attr.1),
                                Style::default()
                                    .fg(Color::White)
                                    .add_modifier(Modifier::BOLD),
                            ),
                        ]),
                        Line::from(vec![Span::raw(get_attribute_description(attr))]),
                    ]
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    line_chunks[chunk_nb as usize].clone()
}

fn get_attributes(sheet: &CharacterSheet) -> Vec<(&str, u8)> {
    vec![
        ("BODY", sheet.attributes.body),
        ("AGILITY", sheet.attributes.agility),
        ("REACTION", sheet.attributes.reaction),
        ("STRENGTH", sheet.attributes.strength),
        ("WILLPOWER", sheet.attributes.willpower),
        ("LOGIC", sheet.attributes.logic),
        ("INTUITION", sheet.attributes.intuition),
        ("CHARISMA", sheet.attributes.charisma),
        ("EDGE", sheet.attributes.edge),
        ("MAGIC", sheet.magic.magic.unwrap_or(0)),
        ("RESONANCE", sheet.resonance.resonance.unwrap_or(0)),
    ]
}

fn get_attribute_description(attributes: &(&str, u8)) -> &'static str {
    match attributes.0 {
        "STRENGTH" => STRENGTH,
        "AGILITY" => AGILITY,
        "BODY" => BODY,
        "LOGIC" => LOGIC,
        "INTUITION" => INTUITION,
        "CHARISMA" => CHARISMA,
        "WILLPOWER" => WILLPOWER,
        "REACTION" => REACTION,
        "EDGE" => EDGE,
        "MAGIC" => MAGIC,
        "RESONANCE" => RESONANCE,
        _ => "This should not appear…",
    }
}

macro_rules! styled_line {
    ($label:expr, $value:expr) => {
        Line::from(vec![
            Span::styled(
                $label,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}", $value),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    };
}

fn get_derived(derived: &DerivedAttributes, nb: usize) -> Vec<Line<'_>> {
    let derived_lines = [
        vec![
            styled_line!(
                "Initiative: ",
                format!("{}+{}d6", derived.initiative.0, derived.initiative.1)
            ),
            Line::from(vec![Span::raw(INITIATIVE)]),
            styled_line!("Armor: ", derived.armor),
            Line::from(vec![Span::raw(ARMOR)]),
            styled_line!(
                "Monitors: ",
                format!(
                    "PHY:{} STU:{}",
                    derived.monitors.physical, derived.monitors.stun
                )
            ),
            Line::from(vec![Span::styled(
                "PHY: ",
                Style::default().fg(Color::Yellow),
            )]),
            Line::from(vec![Span::raw(MONITOR_PHYSICAL)]),
            Line::from(vec![Span::styled(
                "STU: ",
                Style::default().fg(Color::Yellow),
            )]),
            Line::from(vec![Span::raw(MONITOR_STUN)]),
        ],
        vec![
            styled_line!(
                "Limits: ",
                format!(
                    "PHY:{} MEN:{} SOC:{}",
                    derived.limits.physical, derived.limits.mental, derived.limits.social
                )
            ),
            Line::from(vec![Span::styled(
                "PHY: ",
                Style::default().fg(Color::Yellow),
            )]),
            Line::from(vec![Span::raw(LIMIT_PHYSICAL)]),
            Line::from(vec![Span::styled(
                "MEN: ",
                Style::default().fg(Color::Yellow),
            )]),
            Line::from(vec![Span::raw(LIMIT_MENTAL)]),
            Line::from(vec![Span::styled(
                "SOC: ",
                Style::default().fg(Color::Yellow),
            )]),
            Line::from(vec![Span::raw(LIMIT_SOCIAL)]),
            styled_line!("Essence: ", format!("{:.2}", derived.essence.current)),
            Line::from(vec![Span::raw(ESSENCE)]),
            styled_line!("Edge Points: ", derived.edge_points),
            Line::from(vec![Span::raw(EDGE_POINTS)]),
        ],
    ];
    derived_lines[nb].clone()
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
    highlighted: &HighlightedSection,
) {
    let attributes = get_attributes(sheet);
    let max_area: usize = area.width as usize / 6;
    let max_length =
        if (attributes.iter().map(|(name, _)| name.len()).max().unwrap() + 1) > max_area {
            3
        } else {
            attributes.iter().map(|(name, _)| name.len()).max().unwrap() + 1
        };

    let rows: Vec<Row> = attributes
        .chunks(4)
        .map(|chunk| {
            Row::new(chunk.iter().map(|(attr, value)| {
                Cell::from(Line::from(vec![
                    Span::styled(
                        attr.split_at(min(attr.len(), max_length.max(3)))
                            .0
                            .to_string(),
                        Style::default().fg(Color::Green),
                    ),
                    Span::raw(if attr.len() < max_length {
                        " ".repeat(max_length - attr.len())
                    } else {
                        " ".to_string()
                    }),
                    Span::raw(value.to_string()),
                ]))
            }))
        })
        .collect();

    let table = Table::new(rows, [Constraint::Percentage(20); 4])
        .flex(Flex::Center)
        .header(Row::new(vec![""]))
        .column_spacing(1)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(
                    if matches!(highlighted, HighlightedSection::Attributes(_)) {
                        Color::Yellow
                    } else {
                        Color::White
                    },
                ))
                .title(" Attributes "),
        )
        .row_highlight_style(Style::default().add_modifier(Modifier::BOLD));

    f.render_widget(table, area);
}

fn draw_derived_attributes(
    f: &mut Frame,
    sheet: &CharacterSheet,
    area: Rect,
    highlighted: &HighlightedSection,
) {
    let derived = [
        format!(
            "Initiative:  {}+{}d6",
            sheet.derived_attributes.initiative.0, sheet.derived_attributes.initiative.1
        ),
        format!("Armor:  {}", sheet.derived_attributes.armor),
        format!(
            "Monitors:  PHY:{} STU:{}",
            sheet.derived_attributes.monitors.physical, sheet.derived_attributes.monitors.stun
        ),
        format!(
            "Limits:  PHY:{} MEN:{} SOC:{}",
            sheet.derived_attributes.limits.physical,
            sheet.derived_attributes.limits.mental,
            sheet.derived_attributes.limits.social
        ),
        format!("Essence:  {:.2}", sheet.derived_attributes.essence.current),
        format!("Edge Points:  {}", sheet.attributes.edge),
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
        .style(
            Style::default().fg(if matches!(highlighted, HighlightedSection::Derived(_)) {
                Color::Yellow
            } else {
                Color::White
            }),
        )
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
    highlighted: &HighlightedSection,
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
    .row_highlight_style(Style::default().add_modifier(Modifier::BOLD))
    .column_spacing(1)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Skills ")
            .border_style(Style::default().fg(
                if matches!(highlighted, HighlightedSection::Skills) {
                    Color::Yellow
                } else {
                    Color::White
                },
            )),
    );

    f.render_widget(table, area);
}

// Function to handle the display of qualities.

fn draw_qualities(
    f: &mut Frame,
    sheet: &CharacterSheet,
    area: Rect,
    highlighted: &HighlightedSection,
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
        .style(
            Style::default().fg(if matches!(highlighted, HighlightedSection::Qualities) {
                Color::Yellow
            } else {
                Color::White
            }),
        )
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
    highlighted: &HighlightedSection,
) {
    let header_cells = ["Nuyen", "Lifestyle"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow)));
    let header = Row::new(header_cells)
        .style(Style::default())
        .height(1)
        .bottom_margin(0);

    let nuyen = sheet.nuyen;
    let life_style = sheet.lifestyle.to_string();
    let rows: Vec<Row> = vec![Row::new(vec![
        Cell::from(format!("¥{}", nuyen)),
        Cell::from(life_style),
    ])];
    let widths = vec![Constraint::Max(10), Constraint::Fill(0)];
    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(
                if matches!(highlighted, HighlightedSection::Resources) {
                    Color::Yellow
                } else {
                    Color::White
                },
            ))
            .title(" Resources "),
    );

    f.render_widget(table, area);
}

fn draw_augmentations(
    f: &mut Frame,
    sheet: &CharacterSheet,
    area: Rect,
    highlighted: &HighlightedSection,
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
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if sheet.cyberware.is_empty() {
                    Color::DarkGray
                } else if matches!(highlighted, HighlightedSection::Cyberware) {
                    Color::Yellow
                } else {
                    Color::White
                }))
                .title(" Cyberware "),
        )
        .wrap(Wrap { trim: true });

    let bioware_paragraph = Paragraph::new(bioware_elements)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if sheet.bioware.is_empty() {
                    Color::DarkGray
                } else if matches!(highlighted, HighlightedSection::Bioware) {
                    Color::Yellow
                } else {
                    Color::White
                }))
                .title(" Bioware "),
        )
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

    let widths = vec![Constraint::Fill(0), Constraint::Max(8), Constraint::Max(11)];
    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(
                if matches!(highlighted, HighlightedSection::Contact) {
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
                .border_style(Style::default().fg(if sheet.inventory.is_empty() {
                    Color::DarkGray
                } else if matches!(highlighted, HighlightedSection::Inventory) {
                    Color::Yellow
                } else {
                    Color::White
                })),
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
fn center_vertical(area: Rect, height: u16) -> Rect {
    let [area] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(area);
    area
}
