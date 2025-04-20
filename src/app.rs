use crate::audio::AudioNarration;
use crate::ui::ComponentEnum;
use crate::{ai, error::Error};
use crate::{message::Message, message::MessageType, message::UserCompletionRequest};
// /app.rs
use crate::context::{self, Context};
use crate::{
    ai::GameAI,
    error::{AppError, Result, ShadowrunError},
    game_state::GameState,
    message::{self, AIMessage, GameMessage},
    save::SaveManager,
    settings::Settings,
    tui::{Tui, TuiEvent},
    ui::{Component, game::InGame, main_menu::MainMenu},
};

use async_openai::types::RunObject;
use copypasta::ClipboardContext;
use crossterm::cursor;
use crossterm::event::{KeyEvent, KeyEventKind};
use ratatui::widgets::ListState;
use ratatui_image::{picker::Picker, protocol::StatefulProtocol};
use std::any::Any;
use std::io::Cursor;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use tokio::sync::mpsc;

pub enum Action {
    Quit,
    LoadSave(PathBuf),
    CreateNewGame(String),
    StartGame(InGame),
    ProcessMessage(String),
    AIResponse(GameMessage),
    // TODO: Probably don't need the transcription target anymore.
    SwitchComponent(ComponentEnum),
    SwitchInputMode(InputMode),
    AudioNarration(AudioNarration),
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
    component: ComponentEnum,
    context: Option<Context<'a>>,
    openai_api_key_valid: bool,
    settings: Settings,
    save_manager: SaveManager,
    input_mode: InputMode,
    audio_narration: AudioNarration,

    // --- Global information
    ai_client: Option<GameAI>,

    // --- Game elements
    game: Option<InGame>,
    messages: Vec<Message>,

    ai_sender: mpsc::UnboundedSender<AIMessage>,
    ai_receiver: mpsc::UnboundedReceiver<AIMessage>,

    // --- Images
    picker: Option<Picker>,
    image: Option<StatefulProtocol>,
    image_sender: mpsc::UnboundedSender<PathBuf>,
    image_receiver: mpsc::UnboundedReceiver<PathBuf>,
}

