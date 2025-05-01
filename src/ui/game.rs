use super::{
    Component, ComponentEnum, MainMenu, center_rect, chunk_attributes,
    descriptions::*,
    draw_character_sheet, get_attributes, get_derived,
    input::Pastable,
    spinner::{Spinner, spinner_frame},
};
use crate::{
    ai::GameAI,
    app::{Action, InputMode},
    context::Context,
    error::Error,
    game_state::GameState,
    imager::load_image_from_file,
    message::{
        GameMessage, Message, MessageType, UserCompletionRequest, UserMessage, create_user_message,
    },
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use derive_more::Debug;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::*,
};
use ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol};
use std::time::{Duration, Instant};
use tui_input::{Input, backend::crossterm::EventHandler};

pub struct InGame {
    // GamePlay state:
    pub state: GameState,
    pub content: Vec<Message>,
    pub image: Option<StatefulProtocol>,

    //AI
    pub ai: GameAI,

    // User actions:
    pub input: Input,
    pub highlighted_section: HighlightedSection,

    // UI state:
    // TODO: implement the spinner in a seprarte struct and thread
    pub spinner: Spinner,
    pub last_spinner_update: Instant,
    pub spinner_active: bool,
    pub total_lines: usize,
    pub all_lines: Vec<(Line<'static>, Alignment)>,
    pub max_height: usize,
    pub max_width: usize,
    pub content_scroll: usize,
}

impl std::fmt::Debug for InGame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InGame")
            .field("state", &self.state)
            .field("content", &self.content)
            .field(
                "image",
                &format_args!(
                    "The StatefulProtocol cannot be printed. The Option is {}",
                    if self.image.is_some() { "Some" } else { "None" }
                ),
            )
            .finish()
    }
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

impl Component for InGame {
    fn on_key(&mut self, key: KeyEvent, context: Context) -> Option<Action> {
        match context.input_mode {
            InputMode::Normal => self.handle_normal_input(key, context),
            InputMode::Editing => self.handle_edit_input(key, context),
            // TODO: handle the voice recording
            InputMode::Recording => Some(Action::SwitchInputMode(InputMode::Normal)),
        }
    }

    fn render(&mut self, area: Rect, buffer: &mut Buffer, context: &Context) {
        let screen_split_layout = Layout::default()
            .direction(Direction::Horizontal)
            .flex(ratatui::layout::Flex::Center)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
            .split(area);
        let left_screen = Layout::default()
            .direction(Direction::Vertical)
            .flex(ratatui::layout::Flex::Center)
            .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
            .split(screen_split_layout[0]);

        self.draw_game_content(buffer, context, left_screen[0]);

        self.draw_spinner(buffer, left_screen[0]);
        self.draw_user_input(buffer, context, left_screen[1]);

        let image_present = self.state.image_path.is_some();
        match &self.state.main_character_sheet {
            Some(sheet) => {
                draw_character_sheet(
                    buffer,
                    sheet,
                    image_present,
                    screen_split_layout[1],
                    &self.highlighted_section,
                );
                self.draw_detailed_info(screen_split_layout[0], buffer, context);
            }
            None => {
                let center_rect = center_rect(
                    screen_split_layout[1],
                    Constraint::Percentage(100),
                    Constraint::Length(3),
                );
                let center_block = Block::bordered();
                let no_character = Paragraph::new("No character sheet available yet.")
                    .style(Style::default().fg(Color::Yellow))
                    .alignment(Alignment::Center)
                    .block(center_block.padding(Padding {
                        left: 0,
                        right: 0,
                        top: 0,
                        bottom: 0,
                    }));
                no_character.render(center_rect, buffer);
            }
        }
    }
}

impl InGame {
    pub fn new(state: GameState, picker: &Picker, game_ai: GameAI, content: Vec<Message>) -> Self {
        // TODO: Propagate the error
        let image = match &state.image_path {
            Some(image_path) => match load_image_from_file(picker, image_path) {
                Ok(image) => Some(image),
                Err(e) => {
                    log::error!("Couldn't load_image_from_file: {e:#?}");
                    None
                }
            },
            None => None,
        };

        let mut new_self = Self {
            ai: game_ai,
            state,
            content,
            image,
            // TODO: Input should be autonomous with info about its size and scroll
            input: Input::default(),
            highlighted_section: HighlightedSection::None,
            spinner: Spinner::new(),
            last_spinner_update: Instant::now(),
            spinner_active: false,
            all_lines: Vec::new(),
            total_lines: 0,
            max_height: 30,
            max_width: 90,
            content_scroll: 0,
        };
        new_self.on_creation();
        new_self
    }

