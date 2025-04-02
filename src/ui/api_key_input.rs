// ui/api_key_input.rs

use crate::{
    app::{Action, App},
    context::Context,
};
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Position},
    prelude::{Alignment, Buffer, Rect},
    style::{Color, Style},
    widgets::*,
};
use tui_input::Input;

use super::Component;

#[derive(Debug, Default)]
pub struct ApiKeyInput {
    input: Input,
}

impl Component for ApiKeyInput {
    fn on_key(&mut self, key: KeyEvent, context: Context) -> Option<Action> {
        todo!()

        // TODO: adapt this to API input on_key

        // fn handle_api_key_editing(&mut self, key: KeyEvent) {
        //     match key.code {
        //         KeyCode::Enter => {
        //             // Handle API key submission
        //             self.input_mode = InputMode::Normal;
        //         }
        //         KeyCode::Esc => {
        //             self.input_mode = InputMode::Normal;
        //         }
        //         KeyCode::Char('v') => {
        //             if key.modifiers.contains(KeyModifiers::CONTROL) {
        //                 if let Err(e) = self.handle_paste() {
        //                     self.add_debug_message(format!("Failed to paste: {:#?}", e));
        //                 }
        //             } else {
        //                 self.api_key_input.handle_event(&Event::Key(key));
        //             }
        //         }
        //         _ => {
        //             self.api_key_input.handle_event(&Event::Key(key));
        //         }
        //     }

        // fn handle_api_key_input(&mut self, key: KeyEvent) {
        //     match key.code {
        //         KeyCode::Enter => {
        //             if !self.api_key_input.value().is_empty() {
        //                 let api_key = self.api_key_input.value().to_string();
        //                 self.settings.openai_api_key = Some(api_key.clone());
        //
        //                 let sender = self.command_sender.clone();
        //                 tokio::spawn(async move {
        //                     let is_valid = Settings::validate_api_key(&api_key).await;
        //                     let _ = sender.send(Action::ApiKeyValidationResult(is_valid));
        //                 });
        //
        //                 self.state = AppState::SettingsMenu;
        //             }
        //         }
        //         KeyCode::Esc => {
        //             self.state = AppState::SettingsMenu;
        //         }
        //         KeyCode::Char('v') => {
        //             if key.modifiers.contains(KeyModifiers::CONTROL) {
        //                 if let Err(e) = self.handle_paste() {
        //                     self.add_debug_message(format!("Failed to paste: {:#?}", e));
        //                 }
        //             } else {
        //                 self.api_key_input.handle_event(&Event::Key(key));
        //             }
        //         }
        //         _ => {
        //             self.api_key_input.handle_event(&Event::Key(key));
        //         }
        //     }
        // }
    }

    fn render(&self, area: Rect, buffer: &mut Buffer, context: &Context) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .flex(ratatui::layout::Flex::Center)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(1),
                ]
                .as_ref(),
            )
            .split(area);

        let title = Paragraph::new(" Enter a valid OpenAI API Key ")
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center);
        title.render(chunks[0], buffer);

        let input = Paragraph::new(self.input.value())
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .border_type(BorderType::Rounded)
                    .borders(Borders::ALL)
                    .title(" API Key "),
            );
        input.render(chunks[1], buffer);

        let instructions = Paragraph::new(" Press Enter to confirm, Esc to cancel ")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        instructions.render(chunks[2], buffer);

        let paste_info = Paragraph::new(" Use Ctrl+V to paste ")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        paste_info.render(chunks[3], buffer);
        // TODO: Make sure the cursor is properly set.
    }
}
