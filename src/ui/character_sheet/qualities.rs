use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

use super::super::game::HighlightedSection;
use crate::character::CharacterSheet;
pub fn draw_qualities(
    f: &mut Frame,
    sheet: &CharacterSheet,
    area: Rect,
    highlighted: &HighlightedSection,
) {
    let qualities: Vec<Line> = sheet
        .qualities
        .iter()
        .map(|quality| {
            let color = if quality.positive {
                Color::Green
            } else {
                Color::Red
            };
            Line::from(Span::styled(&quality.name, Style::default().fg(color)))
        })
        .collect();

    let qualities_paragraph = Paragraph::new(qualities)
        .block(
            Block::default()
                .border_type(BorderType::Rounded)
                .borders(Borders::ALL)
                .title(" Qualities "),
        )
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
