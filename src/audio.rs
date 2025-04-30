use crate::{
    ai::GameAI,
    error::{AIError, AudioError, Result},
    message::AIMessage,
    message::Fluff,
};
use async_openai::{
    Audio,
    config::OpenAIConfig,
    types::{CreateSpeechRequestArgs, CreateTranscriptionRequestArgs, SpeechModel, Voice},
};
use chrono::Local;
use cpal::{
    FromSample, Sample, Stream,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use futures::{StreamExt, stream::FuturesOrdered};
use rodio::{Decoder, OutputStream, Sink};
use std::{
    fs::{self, File},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};
use uuid::Uuid;

#[derive(Debug)]
pub enum AudioNarration {
    Generating(GameAI, Fluff, PathBuf),
    Playing(Fluff),
    Paused,
    Stopped,
}

impl AudioNarration {
    pub fn handle_audio(
        &mut self,
        ai_sender: tokio::sync::mpsc::UnboundedSender<AIMessage>,
    ) -> Result<()> {
        match &self {
            AudioNarration::Generating(game_ai, fluff, save_path) => {
                self.generate_narration(
                    game_ai.client.clone(),
                    fluff.clone(),
                    save_path.clone(),
                    ai_sender,
                )?;
            }
            AudioNarration::Playing(fluff) => {
                for file in fluff.dialogue.iter() {
                    if let Some(audio_path) = &file.audio {
                        play_audio(audio_path.clone())?;
                    }
                }
            }
            AudioNarration::Paused => todo!("Need to handle the Paused AudioNarration"),
            AudioNarration::Stopped => {}
        }
        Ok(())
    }

    fn generate_narration(
        &mut self,
        client: async_openai::Client<OpenAIConfig>,
        mut fluff: Fluff,
        save_path: PathBuf,
        ai_sender: tokio::sync::mpsc::UnboundedSender<AIMessage>,
    ) -> Result<()> {
        let handle = tokio::spawn(async move {
            fluff
                .speakers
                .iter_mut()
                .for_each(|speaker| speaker.assign_voice());

            let mut audio_futures = FuturesOrdered::new();

            for (index, fluff_line) in fluff.dialogue.iter_mut().enumerate() {
                let voice = fluff
                    .speakers
                    .iter()
                    .find(|s| s.index == fluff_line.speaker_index)
                    .and_then(|s| s.voice.clone())
                    .expect("Voice not found for speaker");

                let text = fluff_line.text.clone();
                let save_path = save_path.clone();
                let client = client.clone();

                // Generate the audio concurrently, keeping track of the index
                audio_futures.push_back(async move {
                    let result = generate_audio(&client, &save_path, &text, voice).await;
                    (result, index)
                });
            }

            // Process the results in order
            while let Some((result, index)) = audio_futures.next().await {
                if let Ok(path) = result {
                    fluff.dialogue[index].audio = Some(path);
                }
            }
            if let Err(e) =
                ai_sender.send(AIMessage::AudioNarration(AudioNarration::Playing(fluff)))
            {
                panic!("Err sending AudioNarration: {}", e)
            };
        });
        Ok(())
    }
}

pub async fn generate_audio(
    client: &async_openai::Client<OpenAIConfig>,
    save_path: &Path,
    text: &str,
    voice: Voice,
) -> Result<PathBuf> {
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

    let logs_dir = save_path.join("logs");
    fs::create_dir_all(&logs_dir).map_err(AIError::Io)?;

    let uuid = Uuid::new_v4();
    let file_name = format!("{}_{}.mp3", Local::now().format("%Y-%m-%d_%H:%M:%S"), uuid);
    let file_path = logs_dir.join(file_name);
    response
        .save(file_path.to_str().expect("Expected a String"))
        .await
        .map_err(AIError::OpenAI)?;

    Ok(file_path)
}

// HACK: Still need an interruption method
pub fn play_audio(file_path: PathBuf) -> Result<()> {
    let (_stream, stream_handle) =
        OutputStream::try_default().expect("Failed to get output stream");
    let sink = Sink::try_new(&stream_handle).expect("Failed to create audio sink");

    let file = File::open(file_path).expect("Failed to open audio file");
    let source = Decoder::new(BufReader::new(file)).expect("Failed to decode audio");

    sink.append(source);
    sink.sleep_until_end();

    Ok(())
}

pub fn record_audio(is_recording: Arc<AtomicBool>) -> Result<()> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| AudioError::AudioRecordingError("No input device available".into()))?;

    let config = device
        .default_input_config()
        .map_err(|e| AudioError::AudioRecordingError(e.to_string()))?;

    let spec = wav_spec_from_config(&config);
    let home_dir = dir::home_dir().expect("Failed to get home directory");
    let path = home_dir.join("sharad").join("data").join("recording.wav");
    let writer = hound::WavWriter::create(path, spec).map_err(AudioError::Hound)?;
    let writer = Arc::new(Mutex::new(Some(writer)));
    let writer_clone = writer.clone();

    let err_fn = move |err| {
        eprintln!("an error occurred on stream: {}", err);
    };

    let stream = match config.sample_format() {
        cpal::SampleFormat::I8 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i8, i8>(data, &writer_clone),
            err_fn,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i16, i16>(data, &writer_clone),
            err_fn,
            None,
        ),
        cpal::SampleFormat::I32 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i32, i32>(data, &writer_clone),
            err_fn,
            None,
        ),
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<f32, f32>(data, &writer_clone),
            err_fn,
            None,
        ),
        sample_format => {
            return Err(AudioError::AudioRecordingError(format!(
                "Unsupported sample format '{sample_format}'"
            ))
            .into());
        }
    };

    let stream = match stream {
        Ok(stream) => stream,
        Err(e) => return Err(AudioError::CpalBuildStream(e).into()),
    };

    stream.play().map_err(AudioError::CpalPlayStream)?;

    // Recording loop
    while is_recording.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(10));
    }

    // Stop the stream (end recording)
    drop(stream);

    // Finalize the WAV file
    if let Ok(mut guard) = writer.lock() {
        if let Some(writer) = guard.take() {
            writer.finalize().map_err(AudioError::Hound)?;
        }
    }

    Ok(())
}

