use crate::ai::{AIError, AppError};
use async_openai::{
    config::OpenAIConfig,
    types::{CreateSpeechRequestArgs, SpeechModel, Voice},
    Audio,
};
use chrono::Local;
use rodio::{Decoder, OutputStream, Sink};
use std::io::BufReader;
use std::{fs::File, path::PathBuf};
use std::{
    fs::{self},
    path::Path,
};

pub async fn generate_and_play_audio(
    client: &async_openai::Client<OpenAIConfig>,
    text: &str,
) -> Result<(), AIError> {
    let voice = Voice::Onyx;
    let audio = Audio::new(client);

    let response = audio
        .speech(
            CreateSpeechRequestArgs::default()
                .input(text)
                .voice(voice)
                .model(SpeechModel::Tts1)
                .speed(1.3)
                .build()
                .map_err(AIError::OpenAI)?,
        )
        .await
        .map_err(AIError::OpenAI)?;

    let file_name = format!("{}.mp3", Local::now().format("%Y%m%d_%H%M%S"));
    let file_path = Path::new("./data/logs").join(file_name);
    fs::create_dir_all("./data/logs").map_err(AIError::Io)?;
    response
        .save(file_path.to_str().unwrap())
        .await
        .map_err(AIError::OpenAI)?;

    play_audio(file_path.to_str().unwrap().to_string())?;

    Ok(())
}

fn play_audio(file_path: String) -> Result<(), AIError> {
    std::thread::spawn(move || {
        let (_stream, stream_handle) =
            OutputStream::try_default().expect("Failed to get output stream");
        let sink = Sink::try_new(&stream_handle).expect("Failed to create audio sink");

        let file = File::open(file_path).expect("Failed to open audio file");
        let source = Decoder::new(BufReader::new(file)).expect("Failed to decode audio");

        sink.append(source);
        sink.sleep_until_end();
    });

    Ok(())
}

fn record_audio() -> Result<PathBuf, AppError> {
    unimplemented!()
}
