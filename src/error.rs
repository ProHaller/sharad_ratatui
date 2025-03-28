use derive_more::{Display, From};
use log::error;
use once_cell::sync::Lazy;
use serde_json;
use std::{sync::Arc, time::Instant};
use thiserror::Error;
use tokio::sync::{Mutex, mpsc};

// TODO: Add Jeremy Chone Error trick https://www.youtube.com/watch?v=j-VQCYP7wyw
pub type Result<T> = core::result::Result<T, Error>;

#[derive(Error, Display, Debug, From)]
pub enum Error {
    ShadowrunError(ShadowrunError),
    AppError(AppError),
    GameError(GameError),
    AIError(AIError),
    SerializationError(serde_json::Error),
    StringError(String),
    BoxError(Box<dyn std::error::Error + Send + Sync>),
    IOError(std::io::Error),
    RatatuiImageError(ratatui_image::errors::Errors),
}

static GLOBAL_ERROR_HANDLER: Lazy<Arc<Mutex<Option<ErrorHandler>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

pub async fn initialize_global_error_handler() -> mpsc::UnboundedReceiver<ShadowrunError> {
    let (error_handler, error_receiver) = ErrorHandler::new();
    let mut global = GLOBAL_ERROR_HANDLER.lock().await;
    *global = Some(error_handler);
    error_receiver
}

pub async fn send_global_error(error: ShadowrunError) {
    if let Some(handler) = &*GLOBAL_ERROR_HANDLER.lock().await {
        handler.send_error(error);
    } else {
        error!("Global error handler not initialized: {:?}", error);
    }
}

#[derive(Debug, Clone, Error)]
pub enum ShadowrunError {
    #[error("AI error: {0}")]
    AI(String),
    #[error("Game error: {0}")]
    Game(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("UI error: {0}")]
    UI(String),
    #[error("Audio error: {0}")]
    Audio(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("IO error: {0}")]
    IO(String),
    #[error("OpenAI API error: {0}")]
    OpenAI(String),
    #[error("Image error: {0}")]
    Image(String),
    #[error("Unknown error: {0}")]
    Unknown(String),
}

#[derive(Clone)]
pub struct ErrorMessage {
    pub error: ShadowrunError,
    pub timestamp: Instant,
}

impl ErrorMessage {
    pub fn new(error: ShadowrunError) -> Self {
        Self {
            error,
            timestamp: Instant::now(),
        }
    }
}

pub struct ErrorHandler {
    sender: mpsc::UnboundedSender<ShadowrunError>,
}

impl ErrorHandler {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<ShadowrunError>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (Self { sender }, receiver)
    }

    pub fn send_error(&self, error: ShadowrunError) {
        if let Err(e) = self.sender.send(error.clone()) {
            error!("Failed to send error through channel: {:?}", e);
        }
        match &error {
            ShadowrunError::Network(msg) => error!("Network Error: {}", msg),
            ShadowrunError::Game(msg) => error!("Game Logic Error: {}", msg),
            ShadowrunError::UI(msg) => error!("UI Error: {}", msg),
            ShadowrunError::AI(msg) => error!("AI Error: {}", msg),
            ShadowrunError::Audio(msg) => error!("Audio Error: {}", msg),
            ShadowrunError::Serialization(msg) => error!("Serialization Error: {}", msg),
            ShadowrunError::IO(msg) => error!("IO Error: {}", msg),
            ShadowrunError::OpenAI(msg) => error!("OpenAI Error: {}", msg),
            ShadowrunError::Image(msg) => error!("Image Error: {}", msg),
            ShadowrunError::Unknown(msg) => error!("Unknown Error: {}", msg),
        }
    }
}

