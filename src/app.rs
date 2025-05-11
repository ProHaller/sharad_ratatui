use crate::{
    ai::GameAI,
    assistant::create_assistant,
    audio::{self, AudioNarration, Transcription},
    character::{CharacterSheet, CharacterSheetUpdate},
    context::Context,
    error::{Error, Result},
    game_state::GameState,
    imager::load_image_from_file,
    message::{
        AIMessage, GameMessage, Message, MessageType, UserCompletionRequest, create_user_message,
    },
    save::{SaveManager, get_save_base_dir},
    settings::Settings,
    tui::{Tui, TuiEvent},
    ui::{Component, ComponentEnum, api_key_input::ApiKeyInput, game::InGame, main_menu::MainMenu},
};

use async_openai::{Client, config::OpenAIConfig};
use crossterm::event::{KeyEvent, KeyEventKind};
use ratatui::widgets::ListState;
use ratatui_image::{picker::Picker, protocol::StatefulProtocol};
use std::{
    fs::{self, create_dir_all},
    mem,
    path::PathBuf,
};
use tokio::sync::mpsc;

pub enum Action {
    Quit,
    LoadSave(PathBuf),
    CreateNewGame(String),
    SwitchComponent(ComponentEnum),
    SwitchInputMode(InputMode),
    EndRecording,
    AudioNarration(AudioNarration),
}

#[derive(Debug, Default, Clone)]
pub enum InputMode {
    #[default]
    Normal,
    Editing,
    Recording(Transcription),
}

pub struct App {
    // Application state and control flow
    running: bool,
    component: ComponentEnum,
    ai_client: Option<Client<OpenAIConfig>>,
    settings: Settings,
    save_manager: SaveManager,
    input_mode: InputMode,
    audio_narration: AudioNarration,

    // --- Global information
    game_ai: Option<GameAI>,

    // --- Game elements
    messages: Vec<Message>,

    ai_sender: mpsc::UnboundedSender<AIMessage>,
    ai_receiver: mpsc::UnboundedReceiver<AIMessage>,

    // --- Images
    picker: Option<Picker>,
    image: Option<StatefulProtocol>,
    image_sender: mpsc::UnboundedSender<PathBuf>,
    image_receiver: mpsc::UnboundedReceiver<PathBuf>,
}

