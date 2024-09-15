// Import necessary modules from the local crate and external crates.
use crate::app::{App, AppCommand};
use crate::cleanup::cleanup;
use crate::message::{AIMessage, Message, MessageType};

use crossterm::{
    event::{Event, KeyEventKind}, // Event handling from crossterm for input events.
    execute,                      // Helper macro to execute terminal commands.
    terminal::{enable_raw_mode, EnterAlternateScreen, SetSize}, // Terminal manipulation utilities.
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::panic; // Panic handling for cleanup.
use std::{io, sync::Arc, time::Duration}; // Standard I/O and concurrency utilities.
use tokio::sync::mpsc; // Asynchronous message passing channel.
use tokio::time::sleep;
use tokio::{sync::Mutex, time::Instant};

// Modules are declared which should be assumed to be part of the application architecture.
pub mod ai;
pub mod ai_response;
pub mod app;
pub mod app_state;
pub mod audio;
pub mod character;
pub mod cleanup;
pub mod dice;
pub mod error;
pub mod game_state;
pub mod image;
pub mod message;
pub mod save;
pub mod settings;
pub mod settings_state;
pub mod ui;
pub mod utils;

// Constants for minimum terminal size.
const MIN_WIDTH: u16 = 100;
const MIN_HEIGHT: u16 = 40;

// Function to ensure the terminal size meets minimum requirements.
fn ensure_minimum_terminal_size() -> io::Result<()> {
    let (width, height) = crossterm::terminal::size()?; // Get current size of the terminal.
                                                        // If the current size is less than minimum, resize to the minimum required.
    if width < MIN_WIDTH || height < MIN_HEIGHT {
        execute!(
            io::stdout(),
            SetSize(MIN_WIDTH.max(width), MIN_HEIGHT.max(height))
        )?;
    }
    Ok(())
}

// Entry point for the Tokio runtime.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up the terminal in raw mode.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?; // Enter an alternate screen.

    // Ensure terminal dimensions are correct.
    ensure_minimum_terminal_size()?;

    // Initialize terminal backend.
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Set panic hook for cleanup and better panic info.
    panic::set_hook(Box::new(|panic_info| {
        cleanup();
        if let Some(location) = panic_info.location() {
            println!(
                "Panic occurred in file '{}' at line {}",
                location.file(),
                location.line(),
            );
        }
        if let Some(message) = panic_info.payload().downcast_ref::<&str>() {
            println!("Panic message: {}", message);
        }
    }));

    // Set up unbounded channel for AI messages.
    let (ai_sender, ai_receiver) = mpsc::unbounded_channel::<AIMessage>();

    // Initialize the application.
    let (app, command_receiver) = App::new(ai_sender).await;
    let app = Arc::new(Mutex::new(app));

    // Run the application and handle errors.
    if let Err(err) = run_app(&mut terminal, app, command_receiver, ai_receiver).await {
        eprintln!("Error: {:#?}", err);
    }

    Ok(())
}

// Asynchronous function to continuously run and update the application.
async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: Arc<Mutex<App>>,
    mut command_receiver: mpsc::UnboundedReceiver<AppCommand>,
    mut ai_receiver: mpsc::UnboundedReceiver<AIMessage>,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(16);
    let _ai_client = app.lock().await.initialize_ai_client().await;

    loop {
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        tokio::select! {
            _ = sleep(timeout) => {
                let mut app = app.lock().await;
                app.on_tick();
            }
            event_result = tokio::task::spawn_blocking(|| crossterm::event::poll(Duration::from_millis(1))) => {
                match event_result {
                    Ok(Ok(true)) => {
                        match crossterm::event::read() {
                            Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                                let mut app = app.lock().await;
                                app.handle_input(key);
                            }
                            Ok(_) => {}, // Ignore non-key events and non-press key events
                            Err(e) => {
                                eprintln!("Error reading event: {:#?}", e);
                            }
                        }
                    }
                    Ok(Ok(false)) => {}, // No event available
                    Ok(Err(e)) => {
                        eprintln!("Error polling for event: {:#?}", e);
                    }
                    Err(e) => {
                        eprintln!("Task join error: {:#?}", e);
                    }
                }
            }
            Some(command) = command_receiver.recv() => {
                match command {
                    AppCommand::ProcessMessage(message) => {
                        let mut app = app.lock().await;
                        app.process_message(message);
                        app.scroll_to_bottom();
                        drop(app);
                    },
                    AppCommand::AIResponse(result) => {
                        let mut app = app.lock().await;
                        app.handle_ai_response(result).await;
                        app.scroll_to_bottom();
                    },
                    AppCommand::LoadGame(save_name) => {
                        if let Err(e) = app.lock().await.load_game(&save_name).await {
                            app.lock().await.add_message(Message::new( MessageType::System, format!("Failed to load game: {:#?}", e)));
                        }
                    },
                    AppCommand::StartNewGame(save_name) => {
                        let app = app.clone();
                        if let Err(e) = app.lock().await.start_new_game(save_name).await {
                            app.lock().await.add_message(Message::new( MessageType::System, format!("Failed to start new game: {:#?}", e)));
                        };
                    },
                    AppCommand::ApiKeyValidationResult(is_valid) => {
                        let mut app = app.lock().await;
                        app.handle_api_key_validation_result(is_valid);
                    }
                    AppCommand::TranscriptionResult(transcription, target) => {
                        let mut app = app.lock().await;
                        match target {
                            app::TranscriptionTarget::UserInput => {
                                for ch in transcription.chars() {
                                    app.user_input.handle(tui_input::InputRequest::InsertChar(ch));
                                }
                            }
                            app::TranscriptionTarget::SaveNameInput => {
                                for ch in transcription.chars() {
                                    app.save_name_input.handle(tui_input::InputRequest::InsertChar(ch));
                                }
                            }
                            app::TranscriptionTarget::ImagePrompt => {
                                for ch in transcription.chars() {
                                    app.image_prompt.handle(tui_input::InputRequest::InsertChar(ch));
                                }
                            }
                        }
                        app.add_debug_message(format!("Transcription successful: {}", transcription));
                    }
                    AppCommand::TranscriptionError(error) => {
                        let app = app.lock().await;
                        app.add_message(Message::new(
                            MessageType::System,
                            format!("Failed to transcribe audio: {}", error),
                        ));
                        app.add_debug_message(format!("Transcription error: {}", error));
                    }
                }
            },
            Some(ai_message) = ai_receiver.recv() => {
                let mut app = app.lock().await;
                match ai_message {
                    AIMessage::Debug(debug_message) => {
                        app.add_debug_message(debug_message);
                    },
                    AIMessage::Response(response) => {
                        if let Some(last_message) = app.game_content.borrow().last() {
                            if last_message.message_type == MessageType::System {
                                app.game_content.borrow_mut().pop();
                            }
                        }
                        app.handle_ai_response(response).await;
                    }
                }
            }
        }

        terminal.draw(|f| {
            let mut app = tokio::task::block_in_place(|| app.blocking_lock());
            ui::draw(f, &mut app)
        })?;

        if app.lock().await.should_quit {
            return Ok(());
        }

        // Ensure consistent tick rate
        let elapsed = last_tick.elapsed();
        if elapsed < tick_rate {
            tokio::time::sleep(tick_rate - elapsed).await;
        }
        last_tick = Instant::now();
    }
}