pub fn start_recording(is_recording: &Arc<AtomicBool>) {
    let is_recording_clone = is_recording.clone();

    thread::spawn(move || {
        if let Err(e) = record_audio(is_recording_clone) {
            eprintln!("Error recording audio: {:?}", e);
        }
    });
}

fn sample_format(format: cpal::SampleFormat) -> hound::SampleFormat {
    if format.is_float() {
        hound::SampleFormat::Float
    } else {
        hound::SampleFormat::Int
    }
}

fn wav_spec_from_config(config: &cpal::SupportedStreamConfig) -> hound::WavSpec {
    hound::WavSpec {
        channels: config.channels() as _,
        sample_rate: config.sample_rate().0 as _,
        bits_per_sample: (config.sample_format().sample_size() * 8) as _,
        sample_format: sample_format(config.sample_format()),
    }
}

type WavWriterHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

fn write_input_data<T, U>(input: &[T], writer: &WavWriterHandle)
where
    T: Sample,
    U: Sample + hound::Sample + FromSample<T>,
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            for &sample in input.iter() {
                let sample: U = U::from_sample(sample);
                writer.write_sample(sample).ok();
            }
        }
    }
}

pub async fn transcribe_audio(client: &async_openai::Client<OpenAIConfig>) -> Result<String> {
    let audio = Audio::new(client);

    let home_dir = dir::home_dir().expect("Failed to get home directory");
    let path = home_dir.join("sharad").join("data").join("recording.wav");
    let recording_path = path;

    match audio
        .transcribe(
            CreateTranscriptionRequestArgs::default()
                .file(recording_path)
                .model("whisper-1")
                .build()
                .map_err(AudioError::OpenAI)?,
        )
        .await
    {
        Ok(transcription) => Ok(transcription.text),
        Err(e) => Err(AudioError::OpenAI(e).into()),
    }
}
