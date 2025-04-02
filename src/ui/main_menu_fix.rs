use super::center_rect;
use crate::ui::constants::{ART, TITLE};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};

pub fn render_header(buffer: &mut Buffer, area: Rect) {
    let header = Paragraph::new(format!("Sharad Ratatui v{}", env!("CARGO_PKG_VERSION")))
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().border_type(BorderType::Rounded))
        .alignment(Alignment::Center);
    header.render(area, buffer);
}
pub fn render_art(buffer: &mut Buffer, area: Rect) {
    let outer_block = Block::default()
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::DarkGray));
    outer_block.render(area, buffer);

    let inner_rect = center_rect(area, Constraint::Length(80), Constraint::Length(18));

    let inner_block = Block::default()
        .border_type(BorderType::Rounded)
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Green));
    inner_block.render(inner_rect, buffer);

    let art = Paragraph::new(ART)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));
    art.render(inner_rect, buffer);
}
pub fn render_title(buffer: &mut Buffer, area: Rect) {
    let outer_block = Block::default()
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::DarkGray));
    let title_area = center_rect(area, Constraint::Length(38), Constraint::Length(8));
    outer_block.render(title_area, buffer);

    let title = Paragraph::new(TITLE)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));
    title.render(title_area, buffer);
}
