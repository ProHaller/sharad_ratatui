use copypasta::{ClipboardContext, ClipboardProvider};
use ratatui::layout::Alignment;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders};
use std::fmt::{self, Debug};
use tui_textarea::{CursorMove, Input, Key, Scrolling, TextArea};

use crate::audio::{self, get_sound};

use super::game::SectionMove;

pub fn new_textarea(placeholder: impl Into<String>) -> TextArea<'static> {
    let mut textarea = TextArea::default();
    textarea.set_placeholder_text(placeholder);
    textarea.set_cursor_line_style(Style::default());
    textarea.set_placeholder_style(Style::default().fg(Color::DarkGray));
    textarea.set_selection_style(Style::new().bg(Color::LightCyan));
    textarea
}

pub fn new_textarea_with_lines(
    lines: Vec<String>,
    placeholder: impl Into<String>,
) -> TextArea<'static> {
    let mut textarea = TextArea::new(lines);
    textarea.set_placeholder_text(placeholder);
    textarea.set_cursor_line_style(Style::default());
    textarea.set_placeholder_style(Style::default().fg(Color::DarkGray));
    textarea.set_selection_style(Style::new().bg(Color::LightCyan));
    textarea
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Warning {
    AudioInputDisabled,
    FailedNewTranscription,
    InputTooShort,
}
impl Warning {
    fn color(&self) -> Color {
        match self {
            Warning::AudioInputDisabled => Color::Yellow,
            Warning::FailedNewTranscription => Color::Red,
            Warning::InputTooShort => Color::Yellow,
        }
    }
    fn text(&self) -> String {
        let text = match self {
            Warning::AudioInputDisabled => {
                " The audio Recording is disabled. Change your settings to record "
            }
            Warning::FailedNewTranscription => " Failed to create a new Transcription. ",
            Warning::InputTooShort => " Input too Short. Write something before validation. ",
        };
        text.to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
    Operator(char),
    Recording,
    Warning(Warning),
}
impl Mode {
    pub fn new_warning(warning: Warning) -> Mode {
        if let Some(alert) = get_sound("warning") {
            tokio::spawn(async move {
                if let Err(e) = audio::play_audio(alert) {
                    log::error!("Failed to play alert sound: {e:#?}");
                }
            });
        }
        Mode::Warning(warning)
    }
}

impl<'a> Mode {
    pub fn block(self) -> Block<'a> {
        let help = match self {
            Mode::Normal => "type i, or a to enter insert mode",
            Mode::Insert => "Esc to go back to normal mode",
            Mode::Visual => "y to yank, d to delete, Esc to cancel",
            Mode::Operator(_) => "move cursor to apply, or repeat for full-line",
            Mode::Recording => "type any key to stop the recording",
            Mode::Warning(warning) => &warning.text(),
        };
        let mode = format!(" {} ", self);
        let help = format!(" {} ", help);
        let border_style = Style::default().fg(match self {
            Mode::Normal => Color::default(),
            Mode::Insert => Color::LightCyan,
            Mode::Visual => Color::LightBlue,
            Mode::Operator(_) => Color::LightYellow,
            Mode::Recording => Color::LightRed,
            Mode::Warning(warning) => warning.color(),
        });
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .border_type(BorderType::Rounded)
            .title_bottom(Line::from(mode).left_aligned())
            .title_bottom(Line::from(help).right_aligned())
            .title_alignment(Alignment::Center)
    }

    pub fn cursor_style(&self) -> Style {
        let color = match self {
            Self::Normal => Color::Reset,
            Self::Insert => Color::LightBlue,
            Self::Visual => Color::LightYellow,
            Self::Operator(_) => Color::LightGreen,
            Self::Recording => Color::LightRed,
            Self::Warning(warning) => warning.color(),
        };
        Style::default().fg(color).add_modifier(Modifier::REVERSED)
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Normal => write!(f, "NORMAL"),
            Self::Insert => write!(f, "INSERT"),
            Self::Visual => write!(f, "VISUAL"),
            Self::Operator(c) => write!(
                f,
                "OPERATOR({})",
                match c {
                    'y' => "Yank".to_string(),
                    'd' => "Delete".to_string(),
                    'c' => "Cut".to_string(),
                    c => c.to_string(),
                }
            ),
            Self::Recording => write!(f, "RECORDING"),
            Self::Warning(_) => write!(f, "WARNING"),
        }
    }
}