    pub fn draw_detailed_info(&mut self, area: Rect, buffer: &mut Buffer, _context: &Context) {
        // Early return if HighlightedSection::None
        if matches!(self.highlighted_section, HighlightedSection::None) {
            return;
        }

        let detail_area = Layout::horizontal([Constraint::Ratio(1, 2); 2]).split(area);

        let sheet = self
            .state
            .main_character_sheet
            .as_ref()
            .expect("Expected a character sheet");
        let attributes = get_attributes(sheet);
        let detail_text = match self.highlighted_section {
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

        Clear.render(area, buffer);

        // Create a block for the floating frame
        let block = Block::default()
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            // TODO: Make this automatic with strum
            .title(match self.highlighted_section {
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

        let detail_paragraph = Paragraph::new(detail_text) // Use
            // the wrapped text as the Paragraph detail_text)
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true })
            .block(block);

        // Render the content inside the block
        if let Some(image) = &mut self.image {
            // HACK: Probably a better way to render the image.
            let image_block = Block::default()
                .border_type(BorderType::Rounded)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White))
                .title(" Portrait ");

            detail_paragraph.render(detail_area[1], buffer);
            image_block.render(detail_area[0], buffer);
            // FIX: How to make the first rendering faster? Pre-rendering?
            StatefulImage::new().render(detail_area[0].inner(Margin::new(1, 1)), buffer, image);
        } else {
            detail_paragraph.render(area, buffer);
        }
    }

    fn draw_game_content(&mut self, buffer: &mut Buffer, _context: &Context, area: Rect) {
        let save_name = &self.state.save_name;
        let fluff_block = Block::default()
            .border_type(BorderType::Rounded)
            .title(if save_name.is_empty() {
                " Game will start momentarily ".to_string()
            } else {
                format!(" {} ", save_name)
            })
            .borders(Borders::ALL);

        let fluff_area = fluff_block.inner(area);

        fluff_block.render(area, buffer);

        self.max_width = fluff_area.width.saturating_sub(2) as usize;
        self.max_height = fluff_area.height.saturating_sub(2) as usize;

        let visible_lines: Vec<Line> = self
            .all_lines
            .iter()
            .skip(self.content_scroll)
            .take(self.max_height)
            .map(|(line, alignment)| {
                let mut new_line = line.clone();
                new_line.alignment = Some(*alignment);
                new_line
            })
            .collect();

        let content = Paragraph::new(visible_lines)
            .block(
                Block::default()
                    .border_type(BorderType::Rounded)
                    .borders(Borders::NONE),
            )
            .wrap(Wrap { trim: true });

        content.render(fluff_area, buffer);

        self.update_scroll();
    }

    // FIX:  the cursor is not currently displayed properly. cf: https://github.com/ratatui/ratatui/discussions/872
    fn draw_user_input(&self, buffer: &mut Buffer, context: &Context, area: Rect) {
        let lines = &format!(
            "Total lines: {}, visible_lines: {}, content_scroll: {}, ",
            self.total_lines, self.max_height, self.content_scroll
        )
        .clone();
        let block = Block::default()
            .border_type(BorderType::Rounded)
            .title(match context.input_mode {
                InputMode::Normal => {
                    " Press 'e' to edit, 'r' to record, and ' Tab ' to see character sheet details "
                }
                InputMode::Editing => " Editing ",
                InputMode::Recording => " Recording… Press 'Esc' to stop ",
            })
            .title_bottom(Line::from(lines.as_str()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(match context.input_mode {
                InputMode::Normal => Color::DarkGray,
                InputMode::Editing => Color::White,
                InputMode::Recording => Color::Red,
            }));

        // let max_width = area.width as usize - 2;
        //
        // let text = self.input.value();
        //
        // // Wrap the text manually, considering grapheme clusters and their widths
        // let mut wrapped_lines = Vec::new();
        // let mut current_line = String::new();
        // let mut current_width = 0;
        //
        // for grapheme in text.graphemes(true) {
        //     let grapheme_width = grapheme.width();
        //     if current_width + grapheme_width > max_width {
        //         wrapped_lines.push(current_line);
        //         current_line = String::new();
        //         current_width = 0;
        //     }
        //     current_line.push_str(grapheme);
        //     current_width += grapheme_width;
        // }
        // if !current_line.is_empty() {
        //     wrapped_lines.push(current_line);
        // }
        //
        // // Calculate cursor position
        // let cursor_position = self.input.visual_cursor();
        // let mut cursor_x = 0;
        // let mut cursor_y = 0;
        // let mut total_width = 0;
        //
        // for (line_idx, line) in wrapped_lines.iter().enumerate() {
        //     let line_width: usize = line.width();
        //     if total_width + line_width >= cursor_position {
        //         cursor_y = line_idx;
        //         cursor_x = cursor_position - total_width;
        //         break;
        //     }
        //     total_width += line_width;
        //     cursor_y = line_idx + 1;
        // }
        //
        // // Ensure cursor_x doesn't exceed the line width
        // if cursor_y < wrapped_lines.len() {
        //     cursor_x = cursor_x.min(wrapped_lines[cursor_y].width());
        // }
        //
        // let joined_lines = wrapped_lines.join("\n");

        let input = Paragraph::new(self.input.value())
            .style(Style::default().fg(match context.input_mode {
                InputMode::Normal => Color::DarkGray,
                InputMode::Editing => Color::Yellow,
                InputMode::Recording => Color::Red,
            }))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false })
            .block(block);

        input.render(area, buffer);

        // TODO: Verify the position of the cursor in the input field

        // // Adjust cursor position if it's beyond the visible area
        // let visible_lines = inner_area.height.saturating_sub(1) as usize;
        // if cursor_y >= visible_lines {
        //     cursor_y = visible_lines.saturating_sub(1);
        // }
        //
        // // Set cursor
    }

    fn parse_full_game_content(&self) -> Vec<(Line<'static>, Alignment)> {
        let mut all_lines = Vec::new();

        for message in self.content.iter() {
            all_lines.extend(self.parse_message(message));
        }

        all_lines
    }

    pub fn new_message(&mut self, new_message: &Message) {
        self.content.push(new_message.clone());
        let new_lines = self.parse_message(new_message);
        self.total_lines += new_lines.len();
        self.all_lines.extend(new_lines);
        self.update_scroll();
        self.scroll_to_bottom();
    }

    fn parse_message(&self, message: &Message) -> Vec<(Line<'static>, Alignment)> {
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

        let wrapped_lines = textwrap::wrap(&content, self.max_width);
        let mut lines = Vec::new();
        for line in wrapped_lines {
            let parsed_line = parse_markdown(line.to_string(), base_style);
            lines.push((parsed_line, alignment));
        }
        lines
    }

    pub fn handle_edit_input(&mut self, key: KeyEvent, context: Context) -> Option<Action> {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => Some(Action::SwitchInputMode(InputMode::Normal)),
            KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.paste(context);
                None
            }
            _ => {
                self.input.handle_event(&crossterm::event::Event::Key(key));
                None
            }
        }
    }

    fn handle_normal_input(&mut self, key: KeyEvent, context: Context) -> Option<Action> {
        match key.code {
            KeyCode::Char('e') => Some(Action::SwitchInputMode(InputMode::Editing)),
            KeyCode::Char('r') => {
                // TODO: Handle Recording
                // Some(Action::SwitchInputMode(InputMode::Recording))
                None
            }
            KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.paste(context);
                None
            }
            // HACK: This should be a different key handling for the detail section
            KeyCode::Esc if (self.highlighted_section != HighlightedSection::None) => {
                self.highlighted_section = HighlightedSection::None;
                None
            }
            KeyCode::Esc => {
                self.content.clear();
                self.input.reset();
                context
                    .save_manager
                    .save(&self.state)
                    .expect("Should have saved from the game");
                Some(Action::SwitchComponent(ComponentEnum::from(
                    MainMenu::default(),
                )))
            }
            KeyCode::Enter if !self.input.value().is_empty() => {
                let value = self.input.value();
                self.spinner_active = true;
                self.new_message(&Message::new(MessageType::User, value.into()));
                let message = self.build_user_completion_message(&context);
                // HACK: How could I avoid to clone this?
                let ai = self.ai.clone();
                tokio::spawn(async move {
                    ai.send_message(message, ai.ai_sender.clone()).await?;
                    Ok::<(), Error>(())
                });
                self.input.reset();
                None
            }
            KeyCode::PageUp => {
                for _ in 0..self.max_height {
                    self.scroll_up();
                }
                None
            }
            KeyCode::PageDown => {
                for _ in 0..self.max_height {
                    self.scroll_down();
                }
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_up();
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll_down();
                None
            }

            KeyCode::Tab => {
                self.cycle_highlighted_section();
                None
            }

            KeyCode::Home => {
                self.content_scroll = 0;
                None
            }
            KeyCode::End => {
                self.scroll_to_bottom();
                None
            }
            _ => None,
        }
    }

    fn build_user_completion_message(&self, context: &Context) -> UserCompletionRequest {
        let message = UserCompletionRequest {
            language: context.settings.language.to_string(),
            message: create_user_message(
                &context.settings.language.to_string(),
                self.input.value(),
            ),
            state: self.state.clone(),
        };
        message
    }

    pub fn update_scroll(&mut self) {
        let max_scroll = self.total_lines.saturating_sub(self.max_height);
        self.content_scroll = self.content_scroll.min(max_scroll);
    }

    pub fn scroll_up(&mut self) {
        if self.content_scroll > 0 {
            self.content_scroll -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        if self.content_scroll < self.total_lines.saturating_sub(self.max_height) {
            self.content_scroll += 1;
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        // Update the scroll position
        self.content_scroll = self.total_lines.saturating_sub(self.max_height);
    }

    fn cycle_highlighted_section(&mut self) {
        let Some(character_sheet) = &mut self.state.main_character_sheet else {
            return;
        };

        let available_sections = [
            Some(HighlightedSection::Backstory),
            Some(HighlightedSection::Attributes(0)),
            Some(HighlightedSection::Attributes(1)),
            Some(HighlightedSection::Attributes(2)),
            Some(HighlightedSection::Derived(0)),
            Some(HighlightedSection::Derived(1)),
            Some(HighlightedSection::Skills),
            Some(HighlightedSection::Qualities),
            (!character_sheet.cyberware.is_empty()).then_some(HighlightedSection::Cyberware),
            (!character_sheet.bioware.is_empty()).then_some(HighlightedSection::Bioware),
            Some(HighlightedSection::Resources),
            (!character_sheet.inventory.is_empty()).then_some(HighlightedSection::Inventory),
            (!character_sheet.contacts.is_empty()).then_some(HighlightedSection::Contact),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        if available_sections.is_empty() {
            self.highlighted_section = HighlightedSection::None;
            return;
        }

        let current_index = available_sections
            .iter()
            .position(|s| s == &self.highlighted_section)
            .unwrap_or(usize::MAX);

        let next_index =
            (current_index.wrapping_add(1)) % (available_sections.len().wrapping_add(1));

        self.highlighted_section = if next_index < available_sections.len() {
            available_sections[next_index].clone()
        } else {
            HighlightedSection::None
        };
    }

    fn on_creation(&mut self) {
        self.all_lines = self.parse_full_game_content();
        self.total_lines = self.all_lines.len();
        // HACK: This should be set to fluff_area max_height
        self.content_scroll = self.total_lines.saturating_sub(30);

        self.scroll_to_bottom();
        // TODO: Maybe I could precompute the image here.
    }

    fn draw_spinner(&mut self, buffer: &mut Buffer, left_screen: Rect) {
        if !self.spinner_active {
            return;
        };
        self.update_spinner();
        let spinner_area = Rect::new(
            left_screen.x,
            left_screen.bottom() - 1,
            left_screen.width,
            1,
        );

        let spinner_text = spinner_frame(&self.spinner);
        let spinner_widget = Paragraph::new(spinner_text)
            .style(Style::default())
            .alignment(Alignment::Center);

        spinner_widget.render(spinner_area, buffer);
    }

    pub fn update_spinner(&mut self) {
        if self.spinner_active && self.last_spinner_update.elapsed() >= Duration::from_millis(100) {
            self.spinner.next_frame();
            self.last_spinner_update = Instant::now();
        }
    }
}

// Function to parse markdown-like text to formatted spans.
pub fn parse_markdown(line: String, base_style: Style) -> Line<'static> {
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

// TODO: add this to the game component

// fn submit_user_input(&mut self) {
//     let input = self.input.value().trim().to_string();
//     self.start_spinner();
//
//     if input.is_empty() {
//         return;
//     }
//
//     self.add_message(Message::new(MessageType::User, input.clone()));
//
//     // Send a command to process the message
//     if let Err(e) = self.action_sender.send(Action::ProcessMessage(input)) {
//         self.add_message(Message::new(
//             MessageType::System,
//             format!("Error sending message command: {:#?}", e),
//         ));
//     }
//
//     // Clear the user input
//     self.input = Input::default();
//     self.scroll_to_bottom();
// }
