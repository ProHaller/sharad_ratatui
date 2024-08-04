use serde_json;
use thiserror::Error;

// Enum for handling various application-level errors.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("AI error: {:#}", 0)]
    AI(#[from] AIError), // Errors related to AI operations.

    #[error("Game error: {:#}", 0)]
    Game(#[from] GameError), // Errors specific to game logic or state.

    #[error("Serialization error: {:#}", 0)]
    Serialization(#[from] serde_json::Error), // Errors related to data serialization.

    #[error("IO error: {:#}", 0)]
    IO(#[from] std::io::Error), // Input/output errors.

    #[error("AI client not initialized")]
    AIClientNotInitialized, // Specific error when the AI client is not properly initialized.

    #[error("No current game")]
    NoCurrentGame, // Error when no game session is active.

    #[error("OpenAI API error: {:#}", 0)]
    OpenAI(#[from] async_openai::error::OpenAIError), // Errors from the OpenAI API.

    #[error("Conversation not initialized")]
    ConversationNotInitialized, // Error for uninitialized conversation state.

    #[error("Timeout occurred")]
    Timeout, // Error when an operation exceeds its allotted time.

    #[error("No message found")]
    NoMessageFound, // Error when no message is found where one is expected.

    #[error("Max Attempts Reached")]
    MaxAttemptsReached,

    #[error("Failed to parse game state: {:#}", 0)]
    GameStateParseError(String), // Error for issues when parsing game state.

    #[error("Character sheet update error: {:#}", 0)]
    CharacterSheetUpdateError(String),

    #[error("Audio Hound error: {:#}", 0)]
    HoundError(String),
}

impl From<String> for AppError {
    fn from(error: String) -> Self {
        AppError::CharacterSheetUpdateError(error)
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

    #[error("Audio from string error: {:#}", 0)]
    FromStringAudioError(String),

    #[error("std io AudioError: {:#}", 0)]
    IO(#[from] std::io::Error),
}

impl From<String> for AudioError {
    fn from(error: String) -> Self {
        AudioError::FromStringAudioError(error)
    }
}

impl From<tokio::task::JoinError> for AIError {
    fn from(err: tokio::task::JoinError) -> Self {
        AIError::ThreadJoinError(err.to_string())
    }
}
