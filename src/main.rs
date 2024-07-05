use crate::app::{App, AppCommand};
use crate::cleanup::cleanup;
use crate::message::{Message, MessageType};
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen, SetSize},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::panic;
use std::{io, sync::Arc, time::Duration};
use tokio::sync::mpsc;
use tokio::{sync::Mutex, time::Instant};

mod ai;
mod ai_response;
mod app;
mod app_state;
mod cleanup;
mod game_state;
mod message;
mod settings;
mod settings_state;
mod ui;

const MIN_WIDTH: u16 = 90;
const MIN_HEIGHT: u16 = 50;

fn ensure_minimum_terminal_size() -> io::Result<()> {
    let (width, height) = crossterm::terminal::size()?;
    if width < MIN_WIDTH || height < MIN_HEIGHT {
        execute!(
            io::stdout(),
            SetSize(MIN_WIDTH.max(width), MIN_HEIGHT.max(height))
        )?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    // Ensure minimum terminal size
    ensure_minimum_terminal_size()?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    // Set up panic hook
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
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create channel for AI responses
    let (ai_sender, ai_receiver) = mpsc::unbounded_channel();

    // Create app and run it
    let (app, command_receiver) = App::new(ai_sender.clone()).await;
    let app = Arc::new(Mutex::new(app));

    // Run the main app loop
    if let Err(err) = run_app(&mut terminal, app, command_receiver, ai_receiver).await {
        println!("Error: {:?}", err);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: Arc<Mutex<App>>,
    mut command_receiver: mpsc::UnboundedReceiver<AppCommand>,
    mut ai_receiver: mpsc::UnboundedReceiver<String>,
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| {
            let mut app = tokio::task::block_in_place(|| app.blocking_lock());
            ui::draw(f, &mut app)
        })?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        tokio::select! {
            _ = tokio::time::sleep(timeout) => {
                let mut app = app.lock().await;
                app.on_tick();
                last_tick = Instant::now();
            }
            event = tokio::task::spawn_blocking(|| {
                if event::poll(Duration::from_millis(0)).unwrap() {
                    event::read()
                } else {
                    Ok(Event::FocusGained)
                }
            }) => {
                if let Ok(Ok(Event::Key(key))) = event {
                    let mut app = app.lock().await;
                    app.on_key(key);
                }
            }
            Some(command) = command_receiver.recv() => {
                let mut app = app.lock().await;
                match command {
                    AppCommand::LoadGame(path) => {
                        if let Err(e) = app.load_game(&path).await {
                            app.add_message(Message::new(format!("Failed to load game: {:?}", e), MessageType::System));
                        }
                    }
                    AppCommand::StartNewGame(save_name) => {
                        if let Err(e) = app.start_new_game(save_name).await {
                            app.add_message(Message::new(format!("Failed to start new game: {:?}", e), MessageType::System));
                        }
                    }
                    AppCommand::ProcessMessage(message) => {
                        if let Err(e) = app.send_message(message).await {
                            app.add_message(Message::new(format!("Failed to process message: {:?}", e), MessageType::System));
                        }
                    }
                    AppCommand::ApiKeyValidationResult(is_valid) => {
                        app.handle_api_key_validation_result(is_valid);
                    }
                }
            }
            Some(ai_response) = ai_receiver.recv() => {
                let mut app = app.lock().await;
                if let Some(last_message) = app.game_content.last() {
                    if last_message.content == "AI is thinking..." && last_message.message_type == MessageType::System {
                        app.game_content.pop();
                    }
                }
                app.handle_ai_response(ai_response);
            }
        }

        if app.lock().await.should_quit {
            return Ok(());
        }
    }
}

// TODO: Add method to check API key validation in settings.rs for better encapsulation

// TODO: Create a helper function to update game content scroll in app.rs to reduce code duplication

// TODO: Add a debug message when invalid API key is detected in app.rs to provide better debugging info

// TODO: Refactor `apply_settings` method in app.rs to use a match statement for cleaner code

// TODO: Add unit tests for `validate_api_key` method in settings.rs to ensure proper functionality

// TODO: Ensure terminal is resized on initialization in main.rs for consistent user experience

// TODO: Add error handling for API key validation in settings.rs to handle invalid responses gracefully

// TODO: Add logging for key events in handle_main_menu_input in app.rs to aid debugging

// TODO: Extract message formatting into a separate function in app.rs to improve readability

// TODO: Add a helper method to switch app states in app.rs to simplify state transitions

// TODO: Add a visual indicator for invalid API key in the UI in api_key_input.rs for better user feedback

// TODO: Validate save name input length in handle_save_name_input in app.rs to avoid excessively long save names

// TODO: Add keyboard shortcuts for main menu options in handle_main_menu_input in app.rs for faster navigation

// TODO: Highlight selected save game file in render_load_game_menu in load_game.rs to improve UX

// TODO: Ensure consistent error messages in handle_api_key_input in app.rs to maintain uniformity

// TODO: Add tests for settings loading and saving in settings.rs to ensure robustness

// TODO: Refactor run_app loop in main.rs to separate concerns and improve readability

// TODO: Add method to clear game content in app.rs for better state management

// TODO: Add helper method to initialize terminal in main.rs to reduce duplication

// TODO: Refactor message history loading in load_game in app.rs to handle large histories efficiently

// TODO: Add a method to fetch the game content as text in app.rs to support different UI components

// TODO: Ensure API key is hidden when entered in api_key_input.rs for security

// TODO: Add placeholder text for API key input in api_key_input.rs for better UX

// TODO: Add a confirmation prompt for exiting the game in app.rs to prevent accidental exits

// TODO: Add constraints to ensure terminal size does not go below the minimum in main.rs for stability

// TODO: Add a method to reset settings to default in settings.rs for easier troubleshooting

// TODO: Refactor state transition logic in handle_key to a separate function in app.rs for clarity

// TODO: Add tests for message serialization and deserialization in ai_response.rs to ensure correctness

// TODO: Refactor draw methods in various UI modules to reduce code duplication and improve maintainability

// TODO: Add a method to render a loading spinner in ui.rs to indicate background operations

// TODO: Add a method to scroll game content to the top in app.rs to support full content review
