// /app.rs
use crate::context::Context;
use crate::tui::{Tui, TuiEvent};
use crate::ui::Component;
use crate::ui::main_menu::MainMenu;
use crate::{
    ai::{GameAI, GameConversationState},
    ai_response::create_user_message,
    assistant::{create_assistant, delete_assistant, get_assistant_id},
    audio::{self, play_audio},
    character::CharacterSheet,
    error::{AppError, Error, Result, ShadowrunError},
    game_state::GameState,
    imager,
    message::{self, AIMessage, GameMessage, Message, MessageType},
    save::{SaveManager, get_save_base_dir},
    settings::Settings,
    settings_state::SettingsState,
    ui::{
        game::{self, HighlightedSection},
        spinner::Spinner,
    },
};

use chrono::Local;
use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures::stream::{FuturesOrdered, StreamExt};
use ratatui::DefaultTerminal;
use ratatui::{layout::Alignment, text::Line, widgets::ListState};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol};
use std::{
    borrow::BorrowMut,
    cell::RefCell,
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::fs::copy;
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::time::sleep;
use tui_input::{Input, InputRequest, backend::crossterm::EventHandler};

pub enum Action {
    Quit,
    LoadGame(PathBuf),
    StartNewGame(String),
    ProcessMessage(String),
    AIResponse(Box<Result<GameMessage>>),
    ApiKeyValidationResult(bool),
    // TODO: Probably don't need the transcription target anymore.
    TranscriptionResult(String, TranscriptionTarget),
    TranscriptionError(String),
    SwitchComponent(Box<dyn Component>),
    SwitchInputMode(InputMode),
}

pub enum TranscriptionTarget {
    UserInput,
    SaveNameInput,
    ImagePrompt,
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub enum InputMode {
    #[default]
    Normal,
    Editing,
    Recording,
}

// TODO: Verify that there is a valid connection internet, else request the user to take action
// after conneecting.
pub struct App<'a> {
    // Application state and control flow
    running: bool,
    component: Box<dyn Component>,
    context: Option<Context<'a>>,

    // --- Global State
    input_mode: InputMode, // TODO: Move it into Input struct
    input: Input,

    image_prompt: Input,
    is_recording: Arc<AtomicBool>,
    clipboard: ClipboardContext,

    // --- Global information
    settings: Settings,
    openai_api_key_valid: bool,
    ai_client: Option<GameAI>,

    // --- UI elements
    spinner: Spinner,
    spinner_active: bool,
    last_spinner_update: Instant,

    game_content_scroll: usize,
    visible_lines: usize,
    total_lines: usize,
    cached_content_len: usize,

    highlighted_section: HighlightedSection, // TODO: Move it into game

    // --- GameState
    save_manager: SaveManager,
    backspace_counter: bool,
    current_save_name: Arc<RwLock<String>>,
    current_game: Option<Arc<Mutex<GameState>>>,
    game_content: RefCell<Vec<message::Message>>,
    cached_game_content: Option<Rc<Vec<(Line<'static>, Alignment)>>>,
    last_known_character_sheet: Option<CharacterSheet>,

    action_sender: mpsc::UnboundedSender<Action>,
    action_receiver: mpsc::UnboundedReceiver<Action>,
    ai_sender: mpsc::UnboundedSender<AIMessage>,
    ai_receiver: mpsc::UnboundedReceiver<AIMessage>,

    // --- Images
    image: Option<StatefulProtocol>,
    image_sender: mpsc::UnboundedSender<PathBuf>,
    image_receiver: mpsc::UnboundedReceiver<PathBuf>,
}

impl<'a> App<'a> {
    pub async fn new() -> Self {
        let (command_sender, command_receiver) = mpsc::unbounded_channel();
        // Set up unbounded channel for AI messages.
        let (ai_sender, ai_receiver) = mpsc::unbounded_channel::<AIMessage>();
        // Set up unbounded channel for images.
        let (image_sender, image_receiver) = mpsc::unbounded_channel::<PathBuf>();
        // Set up unbounded channel for errors.

        let home_dir = dir::home_dir().expect("Failed to get home directory");
        let path = home_dir.join("sharad").join("data").join("settings.json");
        let settings = Settings::load_settings_from_file(path).unwrap_or_default();

        let mut load_game_menu_state = ListState::default();
        load_game_menu_state.select(Some(0));

        let openai_api_key_valid = if let Some(ref api_key) = settings.openai_api_key {
            Settings::validate_api_key(api_key).await
        } else {
            false
        };

        Self {
            running: true,
            component: Box::new(MainMenu::default()),
            context: None,

            openai_api_key_valid,
            ai_client: None,

            input_mode: InputMode::Normal,
            clipboard: ClipboardContext::new().expect("Failed to initialize clipboard"),
            input: Input::default(),
            image_prompt: Input::default(),
            is_recording: Arc::new(AtomicBool::new(false)),

            game_content_scroll: 0,
            cached_game_content: None,
            cached_content_len: 0,
            total_lines: 0,
            visible_lines: 0,

            highlighted_section: HighlightedSection::None,
            spinner: Spinner::new(),
            spinner_active: false,
            last_spinner_update: Instant::now(),
            settings,

            save_manager: SaveManager::new(),
            backspace_counter: false,
            game_content: RefCell::new(Vec::new()),
            current_game: None,
            current_save_name: Arc::new(RwLock::new(String::new())),
            last_known_character_sheet: None,

            action_sender: command_sender,
            action_receiver: command_receiver,
            ai_sender,
            ai_receiver,

            image: None,
            image_sender,
            image_receiver,
        }
    }
    // Asynchronous function to continuously run and update the application.
    pub async fn run(&mut self) -> Result<()> {
        let mut tui = Tui::new()?
            .tick_rate(4.0) // 4 ticks per second
            .frame_rate(30.0); // 30 frames per second

        tui.enter()?; // Starts event handler, enters raw mode, enters alternate screen

        loop {
            tui.draw(|frame| {
                let context = Context {
                    openai_api_key_valid: self.openai_api_key_valid,
                    save_manager: &mut self.save_manager,
                    save_name: "",
                    ai_client: &self.ai_client,
                    settings: &self.settings,
                    clipboard: &self.clipboard,
                    console_messages: &self.game_content,
                    input_mode: &self.input_mode,
                };
                self.component
                    .render(frame.area(), frame.buffer_mut(), &context)
            })?;

            if let Some(event) = tui.next().await {
                // `tui.next().await` blocks till next event
                self.handle_tui_event(event)?;
            };

            if !self.running {
                break;
            }
        }

        tui.exit()?; // stops event handler, exits raw mode, exits alternate screen
        Ok(())

        // self.initialize_ai_client().await?;

        // loop {
        //     tokio::select! {
        //         event_result = tokio::task::spawn_blocking(|| crossterm::event::poll(Duration::from_millis(1))) => {
        //         match event_result {
        //             Ok(Ok(true)) => {
        //                 match crossterm::event::read() {
        //                     Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => {
        //                         self.handle_crossterm_events()?
        //                     }
        //                     Ok(_) => {}, // Ignore non-key events and non-press key events
        //                     Err(e) => {
        //                         eprintln!("Error reading event: {:#?}", e);
        //                     }
        //                 }
        //             }
        //             Ok(Ok(false)) => {}, // No event available
        //             Ok(Err(e)) => {
        //                 eprintln!("Error polling for event: {:#?}", e);
        //             }
        //             Err(e) => {
        //                 eprintln!("Task join error: {:#?}", e);
        //             }
        //         }
        //     }
        //         Some(action) = self.action_receiver.recv() => {
        //             match action {
        //                 Action::ProcessMessage(message) => {
        //                     self.scroll_to_bottom();
        //                 },
        //                 Action::AIResponse(result) => {
        //                     // self.handle_ai_response(*result).await;
        //                     self.scroll_to_bottom();
        //                 },
        //                 Action::LoadGame(save_path) => {
        //                     // if let Err(e) = self.load_game(&save_path).await {
        //                         // self.add_message(Message::new( MessageType::System, format!("Failed to load game: {:#?}", e)));
        //                     // }
        //                 },
        //                 Action::StartNewGame(save_name) => {
        //                     // if let Err(e) = app.lock().await.start_new_game(save_name).await {
        //                     //     app.lock().await.add_message(Message::new( MessageType::System, format!("Failed to start new game: {:#?}", e)));
        //                     // };
        //                 },
        //                 Action::ApiKeyValidationResult(is_valid) => {
        //                     self.handle_api_key_validation_result(is_valid);
        //                 }
        //                 Action::TranscriptionResult(transcription, target) => {
        //                     match target {
        //                         self::TranscriptionTarget::UserInput => {
        //                             // for ch in transcription.chars() {
        //                             //     self.user_input.handle(tui_input::InputRequest::InsertChar(ch));
        //                             // }
        //                         }
        //                         self::TranscriptionTarget::SaveNameInput => {
        //                             // for ch in transcription.chars() {
        //                             //     self.save_name_input.handle(tui_input::InputRequest::InsertChar(ch));
        //                             // }
        //                         }
        //                         self::TranscriptionTarget::ImagePrompt => {
        //                             // for ch in transcription.chars() {
        //                             //     self.image_prompt.handle(tui_input::InputRequest::InsertChar(ch));
        //                             // }
        //                         }
        //                     }
        //                     self.add_debug_message(format!("Transcription successful: {}", transcription));
        //                 }
        //                 Action::TranscriptionError(error) => {
        //                     self.add_message(Message::new(
        //                         MessageType::System,
        //                         format!("Failed to transcribe audio: {}", error),
        //                     ));
        //                     self.add_debug_message(format!("Transcription error: {}", error));
        //                 }
        //                 Action::SwitchComponent(component) => {self.component = component},
        //                 Action::SwitchInputMode(input_mode) => {self.input_mode = input_mode},
        //             }
        //         },
        //             Some(ai_message) = self.ai_receiver.recv() => {
        //         match ai_message {
        //             AIMessage::Debug(debug_message) => {
        //                 self.add_debug_message(debug_message);
        //             },
        //         }
        //     }
        //         Some(image_path) = self.image_receiver.recv() => {
        //             let image_name = image_path.file_name().expect("Expected a Valid path");
        //             let current = self.current_game.clone().expect("Expected a Clone of current_game");
        //             let mut game_state = current.lock().await;
        //             let save_dir = game_state.save_path.clone().expect("Expected a valid path").parent().expect("Expected a parent path").to_path_buf();
        //             let new_image_path = save_dir.join(image_name);
        //             copy(image_path, &new_image_path).await?;
        //             tokio::time::sleep(Duration::from_millis(100)).await;
        //             game_state.image_path = Some(new_image_path.clone().to_path_buf());
        //             self.current_game = Some(Arc::new(Mutex::new(game_state.clone())));
        //             // self.save_current_game().await?;
        //
        //             let _ = self.load_image_from_file(new_image_path);
        //         }
        //     }
        //
        //     self.terminal.draw(|frame| {
        //         let context = Context {
        //             openai_api_key_valid: self.openai_api_key_valid,
        //             save_manager: &self.save_manager,
        //             save_name: "",
        //             ai_client: &self.ai_client,
        //             settings: &self.settings,
        //             clipboard: &self.clipboard,
        //             console_messages: &self.game_content,
        //             input_mode: &self.input_mode,
        //         };
        //         self.component
        //             .render(frame.area(), frame.buffer_mut(), &context)
        //     })?;
        // }
    }

    fn handle_tui_event(&mut self, event: TuiEvent) -> Result<()> {
        match event {
            TuiEvent::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.on_key(key_event)?
            }
            // TODO: Pass the pasted text to the Input
            TuiEvent::Paste(_pasted_text) => {}
            TuiEvent::Mouse(_mouse_event) => {}
            TuiEvent::Key(_) => {}
            TuiEvent::Init => {}
            TuiEvent::Quit => {}
            TuiEvent::Error => {}
            TuiEvent::Closed => {}
            TuiEvent::Tick => {}
            TuiEvent::Render => {}
            TuiEvent::FocusGained => {}
            TuiEvent::FocusLost => {}
            TuiEvent::Resize(_, _) => {}
        }
        Ok(())
    }

    fn on_key(&mut self, key_event: KeyEvent) -> Result<()> {
        if let Some(action) = self.component.on_key(
            key_event,
            // TODO: Should probably not construct a context here.
            Context {
                openai_api_key_valid: self.openai_api_key_valid,
                save_manager: &mut self.save_manager,
                save_name: "",
                ai_client: &self.ai_client,
                settings: &self.settings,
                clipboard: &self.clipboard,
                console_messages: &self.game_content,
                input_mode: &self.input_mode,
            },
        ) {
            self.handle_action(action)?
        };
        Ok(())
    }

    fn handle_action(&mut self, action: Action) -> Result<()> {
        match action {
            Action::ApiKeyValidationResult(bool) => self.openai_api_key_valid = bool,
            Action::SwitchComponent(component) => self.component = component,
            Action::SwitchInputMode(input_mode) => self.input_mode = input_mode,
            Action::Quit => self.quit()?,

            Action::TranscriptionResult(_transcription, _transcription_target) => {}
            Action::TranscriptionError(_) => { /* TODO: handle the error*/ }
            Action::LoadGame(_path_buf) => {}
            Action::StartNewGame(_) => {}
            Action::ProcessMessage(_) => {}
            Action::AIResponse(_game_message) => { /*TODO: Handle gmae_message and pass it to component*/
            }
        }

        Ok(())
    }

    fn quit(&mut self) -> Result<()> {
        self.running = false;
        Ok(())
    }

    // pub fn update_cached_content(&mut self, max_width: usize) {
    //     let parsed_content = game::parse_game_content(self, max_width);
    //     self.cached_game_content = Some(Rc::new(parsed_content));
    //     self.cached_content_len = self.game_content.borrow().len();
    // }

    pub async fn initialize_ai_client(&mut self) -> Result<()> {
        let api_key = self
            .settings
            .openai_api_key
            .as_ref()
            .ok_or(AppError::AIClientNotInitialized)?
            .clone();

        let ai_sender = self.ai_sender.clone();
        let debug_callback = move |message: String| {
            let _ = ai_sender.send(message::AIMessage::Debug(message));
        };

        self.ai_client =
            Some(GameAI::new(api_key, debug_callback, self.image_sender.clone()).await?);

        Ok(())
    }

    // TODO: should probably go to game, or ai sections

    // pub fn process_message(&mut self, message: String) {
    //     let user_message = create_user_message(&self.settings.language.to_string(), &message);
    //     let formatted_message = serde_json::to_string(&user_message).unwrap();
    //
    //     self.start_spinner();
    //
    //     let ai_client = self.ai_client.clone();
    //     let current_game = self.current_game.clone();
    //     let sender = self.command_sender.clone();
    //     todo!();
    //
    //     tokio::spawn(async move {
    //         if let (Some(mut ai), Some(game_state)) = (ai_client, current_game) {
    //             let mut game_state = game_state.lock().await;
    //             let result = ai.send_message(&formatted_message, &mut game_state).await;
    //             let _ = sender.send(Action::AIResponse(Box::new(result)));
    //         } else {
    //             let _ = sender.send(Action::AIResponse(Box::new(Err(Error::from(
    //                 AppError::NoCurrentGame,
    //             )))));
    //         }
    //     });
    // }
    // pub async fn handle_ai_response(&mut self, result: Result<GameMessage>) {
    //     self.stop_spinner();
    //     self.add_debug_message(format!("Spinner: {:#?}", self.spinner_active));
    //
    //     match result {
    //         Ok(game_message) => {
    //             self.add_debug_message(format!(
    //                 "Received game message from AI: {:#?}",
    //                 game_message
    //             ));
    //
    //             let game_message_json = serde_json::to_string(&game_message).unwrap();
    //             self.add_debug_message(format!("Game message: {:#?}", game_message_json.clone()));
    //             self.add_message(Message::new(MessageType::Game, game_message_json.clone()));
    //
    //             if self.settings.audio_output_enabled {
    //                 self.add_debug_message(format!(
    //                     "generating audio from {:#?}",
    //                     game_message.fluff.clone()
    //                 ));
    //                 if let Some(ai_client) = self.ai_client.clone() {
    //                     let mut game_message_clone = game_message.clone();
    //                     let save_name = match self.save_manager.current_save.clone() {
    //                         Some(game_state) => game_state.save_name,
    //                         None => "unknown".to_string(),
    //                     };
    //                     tokio::spawn(async move {
    //                         game_message_clone
    //                             .fluff
    //                             .speakers
    //                             .iter_mut()
    //                             .for_each(|speaker| speaker.assign_voice());
    //
    //                         let mut audio_futures = FuturesOrdered::new();
    //
    //                         for (index, fluff_line) in
    //                             game_message_clone.fluff.dialogue.iter_mut().enumerate()
    //                         {
    //                             let voice = game_message_clone
    //                                 .fluff
    //                                 .speakers
    //                                 .iter()
    //                                 .find(|s| s.index == fluff_line.speaker_index)
    //                                 .and_then(|s| s.voice.clone())
    //                                 .expect("Voice not found for speaker");
    //
    //                             let ai_client = ai_client.clone();
    //                             let text = fluff_line.text.clone();
    //                             let save_name = save_name.clone();
    //
    //                             // Generate the audio in parallel, keeping track of the index
    //                             audio_futures.push_back(async move {
    //                                 let result = audio::generate_audio(
    //                                     &ai_client.client,
    //                                     &save_name,
    //                                     &text,
    //                                     voice,
    //                                 )
    //                                 .await;
    //                                 (result, index)
    //                             });
    //                         }
    //
    //                         // Process the results in order
    //                         while let Some((result, index)) = audio_futures.next().await {
    //                             if let Ok(path) = result {
    //                                 game_message_clone.fluff.dialogue[index].audio = Some(path);
    //                             }
    //                         }
    //
    //                         // Play audio sequentially
    //                         // TODO: Make sure two messages audio are not played at the same time.
    //                         for file in game_message_clone.fluff.dialogue.iter() {
    //                             if let Some(audio_path) = &file.audio {
    //                                 let _status = play_audio(audio_path.clone());
    //                             }
    //                         }
    //                     });
    //                 }
    //             }
    //
    //             // Update the UI
    //             self.cached_game_content = None; // Force recalculation of cached content
    //             self.cached_content_len = 0;
    //             self.scroll_to_bottom();
    //
    //             if let Some(character_sheet) = game_message.character_sheet {
    //                 self.add_debug_message("Updating character sheet".to_string());
    //                 self.update_character_sheet(character_sheet).await;
    //             }
    //             self.add_debug_message("Updated character sheet".to_string());
    //
    //             if let Err(e) = self.save_current_game().await {
    //                 self.add_debug_message(format!("Failed to save game: {:#?}", e));
    //                 self.add_message(Message::new(
    //                     MessageType::System,
    //                     format!("Failed to save game after AI response: {:#?}", e),
    //                 ));
    //             }
    //             self.add_debug_message("saved game".to_string());
    //         }
    //         Err(e) => {
    //             self.add_debug_message(format!("Error: {:#?}", e));
    //             self.add_message(Message::new(
    //                 MessageType::System,
    //                 format!("AI Error: {:#?}", e),
    //             ));
    //         }
    //     }
    // }

    // TODO: Make this go to recording component maybe inside an Input component

    // pub fn start_recording(&mut self) {
    //     self.is_recording.store(true, Ordering::SeqCst);
    //     audio::start_recording(&self.is_recording);
    //     self.input_mode = InputMode::Recording;
    // }
    //
    // pub fn stop_recording(&mut self) {
    //     self.is_recording.store(false, Ordering::SeqCst);
    //
    //     // Wait a bit to ensure the recording has stopped
    //     std::thread::sleep(Duration::from_millis(100));
    //
    //     self.input_mode = InputMode::Normal;
    //
    //     if self.ai_client.is_none() {
    //         self.add_message(Message::new(
    //             MessageType::System,
    //             "AI client not initialized. Cannot transcribe audio.".to_string(),
    //         ));
    //         self.add_debug_message("Transcription failed: AI client not initialized".to_string());
    //         return;
    //     }
    //
    //     let ai_client = self.ai_client.clone();
    //     let state = self.state.clone();
    //     let sender = self.command_sender.clone();
    //
    //     tokio::spawn(async move {
    //         if let Some(ai_client) = ai_client {
    //             match audio::transcribe_audio(&ai_client.client).await {
    //                 Ok(transcription) => {
    //                     let command = match state {
    //                         AppState::InGame => Action::TranscriptionResult(
    //                             transcription,
    //                             TranscriptionTarget::UserInput,
    //                         ),
    //                         AppState::InputSaveName => Action::TranscriptionResult(
    //                             transcription,
    //                             TranscriptionTarget::SaveNameInput,
    //                         ),
    //                         AppState::CreateImage => Action::TranscriptionResult(
    //                             transcription,
    //                             TranscriptionTarget::ImagePrompt,
    //                         ),
    //                         _ => return,
    //                     };
    //                     let _ = sender.send(command);
    //                 }
    //                 Err(e) => {
    //                     let _ = sender.send(Action::TranscriptionError(format!("{}", e)));
    //                 }
    //             }
    //         }
    //     });
    // }

    pub async fn update_save_name(&self, new_name: String) {
        let mut save_name = self.current_save_name.write().await;
        *save_name = new_name;
    }

    // TODO: Make the Game Component and adapt this to its on_key

    // fn handle_in_game_editing(&mut self, key: KeyEvent) {
    //     match key.code {
    //         KeyCode::Enter => {
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
    //                 self.user_input.handle_event(&Event::Key(key));
    //             }
    //         }
    //         _ => {
    //             // Let tui_input handle all other key events
    //             self.user_input.handle_event(&Event::Key(key));
    //         }
    //     }
    // }

    // fn handle_in_game_input(&mut self, key: KeyEvent) {
    //     match self.input_mode {
    //         InputMode::Normal => match key.code {
    //             KeyCode::Char('e') => {
    //                 self.input_mode = InputMode::Editing;
    //             }
    //             KeyCode::Char('r') => {
    //                 self.start_recording();
    //             }
    //             KeyCode::Esc if (self.highlighted_section != HighlightedSection::None) => {
    //                 self.highlighted_section = HighlightedSection::None;
    //             }
    //             KeyCode::Esc => {
    //                 self.game_content.borrow_mut().clear();
    //                 self.current_game = None;
    //                 self.last_known_character_sheet = None;
    //                 self.user_input.reset();
    //                 self.state = AppState::MainMenu;
    //                 self.save_manager.available_saves = SaveManager::scan_save_files();
    //                 self.add_message(Message::new(
    //                     MessageType::System,
    //                     "Game paused. Returned to main menu.".to_string(),
    //                 ))
    //             }
    //             KeyCode::Enter => {
    //                 if !self.user_input.value().is_empty() {
    //                     self.submit_user_input();
    //                 }
    //             }
    //             KeyCode::PageUp => {
    //                 for _ in 0..self.visible_lines {
    //                     self.scroll_up();
    //                 }
    //             }
    //             KeyCode::PageDown => {
    //                 for _ in 0..self.visible_lines {
    //                     self.scroll_down();
    //                 }
    //             }
    //             KeyCode::Up | KeyCode::Char('k') => self.scroll_up(),
    //             KeyCode::Down | KeyCode::Char('j') => self.scroll_down(),
    //
    //             KeyCode::Tab => self.cycle_highlighted_section(),
    //
    //             KeyCode::Home => {
    //                 self.game_content_scroll = 0;
    //             }
    //             KeyCode::End => {
    //                 self.game_content_scroll = self.total_lines.saturating_sub(self.visible_lines);
    //             }
    //             _ => {}
    //         },
    //         InputMode::Editing => match key.code {
    //             KeyCode::Esc => {
    //                 self.input_mode = InputMode::Normal;
    //             }
    //             KeyCode::Enter => {
    //                 self.input_mode = InputMode::Normal;
    //             }
    //             KeyCode::Char('v') => {
    //                 if key.modifiers.contains(KeyModifiers::CONTROL) {
    //                     if let Err(e) = self.handle_paste() {
    //                         self.add_debug_message(format!("Failed to paste: {:#?}", e));
    //                     }
    //                 } else {
    //                     self.user_input.handle_event(&Event::Key(key));
    //                 }
    //             }
    //             _ => {
    //                 self.user_input.handle_event(&Event::Key(key));
    //             }
    //         },
    //         InputMode::Recording => {
    //             match key.code {
    //                 KeyCode::Esc => {
    //                     self.stop_recording();
    //                 }
    //                 _ => {
    //                     // Ignore other keys during recording
    //                 }
    //             }
    //         }
    //     }
    // }

    pub fn handle_api_key_validation_result(&mut self, is_valid: bool) {
        if !is_valid {
            self.settings.openai_api_key = None;
            self.add_message(Message::new(
                MessageType::System,
                "We could not validate your API Key. Please verify your key and internet connection and try again.".to_string(),
            ));
            self.openai_api_key_valid = false;
        } else {
            self.openai_api_key_valid = true;
            self.add_message(Message::new(
                MessageType::System,
                "API Key Validated, Thank you.".to_string(),
            ));
        }
        let home_dir = dir::home_dir().expect("Failed to get home directory");
        let path = home_dir.join("sharad").join("data").join("settings.json");
        if let Err(e) = self.settings.save_to_file(path) {
            self.add_debug_message(format!("Failed to save settings: {:#?}", e));
        }
    }

    // TODO: add this to the game component

    // fn cycle_highlighted_section(&mut self) {
    //     let Some(character_sheet) = self.last_known_character_sheet.as_ref() else {
    //         return;
    //     };
    //
    //     let available_sections = [
    //         Some(HighlightedSection::Backstory),
    //         Some(HighlightedSection::Attributes(0)),
    //         Some(HighlightedSection::Attributes(1)),
    //         Some(HighlightedSection::Attributes(2)),
    //         Some(HighlightedSection::Derived(0)),
    //         Some(HighlightedSection::Derived(1)),
    //         Some(HighlightedSection::Skills),
    //         Some(HighlightedSection::Qualities),
    //         (!character_sheet.cyberware.is_empty()).then_some(HighlightedSection::Cyberware),
    //         (!character_sheet.bioware.is_empty()).then_some(HighlightedSection::Bioware),
    //         Some(HighlightedSection::Resources),
    //         (!character_sheet.inventory.is_empty()).then_some(HighlightedSection::Inventory),
    //         (!character_sheet.contacts.is_empty()).then_some(HighlightedSection::Contact),
    //     ]
    //     .into_iter()
    //     .flatten()
    //     .collect::<Vec<_>>();
    //
    //     if available_sections.is_empty() {
    //         self.highlighted_section = HighlightedSection::None;
    //         return;
    //     }
    //
    //     let current_index = available_sections
    //         .iter()
    //         .position(|s| s == &self.highlighted_section)
    //         .unwrap_or(usize::MAX);
    //
    //     let next_index =
    //         (current_index.wrapping_add(1)) % (available_sections.len().wrapping_add(1));
    //
    //     self.highlighted_section = if next_index < available_sections.len() {
    //         available_sections[next_index].clone()
    //     } else {
    //         HighlightedSection::None
    //     };
    // }

    fn submit_user_input(&mut self) {
        let input = self.input.value().trim().to_string();
        self.start_spinner();

        if input.is_empty() {
            return;
        }

        self.add_message(Message::new(MessageType::User, input.clone()));

        // Send a command to process the message
        if let Err(e) = self.action_sender.send(Action::ProcessMessage(input)) {
            self.add_message(Message::new(
                MessageType::System,
                format!("Error sending message command: {:#?}", e),
            ));
        }

        // Clear the user input
        self.input = Input::default();
        self.scroll_to_bottom();
    }

    // TODO: Make unified and dynamic setting for all settings. cf the Ratatui examples

    pub fn start_spinner(&mut self) {
        self.spinner_active = true;
        self.last_spinner_update = Instant::now();
    }

    pub fn stop_spinner(&mut self) {
        self.spinner_active = false;
    }

    pub fn update_spinner(&mut self) {
        if self.spinner_active && self.last_spinner_update.elapsed() >= Duration::from_millis(100) {
            self.spinner.next_frame();
            self.last_spinner_update = Instant::now();
        }
    }

    pub fn scroll_up(&mut self) {
        if self.game_content_scroll > 0 {
            self.game_content_scroll -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        if self.game_content_scroll < self.total_lines.saturating_sub(self.visible_lines) {
            self.game_content_scroll += 1;
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        // Recalculate total lines
        self.total_lines = self.calculate_total_lines();

        // Update the scroll position
        self.game_content_scroll = self.total_lines.saturating_sub(self.visible_lines);

        // Force UI update
        self.cached_game_content = None;
    }

    fn calculate_total_lines(&self) -> usize {
        self.game_content
            .borrow()
            .iter()
            .map(|message| {
                let wrapped_lines = textwrap::wrap(&message.content, self.visible_lines);
                wrapped_lines.len()
            })
            .sum()
    }

    pub fn update_scroll(&mut self) {
        let max_scroll = self.total_lines.saturating_sub(self.visible_lines);
        self.game_content_scroll = self.game_content_scroll.min(max_scroll);
    }

    pub fn add_debug_message(&self, message: String) {
        if !self.settings.debug_mode {
            return;
        }

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("sharad_debug.log")
        {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            let _ = writeln!(file, "[{}] {}", timestamp, &message);
        }
    }

    pub fn add_message(&self, message: message::Message) {
        self.game_content.borrow_mut().push(message);
    }

    // pub async fn start_new_game(&mut self, save_name: String) -> Result<()> {
    //     // Initialize AI client if not already initialized
    //     if self.ai_client.is_none() {
    //         self.initialize_ai_client().await?;
    //     }
    //
    //     let client = self.ai_client.clone().unwrap().client;
    //     let assistant = match create_assistant(&client, &self.settings.model, &save_name).await {
    //         Ok(assistant) => assistant,
    //         Err(e) => {
    //             println!("{}", e);
    //             return Err(e);
    //         }
    //     };
    //     let assistant_id = &assistant.id;
    //
    //     if let Some(ai) = &self.ai_client {
    //         // Start a new conversation
    //         ai.start_new_conversation(
    //             assistant_id,
    //             GameConversationState {
    //                 assistant_id: assistant_id.to_string(),
    //                 thread_id: String::new(),
    //                 character_sheet: None,
    //             },
    //         )
    //         .await?;
    //
    //         // Get the thread_id from the conversation state
    //         let thread_id = ai
    //             .conversation_state
    //             .lock()
    //             .await
    //             .as_ref()
    //             .ok_or("Conversation state not initialized".to_string())?
    //             .thread_id
    //             .clone();
    //
    //         // Create a new game state
    //         let new_game_state = Arc::new(Mutex::new(GameState {
    //             assistant_id: assistant_id.to_string(),
    //             thread_id,
    //             main_character_sheet: None,
    //             characters: Vec::new(),
    //             save_name: save_name.clone(),
    //             save_path: Some(
    //                 get_save_base_dir()
    //                     .join(&save_name)
    //                     .join(format!("{}.json", &save_name)),
    //             ),
    //             image_path: None,
    //         }));
    //
    //         self.current_game = Some(new_game_state);
    //
    //         // Save the game
    //         self.save_current_game().await?;
    //
    //         self.state = AppState::InGame;
    //         self.add_message(message::Message::new(
    //             message::MessageType::System,
    //             format!("New game '{}' started!", save_name),
    //         ));
    //
    //         // Start the spinner
    //         self.start_spinner();
    //
    //         // Send initial message to start the game
    //         self.process_message(format!(
    //             "Start the game. Respond with the fluff in the following language: {}",
    //             self.settings.language
    //         ));
    //
    //         Ok(())
    //     } else {
    //         Err("AI client not initialized".to_string().into())
    //     }
    // }

    pub async fn update_character_sheet(&mut self, character_sheet: CharacterSheet) {
        if let Some(game_state) = &self.current_game {
            let mut game_state = game_state.lock().await;
            if let Some(ai) = &self.ai_client {
                if let Err(e) = ai.update_character_sheet(&mut game_state, character_sheet) {
                    self.add_message(Message::new(
                        MessageType::System,
                        format!("Failed to update character sheet: {:#?}", e),
                    ));
                } else {
                    self.add_debug_message("Character sheet updated successfully".to_string());
                }
            }
        }
    }

    pub fn load_image_from_file(&mut self, path: PathBuf) -> Result<()> {
        if let Some(current_game_state) = self.current_game.clone() {
            let path_clone = path.clone();
            tokio::spawn(async move {
                current_game_state.lock().await.image_path = Some(path_clone);
            });
        };

        let picker: Picker = Picker::from_query_stdio()?;

        // Open and decode the image file
        match image::ImageReader::open(&path)?.decode() {
            Ok(image) => {
                // Store the image with the new resize protocol
                self.image = Some(picker.new_resize_protocol(image));
                Ok(())
            }
            Err(err) => {
                // Convert ImageError to ShadowrunError using the implemented From trait
                Err(ShadowrunError::from(err).into())
            }
        }
    }

    // TODO: move this to save_manager
    // pub async fn save_current_game(&mut self) -> Result<()> {
    //     let game_state = match &self.current_game {
    //         Some(arc_mutex) => arc_mutex,
    //         None => return Err(AppError::NoCurrentGame.into()),
    //     };
    //
    //     // Clone the Arc to get a new reference
    //     let game_state_clone = Arc::clone(game_state);
    //
    //     // Clone the save_name to own the data
    //     let mut save_manager_clone = self.save_manager.clone();
    //
    //     // Spawn a new task to handle the saving process
    //     tokio::spawn(async move {
    //         // Now we can safely lock the mutex without blocking the main thread
    //         let game_state = game_state_clone.lock().await;
    //         save_manager_clone.current_save = Some(game_state.clone());
    //
    //         let _ = save_manager_clone.save();
    //     });
    //
    //     // Update self.save_manager.current_save with the current game state
    //     let game_state = game_state.lock().await;
    //     self.save_manager.current_save = Some(game_state.clone());
    //
    //     Ok(())
    // }
}
