// /ui/sheet/inventory.rs
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Cell, Row, Table, Widget},
};

use crate::character::CharacterSheet;
use crate::ui::game::HighlightedSection;

pub fn draw_inventory(
    buffer: &mut Buffer,
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
                .border_type(BorderType::Rounded)
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

    inventory_table.render(area, buffer);
}
