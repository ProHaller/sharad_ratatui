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
