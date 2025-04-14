use super::{
    Component, MainMenu, center_rect, chunk_attributes, descriptions::*, draw_character_sheet,
    get_attributes, get_derived, input::Pastable, spinner::Spinner,
};
use crate::ai::GameAI;
use crate::{
    app::{Action, InputMode},
    context::Context,
    game_state::GameState,
    imager::load_image_from_file,
    message::{GameMessage, Message, MessageType, UserMessage},
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
use std::{fs, path::Path, time::Instant};
use tui_input::{Input, backend::crossterm::EventHandler};

// TODO: Make sure I still need the cache
//
// type Cache = RefCell<Option<(Rect, Vec<Rect>, Vec<Rect>)>>;
// thread_local! {
//     static CACHED_LAYOUTS: Cache = const {RefCell::new(None)};
// }

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
    pub visible_lines: usize,
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
        // TODO:  Do I need this cache ?
        //
        // let (_main_chunk, left_chunk, game_info_area) = CACHED_LAYOUTS.with(|cache: &Cache| {
        //     let mut cache = cache.borrow_mut();
        //     if cache.is_none() || cache.as_ref().expect("Expected a valide cache").0 != size {
        //         let main_chunk = Layout::default()
        //             .direction(Direction::Horizontal)
        //             .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        //             .split(size);
        //
        //         let left_chunk = Layout::default()
        //             .direction(Direction::Vertical)
        //             .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        //             .split(main_chunk[0]);
        //
        //         let new_cache = (size, main_chunk.to_vec(), left_chunk.to_vec());
        //         *cache = Some(new_cache);
        //     }
        //
        //     let (_, main_chunks, left_chunks) = cache.as_ref().expect("Expected a valide cache");
        //     (main_chunks.clone(), left_chunks.clone(), main_chunks[1])
        // });
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

        self.draw_user_input(buffer, context, left_screen[1]);

        match &self.state.main_character_sheet {
            Some(sheet) => {
                draw_character_sheet(
                    buffer,
                    sheet,
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
                        top: 1,
                        bottom: 0,
                    }));
                no_character.render(center_rect, buffer);
            }
        }
    }
}

impl InGame {
    pub fn new(state: GameState, picker: &Picker, game_ai: GameAI) -> Self {
        // TODO: Propagate the error
        let image = match &state.image_path {
            Some(image_path) => match load_image_from_file(picker, image_path) {
                Ok(image) => image,
                Err(e) => {
                    let error_message = format!(
                        " Path: {:?} Image error: {:?} ",
                        state.image_path,
                        e.to_string()
                    );
                    let log_path = Path::new("./error_log_image.txt");
                    if let Err(log_err) = fs::write(log_path, &error_message) {
                        eprintln!("Failed to write error log: {}", log_err);
                    }
                    panic!("Failed to load image: {}", error_message);
                }
            },
            None => {
                let error_message = "image_path is None".to_string();
                let log_path = Path::new("./error_log_path.txt");
                if let Err(log_err) = fs::write(log_path, &error_message) {
                    eprintln!("Failed to write error log: {}", log_err);
                }
                panic!("{}", error_message);
            }
        };

        Self {
            state,
            // Content should go fetch the meesages from the memory/AI
            content: Vec::new(),
            image: Some(image),
            // TODO: Input should be autonomous with info about its size and scroll
            input: Input::default(),
            highlighted_section: HighlightedSection::None,
            spinner: Spinner::new(),
            last_spinner_update: Instant::now(),
            spinner_active: false,
            total_lines: 0,
            visible_lines: 0,
            content_scroll: 0,
            ai: game_ai,
        }
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

    fn draw_game_content(&self, buffer: &mut Buffer, _context: &Context, area: Rect) {
        let save_name = &self.state.save_name;
        let fluff_block = Block::default()
            .border_type(BorderType::Rounded)
            .title(if save_name.is_empty() {
                " Game will start momentarily ".to_string()
            } else {
                format!(" {} ", save_name)
            })
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green));

        let fluff_area = fluff_block.inner(area);

        fluff_block.render(area, buffer);

        let max_width = fluff_area.width.saturating_sub(2) as usize;
        let max_height = fluff_area.height.saturating_sub(2) as usize;

        // TODO: cached content logic, verify it is needed.
        //
        // if app.cached_game_content.is_none()
        //     || app.cached_content_len != app.game_content.borrow().len()
        // {
        //     app.update_cached_content(max_width);
        // }
        //
        // let all_lines = app
        //     .cached_game_content
        //     .as_ref()
        //     .expect("Expected a valid ref to a cached_game_content");
        //
        // app.total_lines = all_lines.len();
        // *app.debug_info.borrow_mut() += &format!(", Total lines: {}", app.total_lines);
        let all_lines = self.parse_game_content(max_width);

        let visible_lines: Vec<Line> = all_lines
            .iter()
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

        // TODO: Make sure the scrolling works
        //
        // app.visible_lines = max_height;
        // app.update_scroll();
    }

    // FIX:  the cursor is not currently displayed properly. cf: https://github.com/ratatui/ratatui/discussions/872
    fn draw_user_input(&self, buffer: &mut Buffer, context: &Context, area: Rect) {
        let block = Block::default()
            .border_type(BorderType::Rounded)
            .title(match context.input_mode {
                InputMode::Normal => {
                    " Press 'e' to edit, 'r' to record, and ' Tab ' to see character sheet details "
                }
                InputMode::Editing => " Editing ",
                InputMode::Recording => " Recording… Press 'Esc' to stop ",
            })
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

    fn parse_game_content(&self, max_width: usize) -> Vec<(Line<'static>, Alignment)> {
        let mut all_lines = Vec::new();

        for message in self.content.iter() {
            let (content, base_style, alignment) = match message.message_type {
                MessageType::Game => {
                    if let Ok(game_message) = serde_json::from_str::<GameMessage>(&message.content)
                    {
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
                    if let Ok(user_message) = serde_json::from_str::<UserMessage>(&message.content)
                    {
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
            // HACK: This should be a different key handling for the detail section
            KeyCode::Esc if (self.highlighted_section != HighlightedSection::None) => {
                self.highlighted_section = HighlightedSection::None;
                None
            }
            KeyCode::Esc => {
                self.content.clear();
                self.input.reset();
                Some(Action::SwitchComponent(Box::new(MainMenu::default())))
            }
            KeyCode::Enter if !self.input.value().is_empty() => {
                Some(Action::SendMessage(self.input.value().into()))
            }
            KeyCode::PageUp => {
                for _ in 0..self.visible_lines {
                    self.scroll_up();
                }
                None
            }
            KeyCode::PageDown => {
                for _ in 0..self.visible_lines {
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

    pub fn update_scroll(&mut self) {
        let max_scroll = self.total_lines.saturating_sub(self.visible_lines);
        self.content_scroll = self.content_scroll.min(max_scroll);
    }

    pub fn scroll_up(&mut self) {
        if self.content_scroll > 0 {
            self.content_scroll -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        if self.content_scroll < self.total_lines.saturating_sub(self.visible_lines) {
            self.content_scroll += 1;
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        // Recalculate total lines
        self.total_lines = self.calculate_total_lines();

        // Update the scroll position
        self.content_scroll = self.total_lines.saturating_sub(self.visible_lines);
    }

    fn calculate_total_lines(&self) -> usize {
        self.content
            .iter()
            .map(|message| {
                let wrapped_lines = textwrap::wrap(&message.content, self.visible_lines);
                wrapped_lines.len()
            })
            .sum()
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
//
