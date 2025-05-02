// /ui/sheet/augmentations.rs

use crate::character::CharacterSheet;
use crate::ui::game::HighlightedSection;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Widget, Wrap},
};

pub fn draw_augmentations(
    buffer: &mut Buffer,
    sheet: &CharacterSheet,
    area: Rect,
    highlighted: &HighlightedSection,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(sheet.cyberware.len() as u16),
            Constraint::Fill(sheet.bioware.len() as u16),
        ])
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
                .border_type(BorderType::Rounded)
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
                .border_type(BorderType::Rounded)
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

    cyberware_paragraph.render(chunks[0], buffer);
    bioware_paragraph.render(chunks[1], buffer);
}
