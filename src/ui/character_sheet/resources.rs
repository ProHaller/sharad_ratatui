// /ui/sheet/resources.rs
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Cell, Row, Table, Widget},
};

use crate::character::CharacterSheet;
use crate::ui::game::HighlightedSection;

pub fn draw_resources(
    buffer: &mut Buffer,
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
            .border_type(BorderType::Rounded)
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

    table.render(area, buffer);
}
