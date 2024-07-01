use crate::app::{App, AppCommand};
use crate::cleanup::cleanup;
use crossterm::{
    event::{self, Event, KeyCode},
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

use crate::app_state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let (app, command_receiver) = App::new();
    let app = Arc::new(Mutex::new(app));

    // Run the main app loop
    let result = run_app(&mut terminal, app, command_receiver).await;

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
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    loop {
        // Draw the current state of the app
        terminal.draw(|f| {
            let mut app = tokio::task::block_in_place(|| app.blocking_lock());
            ui::draw(f, &mut *app)
        })?;

        // Check if we need to handle input, perform a tick, or process a command
        tokio::select! {
            _ = tokio::time::sleep_until(last_tick + tick_rate) => {
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
                if let Ok(Event::Key(key)) = event.unwrap() {
                    let mut app = app.lock().await;
                    if key.code == KeyCode::Char('q') && app.state == AppState::MainMenu {
                        return Ok(());
                    }

                    // Rescan save files when entering the load game menu
                    if app.state == AppState::MainMenu && key.code == KeyCode::Enter {
                        if app.main_menu_state.selected() == Some(1) { // Assuming "Load Game" is the second option
                            app.available_saves = App::scan_save_files();
                            app.load_game_menu_state.select(Some(0));
                        }
                    }

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
                }
            }
        }

        // Check if we should quit
        if app.lock().await.should_quit {
            return Ok(());
        }
    }
}