#[derive(Debug, Error, Clone)]
pub enum AppError {
    #[error("Shadowrun error: {0}")]
    Shadowrun(#[from] ShadowrunError),

    #[error("AI client not initialized")]
    AIClientNotInitialized,

    #[error("No current game")]
    NoCurrentGame,

    #[error("Conversation not initialized")]
    ConversationNotInitialized,

    #[error("Timeout occurred")]
    Timeout,

    #[error("Max Attempts Reached")]
    MaxAttemptsReached,
}

impl From<String> for ShadowrunError {
    fn from(error: String) -> Self {
        ShadowrunError::Unknown(error)
    }
}

impl From<AppError> for ShadowrunError {
    fn from(error: AppError) -> Self {
        match error {
            AppError::Shadowrun(e) => e,
            AppError::AIClientNotInitialized => {
                ShadowrunError::AI("AI client not initialized".to_string())
            }
            AppError::NoCurrentGame => ShadowrunError::Game("No current game".to_string()),
            AppError::ConversationNotInitialized => {
                ShadowrunError::AI("Conversation not initialized".to_string())
            }
            AppError::Timeout => ShadowrunError::Unknown("Timeout occurred".to_string()),
            AppError::MaxAttemptsReached => {
                ShadowrunError::Unknown("Max attempts reached".to_string())
            }
        }
    }
}

// Implement From traits for easy conversion
impl From<GameError> for ShadowrunError {
    fn from(error: GameError) -> Self {
        ShadowrunError::Game(error.to_string())
    }
}

impl From<AIError> for ShadowrunError {
    fn from(error: AIError) -> Self {
        ShadowrunError::AI(error.to_string())
    }
}

impl From<AudioError> for ShadowrunError {
    fn from(error: AudioError) -> Self {
        ShadowrunError::Audio(error.to_string())
    }
}

impl From<serde_json::Error> for ShadowrunError {
    fn from(error: serde_json::Error) -> Self {
        ShadowrunError::Serialization(error.to_string())
    }
}

impl From<std::io::Error> for ShadowrunError {
    fn from(error: std::io::Error) -> Self {
        ShadowrunError::IO(error.to_string())
    }
}

impl From<image::ImageError> for ShadowrunError {
    fn from(error: image::ImageError) -> Self {
        ShadowrunError::Image(error.to_string())
    }
}

impl From<async_openai::error::OpenAIError> for ShadowrunError {
    fn from(error: async_openai::error::OpenAIError) -> Self {
        ShadowrunError::OpenAI(error.to_string())
    }
}

// Enum for game-specific errors.
#[derive(Debug, Error)]
pub enum GameError {
    #[error("Invalid game state: {:#}", 0)]
    InvalidGameState(String), // Error for invalid game state conditions.

    #[error("Character not found: {:#}", 0)]
    CharacterNotFound(String), // Error when a specified character cannot be found.
                               // Potential additional game-specific errors could be defined here.
}

// Errors related to AI operations are separated into their own enum for clarity.
#[derive(Debug, Error)]
pub enum AIError {
    #[error("OpenAI API error: {:#}", 0)]
    OpenAI(#[from] async_openai::error::OpenAIError), // Errors from the OpenAI API.

    #[error("Conversation not initialized")]
    ConversationNotInitialized, // Error for uninitialized conversation state.

    #[error("Timeout occurred")]
    Timeout, // Error when an AI operation exceeds its time limit.

    #[error("No message found")]
    NoMessageFound, // Error when expected message content is not found.

    #[error("Failed to parse game state: {:#}", 0)]
    GameStateParseError(String), // Error during parsing of game state.

    #[error("Audio recording error: {:#}", 0)]
    AudioRecordingError(String),

    #[error("Audio playback error: {:#}", 0)]
    AudioPlaybackError(String),

    #[error("Error handling IO: {:#}", 0)]
    Io(std::io::Error),

    #[error("Thread join error: {:#}", 0)]
    ThreadJoinError(String),
}

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("Failed to load audio file: {:#}", 0)]
    LoadError(String),

    #[error("hound audio error: {:#}", 0)]
    Hound(#[from] hound::Error),

    #[error("Audio recording error: {:#}", 0)]
    AudioRecordingError(String),

    #[error("Cpal build stream error: {:#}", 0)]
    CpalBuildStream(#[from] cpal::BuildStreamError),

    #[error("Cpal play stream error: {:#}", 0)]
    CpalPlayStream(#[from] cpal::PlayStreamError),

    #[error("Pause Stream Error: {:#}", 0)]
    CpalPauseStream(#[from] cpal::PauseStreamError),

    #[error("Audio from string error: {:#}", 0)]
    FromStringAudioError(String),

    #[error("std io AudioError: {:#}", 0)]
    IO(#[from] std::io::Error),

    #[error("OpenAI Error: {:#}", 0)]
    OpenAI(#[from] async_openai::error::OpenAIError),
}

impl From<String> for AudioError {
    fn from(error: String) -> Self {
        AudioError::FromStringAudioError(error)
    }
}

impl From<ratatui_image::errors::Errors> for ShadowrunError {
    fn from(error: ratatui_image::errors::Errors) -> Self {
        ShadowrunError::Image(error.to_string())
    }
}
impl From<tokio::task::JoinError> for AIError {
    fn from(err: tokio::task::JoinError) -> Self {
        AIError::ThreadJoinError(err.to_string())
    }
}
