use std::path::PathBuf;

// /error.rs
use derive_more::{Display, From};
use log::error;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

use crate::message::AIMessage;

// TODO: Add Jeremy Chone Error trick https://www.youtube.com/watch?v=j-VQCYP7wyw
pub type Result<T> = core::result::Result<T, Error>;

#[derive(Error, Display, Debug, From)]
pub enum Error {
    Shadowrun(ShadowrunError),
    App(AppError),
    Game(GameError),
    AI(AIError),
    Serialization(serde_json::Error),
    String(String),
    Box(Box<dyn std::error::Error + Send + Sync>),
    IO(std::io::Error),
    RatatuiImage(ratatui_image::errors::Errors),
    Report(color_eyre::eyre::Report),
    Audio(AudioError),
    AISend(SendError<AIMessage>),
    ImageSend(SendError<PathBuf>),
}

impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Error::String(value.to_string())
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
    #[error("Character not found: {:#}", 0)]
    CharacterNotFound(String), // Error when a specified character cannot be found.
                               // Potential additional game-specific errors could be defined here.
}

// Errors related to AI operations are separated into their own enum for clarity.
#[derive(Debug, Error)]
pub enum AIError {
    #[error("OpenAI API error: {:#}", 0)]
    OpenAI(#[from] async_openai::error::OpenAIError), // Errors from the OpenAI API.

    #[error("No message found")]
    NoMessageFound, // Error when expected message content is not found.

    #[error("Failed to parse game state: {:#}", 0)]
    GameStateParseError(String), // Error during parsing of game state.

    #[error("Error handling IO: {:#}", 0)]
    Io(std::io::Error),

    #[error("Thread join error: {:#}", 0)]
    ThreadJoinError(String),
}

#[derive(Debug, Error)]
pub enum AudioError {
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
    FromStringAudio(String),

    #[error("std io AudioError: {:#}", 0)]
    IO(#[from] std::io::Error),

    #[error("OpenAI Error: {:#}", 0)]
    OpenAI(#[from] async_openai::error::OpenAIError),

    #[error("Decode Error: {:#}", 0)]
    Decode(#[from] rodio::decoder::DecoderError),
}

impl From<String> for AudioError {
    fn from(error: String) -> Self {
        AudioError::FromStringAudio(error)
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