impl<'a> App<'a> {
    pub async fn new() -> Self {
        // Set up unbounded channel for AI messages.
        let (ai_sender, ai_receiver) = mpsc::unbounded_channel::<AIMessage>();
        // Set up unbounded channel for images.
        let (image_sender, image_receiver) = mpsc::unbounded_channel::<PathBuf>();
        // Set up unbounded channel for errors.
        let mut load_game_menu_state = ListState::default();
        load_game_menu_state.select(Some(0));

        let settings = Settings::load().expect("Could not read settings");
        let openai_api_key_valid: bool;
        let mut ai: Option<GameAI> = None;
        if let Some(api_key) = &settings.openai_api_key {
            openai_api_key_valid = Settings::validate_api_key(api_key).await;
            ai = match GameAI::new(api_key, ai_sender.clone(), image_sender.clone()).await {
                Ok(ai) => Some(ai),
                Err(_) => None,
            }
        } else {
            openai_api_key_valid = false
        };

        Self {
            running: true,
            component: ComponentEnum::from(MainMenu::default()),
            ai_client: ai,
            context: None,
            input_mode: InputMode::Normal,
            game: None,
            messages: Vec::new(),
            ai_sender,
            ai_receiver,

            picker: None,
            image: None,
            image_sender,
            image_receiver,
            openai_api_key_valid,
            settings,
            save_manager: SaveManager::new(),
            audio_narration: AudioNarration::Stopped,
        }
    }
    // Asynchronous function to continuously run and update the application.
    pub async fn run(&mut self) -> Result<()> {
        let mut tui = Tui::new()?
            .tick_rate(4.0) // 4 ticks per second
            .frame_rate(30.0); // 30 frames per second

        tui.enter()?; // Starts event handler, enters raw mode, enters alternate screen
        let picker = tui.picker;
        self.picker = Some(picker);

        loop {
            tui.draw(|frame| {
                let context = Context {
                    openai_api_key_valid: self.openai_api_key_valid,
                    save_manager: &mut self.save_manager,
                    settings: &mut self.settings,
                    clipboard: ClipboardContext::new().expect("Failed to initialize clipboard"),
                    messages: &self.messages,
                    input_mode: &self.input_mode,
                    audio_narration: &mut self.audio_narration,
                };
                self.component
                    .render(frame.area(), frame.buffer_mut(), &context)
            })?;

            // TODO: improve input cursor position
            match self.input_mode {
                InputMode::Editing => tui.terminal.show_cursor()?,
                _ => tui.terminal.hide_cursor()?,
            };

            tokio::select! {
                Some(event) = tui.next() => {
                    self.handle_tui_event(event)?;
                },
                Some(ai_message) = self.next_ai_message() => {
                    if let Some(action) = self.handle_ai_message(ai_message)? {
                        self.handle_action(action)?;
                    };
                },
                else => break,

            }

            if !self.running {
                break;
            }
        }

        tui.exit()?; // stops event handler, exits raw mode, exits alternate screen
        Ok(())

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

    pub async fn next_ai_message(&mut self) -> Option<AIMessage> {
        self.ai_receiver.recv().await
    }

    fn handle_ai_message(&mut self, ai_message: AIMessage) -> Result<Option<Action>> {
        let result: Option<Action> = match ai_message {
            AIMessage::Game((messages, ai, state)) => {
                self.component = ComponentEnum::from(InGame::new(
                    state,
                    &self.picker.expect("Expected a Picker from app"),
                    ai,
                    messages,
                ));
                None
            }
            AIMessage::Image(image_path) => {
                self.image_sender.send(image_path);
                None
            }
            AIMessage::Load(save_path) => {
                let game_state = self.load_game_state(&save_path)?;
                self.get_messages(game_state)?;
                None
            }
            AIMessage::Response(game_message) => {
                self.append_ai_response(&game_message);
                if self.settings.audio_output_enabled {
                    Some(Action::AudioNarration(AudioNarration::Generating(
                        self.ai_client.clone().unwrap().clone(),
                        game_message.fluff.clone(),
                        self.component.get_ingame_save_path().unwrap().clone(),
                    )))
                } else {
                    None
                }
            }
            AIMessage::NewMessage => None,
            AIMessage::AudioNarration(audio_narration) => {
                self.audio_narration = audio_narration;
                self.audio_narration.handle_audio(self.ai_sender.clone());
                None
            }
        };
        Ok(result)
    }
    fn handle_tui_event(&mut self, event: TuiEvent) -> Result<()> {
        match event {
            TuiEvent::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.on_key(key_event)?
            }
            // TODO: Pass the pasted text to the Input
            // Maybe I don't need copypasta anymore?
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
                settings: &mut self.settings,
                clipboard: ClipboardContext::new().expect("Failed to initialize clipboard"),
                messages: &self.messages,
                input_mode: &self.input_mode,
                audio_narration: &mut self.audio_narration,
            },
        ) {
            self.handle_action(action)?
        };
        Ok(())
    }

    fn handle_action(&mut self, action: Action) -> Result<()> {
        match action {
            Action::SwitchComponent(component) => self.component = component,
            Action::SwitchInputMode(input_mode) => {
                self.input_mode = input_mode;
            }
            Action::Quit => self.quit()?,
            Action::LoadSave(save_path) => {
                self.ai_sender.send(AIMessage::Load(save_path));
            }
            Action::CreateNewGame(_) => {}
            Action::ProcessMessage(_) => {}
            Action::AIResponse(_game_message) => { /*TODO: Handle game_message and pass it to component*/
            }
            Action::StartGame(game) => {
                // self.game = Some(game);
                self.component = ComponentEnum::from(game);
            }
            Action::AudioNarration(audio_narration) => {
                self.audio_narration = audio_narration;
                self.audio_narration.handle_audio(self.ai_sender.clone())?;
            }
        }

        Ok(())
    }

    fn get_messages(&mut self, game_state: GameState) -> Result<()> {
        let thread_id = game_state.thread_id.clone();
        let ai = self.ai_client.clone().expect("Expected GameAI");
        let sender = self.ai_sender.clone();
        tokio::spawn(async move {
            let messages = ai
                .fetch_all_messages(&thread_id)
                .await
                .expect("Expected the return of vec messages");
            sender.send(AIMessage::Game((messages, ai, game_state)));
        });

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

        // let ai_sender = self.ai_sender.clone();
        // let debug_callback = move |message: String| {
        //     let _ = ai_sender.send(message::AIMessage::Debug(message));
        // };

        self.ai_client =
            Some(GameAI::new(&api_key, self.ai_sender.clone(), self.image_sender.clone()).await?);

        Ok(())
    }

    fn load_game_state(&mut self, save_path: &PathBuf) -> Result<GameState> {
        self.save_manager.load_from_file(save_path)
    }

    // TODO: should probably go to game, or ai sections

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

    // pub async fn update_save_name(&self, new_name: String) {
    //     let mut save_name = self.current_save_name.write().await;
    //     *save_name = new_name;
    // }

    // TODO: Make unified and dynamic setting for all settings. cf the Ratatui examples

    pub fn append_ai_response(&mut self, message: &GameMessage) {
        if let ComponentEnum::InGame(game) = &mut self.component {
            let game_message_json = serde_json::to_string(&message).unwrap();
            game.new_message(&Message::new(MessageType::Game, game_message_json.clone()));
        }
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

    // pub async fn update_character_sheet(&mut self, character_sheet: CharacterSheet) {
    //     if let Some(game_state) = &self.current_game {
    //         let mut game_state = game_state.lock().await;
    //         if let Some(ai) = &self.ai_client {
    //             if let Err(e) = ai.update_character_sheet(&mut game_state, character_sheet) {
    //                 self.add_message(Message::new(
    //                     MessageType::System,
    //                     format!("Failed to update character sheet: {:#?}", e),
    //                 ));
    //             } else {
    //                 self.add_debug_message("Character sheet updated successfully".to_string());
    //             }
    //         }
    //     }
    // }

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
