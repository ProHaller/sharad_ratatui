// /ui/sheet/sheet.rs
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::*,
};
use std::cmp::min;

use crate::ui::game::HighlightedSection;
use crate::{character::CharacterSheet, character::DerivedAttributes, ui::descriptions::*};

use super::{draw_augmentations, draw_inventory, draw_qualities, draw_resources};

pub fn draw_character_sheet(
    f: &mut Frame,
    sheet: &CharacterSheet,
    area: Rect,
    highlighted: &HighlightedSection,
) {
    // Layout for different sections of the character sheet.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Max(3),
            Constraint::Max(14),
            Constraint::Fill(1),
            Constraint::Max(sheet.contacts.len() as u16 + 3),
        ])
        .split(area);

    // Drawing individual sections of the character sheet.
    draw_basic_info(f, sheet, chunks[0], highlighted);
    draw_attributes_and_derived(f, sheet, chunks[1], highlighted);
    draw_skills_qualities_and_other(f, sheet, chunks[2], highlighted);
    draw_contacts(f, sheet, chunks[3], highlighted);
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
                .border_type(BorderType::Rounded)
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
fn draw_attributes(
    f: &mut Frame,
    sheet: &CharacterSheet,
    area: Rect,
    highlighted: &HighlightedSection,
) {
    let attributes = get_attributes(sheet);
    let max_area: usize = area.width as usize / 6;
    let max_length = if (attributes
        .iter()
        .map(|(name, _)| name.len())
        .max()
        .expect("Expected a valid max len")
        + 1)
        > max_area
    {
        3
    } else {
        attributes
            .iter()
            .map(|(name, _)| name.len())
            .max()
            .expect("Expected a valid max len")
            + 1
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
                .border_type(BorderType::Rounded)
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
        format!("Essence:  {:.2}", sheet.derived_attributes.essence.current),
        format!("Edge Points:  {}", sheet.attributes.edge),
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
                .border_type(BorderType::Rounded)
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
        .column_spacing(1)
        .flex(Flex::Center);

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
            Constraint::Max(7),  // Skills
            Constraint::Fill(0), // Other Info
        ])
        .split(area);

    draw_skills(f, sheet, chunks[0], highlighted);
    draw_other_info(f, sheet, chunks[1], highlighted);
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

    // Header row
    let header = Row::new(
        categories
            .iter()
            .map(|(category, _)| {
                Cell::from(Span::styled(
                    *category,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ))
            })
            .collect::<Vec<Cell>>(),
    );

    // Extract skill lists into a Vec of Vec<(skill, rating)>
    let skill_columns: Vec<Vec<(String, u8)>> = categories
        .iter()
        .map(|(_, skills)| {
            skills
                .iter()
                .map(|(s, r)| (s.to_string(), *r))
                .collect::<Vec<_>>()
        })
        .collect();

    // Find max number of skill rows
    let max_rows = skill_columns.iter().map(|col| col.len()).max().unwrap_or(0);

    // Build rows row-by-row across columns
    let rows: Vec<Row> = (0..max_rows)
        .map(|i| {
            let cells = skill_columns
                .iter()
                .map(|col| {
                    if let Some((skill, rating)) = col.get(i) {
                        Cell::from(format!("{}:{}", skill, rating))
                    } else {
                        Cell::from("") // Empty cell if no skill at this row
                    }
                })
                .collect::<Vec<Cell>>();
            Row::new(cells)
        })
        .collect();

    let table = Table::new(rows, vec![Constraint::Fill(0); 5])
        .header(header)
        .row_highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .column_spacing(1)
        .block(
            Block::default()
                .border_type(BorderType::Rounded)
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
            Constraint::Min(sheet.qualities.len() as u16 + 1),
            Constraint::Max(4),
            Constraint::Fill(0),
        ])
        .split(chunks[0]);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill((sheet.bioware.len() as u16).max(sheet.cyberware.len() as u16)),
            Constraint::Min(sheet.inventory.len() as u16 + 2),
        ])
        .split(chunks[1]);

    draw_qualities(f, sheet, left_chunks[0], highlighted);
    draw_resources(f, sheet, left_chunks[1], highlighted);
    draw_augmentations(f, sheet, right_chunks[0], highlighted);
    draw_inventory(f, sheet, right_chunks[1], highlighted);
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
            .border_type(BorderType::Rounded)
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

pub fn chunk_attributes(attributes: Vec<(&str, u8)>, chunk_nb: u8) -> Vec<Line<'_>> {
    let line_chunks = attributes
        .chunks(4)
        .map(|chunk| {
            chunk
                .iter()
                .flat_map(|attr| {
                    vec![
                        Line::from(vec![Span::raw("")]),
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

pub fn get_attributes(sheet: &CharacterSheet) -> Vec<(&str, u8)> {
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

pub fn get_derived(derived: &DerivedAttributes, nb: usize) -> Vec<Line<'_>> {
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