impl App {
    pub async fn new() -> Self {
        // Set up unbounded channel for AI messages.
        let (ai_sender, ai_receiver) = mpsc::unbounded_channel::<AIMessage>();
        // Set up unbounded channel for images.
        let (image_sender, image_receiver) = mpsc::unbounded_channel::<PathBuf>();
        // Set up unbounded channel for errors.
        let mut load_game_menu_state = ListState::default();
        load_game_menu_state.select(Some(0));

        let settings = Settings::try_load();
        let ai_client;
        let mut game_ai: Option<GameAI> = None;
        if let Some(api_key) = &settings.openai_api_key {
            ai_client = Settings::validate_ai_client(api_key).await;
            game_ai = match GameAI::new(api_key, ai_sender.clone(), image_sender.clone()).await {
                Ok(game_ai) => Some(game_ai),
                Err(_) => None,
            }
        } else {
            ai_client = None
        };

        Self {
            running: true,
            component: ComponentEnum::from(MainMenu::default()),
            ai_client,
            game_ai,
            input_mode: InputMode::Normal,
            messages: Vec::new(),
            ai_sender,
            ai_receiver,
            picker: None,
            image: None,
            image_sender,
            image_receiver,
            settings,
            save_manager: SaveManager::new(),
            audio_narration: AudioNarration::Stopped,
        }
    }
    // Asynchronous function to continuously run and update the application.
    pub async fn run(&mut self) -> Result<()> {
        log::info!("Started the app");

        tokio::spawn(async move {
            audio::warm_up_audio();
        });

        let mut tui = Tui::new()?
            .tick_rate(4.0) // 4 ticks per second
            .frame_rate(30.0); // 30 frames per second

        log::info!("New Tui successfull");

        tui.enter()?; // Starts event handler, enters raw mode, enters alternate screen
        log::info!("Entered Tui");
        let picker = tui.picker;
        self.picker = Some(picker);
        log::info!("Entering run loop");

        let mut context = Context {
            ai_client: &mut self.ai_client.clone(),
            image_sender: self.image_sender.clone(),
            save_manager: &mut self.save_manager.clone(),
            settings: &mut self.settings.clone(),
            messages: &mut self.messages.clone(),
            input_mode: &mut self.input_mode.clone(),
            audio_narration: &mut self.audio_narration.clone(),
        };
        loop {
            tui.draw(|frame| {
                self.component
                    .render(frame.area(), frame.buffer_mut(), &context)
            })?;

            // TODO: improve input cursor position
            let ai_receiver = &mut self.ai_receiver;
            let image_receiver = &mut self.image_receiver;
            tokio::select! {
                Some(event) = tui.next() => {
                    self.handle_tui_event(event, &mut context)?;
                },
                Some(ai_message) = ai_receiver.recv() => {
                    log::info!("Received ai_message: {ai_message:#?}");
                    if let Some(action) = self.handle_ai_message(ai_message)? {
                        self.handle_action(action)?;
                    };
                },
                Some(image_path) = image_receiver.recv() => {
                    log::info!("Received path: {image_path:#?}");
                    self.handle_image(image_path)?;
                },
                else => break,

            }

            if !self.running {
                break;
            }
        }

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
                log::info!("Action::LoadSave: {save_path:#?}");
                self.ai_sender.send(AIMessage::Load(save_path))?;
            }
            Action::CreateNewGame(save_name) => {
                log::info!("Action::CreateNewGame: {save_name:#?}");
                self.ai_sender.send(AIMessage::StartGame(save_name))?;
            }
            // Action::ProcessMessage(message) => {
            //     todo!("Need to ProcessMessage: {}", message)
            // }
            Action::AudioNarration(audio_narration) => {
                log::info!("Action::AudioNarration: {audio_narration:#?}");
                self.audio_narration = audio_narration;
                self.audio_narration.handle_audio(self.ai_sender.clone())?;
            }
            Action::EndRecording => {
                if let InputMode::Recording(transcription) =
                    mem::replace(&mut self.input_mode, InputMode::Normal)
                {
                    tokio::spawn(async move {
                        transcription.input().await;
                    });
                }
                log::debug!("Replaced self.input_mode: {:#?}", self.input_mode);
            }
        }

        Ok(())
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
            AIMessage::Load(save_path) => {
                let game_state = self.load_game_state(&save_path)?;
                self.get_messages(game_state)?;
                None
            }
            AIMessage::Response(game_message) => {
                self.append_ai_response(&game_message);
                if self.settings.audio_output_enabled {
                    Some(Action::AudioNarration(AudioNarration::Generating(
                        self.game_ai.clone().unwrap().clone(),
                        game_message.fluff.clone(),
                        self.component
                            .get_ingame_save_path()
                            .expect("Expected a valid save_path")
                            .clone(),
                    )))
                } else {
                    None
                }
            }
            AIMessage::AudioNarration(audio_narration) => {
                self.audio_narration = audio_narration;
                self.audio_narration.handle_audio(self.ai_sender.clone())?;
                None
            }
            AIMessage::RequestCharacterUpdate(update, character_name) => {
                self.apply_update(&update, character_name)?;
                None
            }
            AIMessage::Save(game_state) => {
                self.save(&game_state)?;
                None
            }
            AIMessage::StartGame(save_name) => {
                self.start_new_game(save_name)?;
                None
            }
            AIMessage::AddCharacter(character_sheet) => {
                self.add_character(character_sheet);
                None
            }
        };
        Ok(result)
    }
    fn handle_tui_event(&mut self, event: TuiEvent, context: &mut Context) -> Result<()> {
        match event {
            TuiEvent::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.on_key(key_event, context)?
            }
            // Maybe I don't need copypasta anymore?
            TuiEvent::Paste(_pasted_text) => {}
            TuiEvent::Mouse(_mouse_event) => {}
            TuiEvent::Key(_) => {}
            TuiEvent::Init => {}
            // TuiEvent::Quit => {}
            TuiEvent::Error => {}
            // TuiEvent::Closed => {}
            TuiEvent::Tick => {}
            TuiEvent::Render => {}
            TuiEvent::FocusGained => {}
            TuiEvent::FocusLost => {}
            TuiEvent::Resize(_, _) => {}
        }
        Ok(())
    }

    // TODO: should implement an image generation spinner
    fn handle_image(&mut self, path: PathBuf) -> Result<()> {
        // Load and store image in self
        let picker = self.picker.expect("Expected a Picker");
        self.image = Some(load_image_from_file(&picker, &path)?);

        // Handle game-specific image loading and saving
        match &mut self.component {
            ComponentEnum::ImageMenu(image_menu) => {
                image_menu.image = Some(load_image_from_file(&picker, &path)?);
            }
            ComponentEnum::InGame(game) => {
                if let Some(save_path) = &game.state.save_path {
                    if let Some(save_dir) = save_path.parent() {
                        let images_dir = save_dir.join("images");
                        create_dir_all(&images_dir)?;

                        if let Some(file_name) = path.file_name() {
                            let img_path = images_dir.join(file_name);
                            fs::copy(&path, &img_path)?;
                            game.image = Some(load_image_from_file(&picker, &img_path)?);
                            game.state.image_path = Some(img_path);
                            self.ai_sender.send(AIMessage::Save(game.state.clone()))?;
                        }
                    }
                }
            }
            _ => {
                unreachable!()
            }
        }

        Ok(())
    }

    fn on_key(&mut self, key_event: KeyEvent, context: &mut Context) -> Result<()> {
        if let Some(action) = self.component.on_key(key_event, context) {
            self.handle_action(action)?
        };
        Ok(())
    }

    fn get_messages(&mut self, game_state: GameState) -> Result<()> {
        let thread_id = game_state.thread_id.clone();
        let ai = self.game_ai.clone().expect("Expected GameAI");
        let sender = self.ai_sender.clone();
        tokio::spawn(async move {
            let all_messages: Vec<Message> = ai
                .fetch_all_messages(&thread_id)
                .await
                .expect("Expected the return of vec messages");
            let messages = all_messages[1..].to_vec();

            match sender.send(AIMessage::Game((messages, ai, game_state))) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("Couldn't send the AIMessage: {:#?}", e)
                }
            };
        });

        Ok(())
    }

    fn quit(&mut self) -> Result<()> {
        self.running = false;
        Ok(())
    }

    fn load_game_state(&mut self, save_path: &PathBuf) -> Result<GameState> {
        self.save_manager.load_from_file(save_path)
    }

    // TODO: Make unified and dynamic setting for all settings. cf the Ratatui examples

    pub fn append_ai_response(&mut self, message: &GameMessage) {
        if let ComponentEnum::InGame(game) = &mut self.component {
            let game_message_json = serde_json::to_string(&message).unwrap();
            game.new_message(&Message::new(MessageType::Game, game_message_json.clone()));
            game.spinner_active = false;
        }
    }
    pub fn apply_update(
        &mut self,
        update: &CharacterSheetUpdate,
        character_name: String,
    ) -> Result<()> {
        if let ComponentEnum::InGame(game) = &mut self.component {
            if let Some(character) = game
                .state
                .characters
                .iter_mut()
                .find(|c| c.name == character_name)
            {
                character.apply_update(update)?;
                if character.main {
                    game.state.main_character_sheet = Some(character.clone());
                }
                self.ai_sender.send(AIMessage::Save(game.state.clone()))?;
            }
        }
        Ok(())
    }

    fn save(&mut self, game_state: &GameState) -> Result<()> {
        if let ComponentEnum::InGame(game) = &mut self.component {
            self.save_manager.save(game_state)?;
            game.state = game_state.clone();
        }
        Ok(())
    }

    pub fn start_new_game(&mut self, save_name: String) -> Result<()> {
        if self.ai_client.is_none() {
            self.component = ComponentEnum::ApiKeyInput(ApiKeyInput::new(&None));
            return Ok(());
        }
        let ai_client = self.ai_client.clone().unwrap();
        let settings = self.settings.clone();
        let game_ai = self.game_ai.clone();
        let ai_sender = self.ai_sender.clone();
        let save_manager = self.save_manager.clone();

        tokio::spawn(async move {
            let assistant = match create_assistant(&ai_client, &settings.model, &save_name).await {
                Ok(assistant) => assistant,
                Err(e) => {
                    log::error!("Failed to create assistant: {:?}", e);
                    return;
                }
            };

            let assistant_id = &assistant.id;

            if let Some(ai) = game_ai {
                let mut game_state = match ai.start_new_conversation(assistant_id, &save_name).await
                {
                    Ok(game_state) => game_state,
                    Err(e) => {
                        log::error!(
                            "Failed to start_new_conversation and get game_state: {:?}",
                            e
                        );
                        return;
                    }
                };

                game_state.save_path = Some(
                    get_save_base_dir()
                        .join(&save_name)
                        .join(format!("{}.json", &save_name)),
                );
                save_manager
                    .save(&game_state)
                    .expect("Expected to save the game");

                if let Err(e) =
                    ai_sender.send(AIMessage::Load(game_state.save_path.clone().unwrap()))
                {
                    log::error!("Failed to send StartGame message: {:?}", e)
                }

                if let Err(e) = ai
                    .send_message(
                        UserCompletionRequest {
                            language: settings.language.to_string(),
                            message: create_user_message(
                                &settings.language.to_string(),
                                "Start the Game",
                            ),
                            state: game_state.clone(),
                        },
                        ai_sender.clone(),
                    )
                    .await
                {
                    log::error!("Failed to send initial game message: {:?}", e)
                }
            } else {
                log::error!("Missing game_ai when starting new game");
            }
        });

        Ok(())
    }

    fn add_character(&mut self, character_sheet: CharacterSheet) {
        if let ComponentEnum::InGame(game) = &mut self.component {
            if character_sheet.main {
                game.state.main_character_sheet = Some(character_sheet.clone());
            }

            if let Some(existing) = game
                .state
                .characters
                .iter_mut()
                .find(|char| char.name == character_sheet.name)
            {
                *existing = character_sheet;
            } else {
                game.state.characters.push(character_sheet);
            }
        }
    }
}