// How the Vim emulation state transitions
pub enum Transition {
    Nop,
    Validation,
    EndRecording,
    Detail(SectionMove),
    Exit,
    Mode(Mode),
    Pending(Input),
    ScrollTop,
    ScrollBottom,
    PageUp,
    PageDown,
    ScrollUp,
    ScrollDown,
}

// State of Vim emulation
pub struct Vim {
    pub mode: Mode,
    pub pending: Input, // Pending input to handle a sequence with two keys like gg
    pub clipboard: ClipboardContext,
}

impl Default for Vim {
    fn default() -> Self {
        Vim::new(Mode::Normal)
    }
}

impl fmt::Debug for Vim {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Vim")
            .field("mode", &self.mode)
            .field("pending", &self.pending)
            .field("clipboard", &"<ClipboardContext omitted>")
            .finish()
    }
}

impl Clone for Vim {
    fn clone(&self) -> Self {
        Self {
            mode: self.mode,
            pending: self.pending.clone(),
            clipboard: ClipboardContext::new().expect("Expected a System ClipboardContext"),
        }
    }
}

impl Vim {
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            pending: Input::default(),
            clipboard: ClipboardContext::new().expect("Failed to initialize clipboard"),
        }
    }

    pub fn with_pending(self, pending: Input) -> Self {
        Self {
            mode: self.mode,
            pending,
            clipboard: ClipboardContext::new().expect("Failed to initialize clipboard"),
        }
    }

    pub fn transition(&mut self, input: Input, textarea: &mut TextArea) -> Transition {
        if input.key == Key::Null {
            return Transition::Nop;
        }

        match self.mode {
            Mode::Normal | Mode::Visual | Mode::Operator(_) => {
                if let Some(transition) = self.handle_normal_input(input, textarea) {
                    return transition;
                }

                // Handle the pending operator
                match self.mode {
                    Mode::Operator('y') => {
                        textarea.copy();
                        self.clipboard.set_contents(textarea.yank_text());
                        Transition::Mode(Mode::Normal)
                    }
                    Mode::Operator('d') => {
                        textarea.cut();
                        self.clipboard.set_contents(textarea.yank_text());
                        Transition::Mode(Mode::Normal)
                    }
                    Mode::Operator('c') => {
                        textarea.cut();
                        self.clipboard.set_contents(textarea.yank_text());
                        Transition::Mode(Mode::Insert)
                    }
                    _ => Transition::Nop,
                }
            }
            Mode::Insert => match input {
                Input { key: Key::Esc, .. }
                | Input {
                    key: Key::Char('c'),
                    ctrl: true,
                    ..
                } => Transition::Mode(Mode::Normal),
                input => {
                    textarea.input(input); // Use default key mappings in insert mode
                    Transition::Mode(Mode::Insert)
                }
            },
            Mode::Recording => Transition::EndRecording,
            Mode::Warning(_) => Transition::Mode(Mode::Normal),
        }
    }

    fn handle_normal_input(
        &mut self,
        input: Input,
        textarea: &mut TextArea<'_>,
    ) -> Option<Transition> {
        match input {
            // motions
            Input {
                key: Key::Char('h'),
                ..
            }
            | Input { key: Key::Left, .. } => {
                textarea.move_cursor(CursorMove::Back);
                None
            }
            Input {
                key: Key::Char('j'),
                ..
            }
            | Input { key: Key::Down, .. } => {
                textarea.move_cursor(CursorMove::Down);
                None
            }
            Input {
                key: Key::Char('k'),
                ..
            }
            | Input { key: Key::Up, .. } => {
                textarea.move_cursor(CursorMove::Up);
                None
            }
            Input {
                key: Key::Char('l'),
                ..
            }
            | Input {
                key: Key::Right, ..
            } => {
                textarea.move_cursor(CursorMove::Forward);
                None
            }
            Input {
                key: Key::Char('w'),
                ..
            } => {
                textarea.move_cursor(CursorMove::WordForward);
                None
            }
            Input {
                key: Key::Char('b'),
                ctrl: false,
                ..
            } => {
                textarea.move_cursor(CursorMove::WordBack);
                None
            }
            Input {
                key: Key::Char('^'),
                ..
            } => {
                textarea.move_cursor(CursorMove::Head);
                None
            }
            Input {
                key: Key::Char('$'),
                ..
            } => {
                textarea.move_cursor(CursorMove::End);
                None
            }
            Input {
                key: Key::Char('e'),
                ctrl: false,
                ..
            } => {
                textarea.move_cursor(CursorMove::WordEnd);
                if matches!(self.mode, Mode::Operator(_)) {
                    textarea.move_cursor(CursorMove::Forward);
                }
                None
            }

            // line edits
            Input {
                key: Key::Char('D'),
                ..
            } => {
                textarea.delete_line_by_end();
                Some(Transition::Mode(Mode::Normal))
            }
            Input {
                key: Key::Char('C'),
                ..
            } => {
                textarea.delete_line_by_end();
                textarea.cancel_selection();
                Some(Transition::Mode(Mode::Insert))
            }
            Input {
                key: Key::Char('x'),
                ..
            } => {
                textarea.delete_next_char();
                Some(Transition::Mode(Mode::Normal))
            }
            Input {
                key: Key::Char('p'),
                ..
            } => {
                textarea.paste();
                Some(Transition::Mode(Mode::Normal))
            }

            // undo/redo
            Input {
                key: Key::Char('u'),
                ctrl: false,
                ..
            } => {
                textarea.undo();
                Some(Transition::Mode(Mode::Normal))
            }
            Input {
                key: Key::Char('r'),
                ctrl: true,
                ..
            } => {
                textarea.redo();
                Some(Transition::Mode(Mode::Normal))
            }

            // insert-mode switches
            Input {
                key: Key::Char('i'),
                ..
            } => {
                textarea.cancel_selection();
                Some(Transition::Mode(Mode::Insert))
            }
            Input {
                key: Key::Char('a'),
                ..
            } => {
                textarea.cancel_selection();
                textarea.move_cursor(CursorMove::Forward);
                Some(Transition::Mode(Mode::Insert))
            }
            Input {
                key: Key::Char('A'),
                ..
            } => {
                textarea.cancel_selection();
                textarea.move_cursor(CursorMove::End);
                Some(Transition::Mode(Mode::Insert))
            }
            Input {
                key: Key::Char('I'),
                ..
            } => {
                textarea.cancel_selection();
                textarea.move_cursor(CursorMove::Head);
                Some(Transition::Mode(Mode::Insert))
            }
            Input {
                key: Key::Char('o'),
                ..
            } => {
                textarea.move_cursor(CursorMove::End);
                textarea.insert_newline();
                Some(Transition::Mode(Mode::Insert))
            }
            Input {
                key: Key::Char('O'),
                ..
            } => {
                textarea.move_cursor(CursorMove::Head);
                textarea.insert_newline();
                textarea.move_cursor(CursorMove::Up);
                Some(Transition::Mode(Mode::Insert))
            }

            // scrolling
            Input {
                key: Key::Char('e'),
                ctrl: true,
                ..
            } => {
                textarea.scroll((1, 0));
                None
            }
            Input {
                key: Key::Char('y'),
                ctrl: true,
                ..
            } => {
                textarea.scroll((-1, 0));
                None
            }
            Input {
                key: Key::Char('d'),
                ctrl: true,
                ..
            } => {
                textarea.scroll(Scrolling::HalfPageDown);
                None
            }
            Input {
                key: Key::Char('u'),
                ctrl: true,
                ..
            } => {
                textarea.scroll(Scrolling::HalfPageUp);
                None
            }
            Input {
                key: Key::Char('f'),
                ctrl: true,
                ..
            } => {
                textarea.scroll(Scrolling::PageDown);
                None
            }
            Input {
                key: Key::Char('b'),
                ctrl: true,
                ..
            } => {
                textarea.scroll(Scrolling::PageUp);
                None
            }

            // visual mode toggles
            Input {
                key: Key::Char('v'),
                ctrl: false,
                ..
            } if self.mode == Mode::Normal => {
                textarea.start_selection();
                Some(Transition::Mode(Mode::Visual))
            }
            Input {
                key: Key::Char('V'),
                ctrl: false,
                ..
            } if self.mode == Mode::Normal => {
                textarea.move_cursor(CursorMove::Head);
                textarea.start_selection();
                textarea.move_cursor(CursorMove::End);
                Some(Transition::Mode(Mode::Visual))
            }
            Input {
                key: Key::Char('y'),
                ctrl: false,
                ..
            } if self.mode == Mode::Visual => {
                textarea.move_cursor(CursorMove::Forward);
                textarea.copy();
                let _ = self.clipboard.set_contents(textarea.yank_text());
                Some(Transition::Mode(Mode::Normal))
            }
            Input {
                key: Key::Char('d'),
                ctrl: false,
                ..
            } if self.mode == Mode::Visual => {
                textarea.move_cursor(CursorMove::Forward);
                textarea.cut();
                let _ = self.clipboard.set_contents(textarea.yank_text());
                Some(Transition::Mode(Mode::Normal))
            }
            Input {
                key: Key::Char('c'),
                ctrl: false,
                ..
            } if self.mode == Mode::Visual => {
                textarea.move_cursor(CursorMove::Forward);
                textarea.cut();
                let _ = self.clipboard.set_contents(textarea.yank_text());
                Some(Transition::Mode(Mode::Insert))
            }
            Input { key: Key::Esc, .. }
            | Input {
                key: Key::Char('v'),
                ..
            } if self.mode == Mode::Visual => {
                textarea.cancel_selection();
                Some(Transition::Mode(Mode::Normal))
            }
            Input { key: Key::Esc, .. } if matches!(self.mode, Mode::Operator(_)) => {
                Some(Transition::Mode(Mode::Normal))
            }

            // special normal-mode keys
            Input { key: Key::Esc, .. } if self.mode == Mode::Normal => Some(Transition::Exit),
            Input {
                key: Key::Char('['),
                ..
            } if self.mode == Mode::Normal => Some(Transition::ScrollUp),
            Input {
                key: Key::Char(']'),
                ..
            } if self.mode == Mode::Normal => Some(Transition::ScrollDown),
            Input {
                key: Key::Char('['),
                shift: true,
                ..
            } if self.mode == Mode::Normal => Some(Transition::PageUp),
            Input {
                key: Key::Char(']'),
                shift: true,
                ..
            } if self.mode == Mode::Normal => Some(Transition::PageDown),
            Input {
                key: Key::Char(']'),
                ctrl: true,
                ..
            } if self.mode == Mode::Normal => Some(Transition::ScrollBottom),
            Input {
                key: Key::Char('['),
                ctrl: true,
                ..
            } if self.mode == Mode::Normal => Some(Transition::ScrollTop),
            Input {
                key: Key::Char('r'),
                ..
            } if self.mode == Mode::Normal => Some(Transition::Mode(Mode::Recording)),
            Input {
                key: Key::Enter, ..
            } if self.mode == Mode::Normal => Some(Transition::Validation),
            Input {
                key: Key::Char('v'),
                ctrl: true,
                ..
            } if self.mode == Mode::Normal => {
                textarea.set_yank_text(
                    self.clipboard
                        .get_contents()
                        .expect("Expected the clipboard Content"),
                );
                textarea.paste();
                Some(Transition::Mode(Mode::Normal))
            }
            Input { key: Key::Tab, .. } if self.mode == Mode::Normal => {
                Some(Transition::Detail(SectionMove::Next))
            }
            Input {
                key: Key::Tab,
                shift: true,
                ..
            } if self.mode == Mode::Normal => Some(Transition::Detail(SectionMove::Previous)),

            // gg / G
            Input {
                key: Key::Char('g'),
                ctrl: false,
                ..
            } if matches!(
                self.pending,
                Input {
                    key: Key::Char('g'),
                    ..
                }
            ) =>
            {
                textarea.move_cursor(CursorMove::Top);
                None
            }
            Input {
                key: Key::Char('G'),
                ctrl: false,
                ..
            } => {
                textarea.move_cursor(CursorMove::Bottom);
                None
            }

            // operator + motion
            Input {
                key: Key::Char(c @ ('y' | 'd' | 'c')),
                ctrl: false,
                ..
            } if self.mode == Mode::Operator(c) => {
                textarea.move_cursor(CursorMove::Head);
                textarea.start_selection();
                let start = textarea.cursor();
                textarea.move_cursor(CursorMove::Down);
                if start == textarea.cursor() {
                    textarea.move_cursor(CursorMove::End);
                }
                None
            }
            Input {
                key: Key::Char(op @ ('y' | 'd' | 'c')),
                ctrl: false,
                ..
            } if self.mode == Mode::Normal => {
                textarea.start_selection();
                Some(Transition::Mode(Mode::Operator(op)))
            }

            // anything else
            input => Some(Transition::Pending(input)),
        }
    }
}
