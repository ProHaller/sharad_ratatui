use crate::app::{App, AppCommand};
use crate::cleanup::cleanup;
use crate::message::MessageType;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{error::Error, io, sync::Arc, time::Duration};
use tokio::sync::mpsc;
use tokio::{sync::Mutex, time::Instant};

mod ai;
mod app;
mod app_state;
mod cleanup;
mod game_state;
mod message;
mod settings;
mod settings_state;
mod ui;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create channel for AI responses
    let (ai_sender, ai_receiver) = mpsc::unbounded_channel();

    // Create app and run it
    let (app, command_receiver) = App::new(ai_sender.clone());
    let app = Arc::new(Mutex::new(app));

    // Run the main app loop
    let result = run_app(&mut terminal, app, command_receiver, ai_receiver).await;

    // Restore terminal
    cleanup();

    if let Err(err) = result {
        println!("{:?}", err)
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
        // Draw the current state of the app
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
                            app.add_system_message(format!("Failed to load game: {:?}", e));
                        }
                    }
                    AppCommand::StartNewGame => {
                        if let Err(e) = app.start_new_game().await {
                            app.add_system_message(format!("Failed to start new game: {:?}", e));
                        }
                    }
                    AppCommand::ProcessMessage(message) => {
                        if let Err(e) = app.send_message(message).await {
                            app.add_system_message(format!("Failed to process message: {:?}", e));
                        }
                    }
                }
            }
            Some(ai_response) = ai_receiver.recv() => {
                let mut app = app.lock().await;
                // Remove the "AI is thinking..." message
                if let Some(last_message) = app.game_content.last() {
                    if last_message.content == "AI is thinking..." && last_message.message_type == MessageType::System {
                        app.game_content.pop();
                    }
                }
                app.add_game_message(ai_response);
            }
        }

        // Check if we should quit
        if app.lock().await.should_quit {
            return Ok(());
        }
    }
}
