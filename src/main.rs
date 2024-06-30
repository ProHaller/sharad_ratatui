use crate::ai::{GameAI, GameConversationState};
use crate::app::App;
use crate::cleanup::cleanup;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{error::Error, io, sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::Instant};

mod ai;
mod app;
mod cleanup;
mod ui;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let (mut app, mut message_receiver) = App::new();
    let app = Arc::new(Mutex::new(app));

    // Initialize the AI conversation
    {
        let mut app_lock = app.lock().await;
        if let Some(ai_client) = &mut app_lock.ai_client {
            if ai_client.conversation_state.is_none() {
                let assistant_id = "asst_4kaphuqlAkwnsbBrf482Z6dR"; // Set your assistant_id here
                ai_client
                    .start_new_conversation(
                        assistant_id,
                        GameConversationState {
                            assistant_id: assistant_id.to_string(),
                            thread_id: String::new(), // Placeholder, will be set by the conversation
                            player_health: 100,
                            player_gold: 0,
                        },
                    )
                    .await?;
            }
        }
    }

    // Spawn a task to handle AI messages
    let ai_app = app.clone();
    tokio::spawn(async move {
        while let Some(message) = message_receiver.recv().await {
            let mut app = ai_app.lock().await;
            if let Some(ai) = &mut app.ai_client {
                match ai.send_message(&message).await {
                    Ok(response) => {
                        app.add_game_message(response);
                    }
                    Err(e) => {
                        app.add_system_message(format!("Error: {:?}", e));
                    }
                }
            }
        }
    });

    // Run the main app loop
    let result = run_app(&mut terminal, app).await;

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
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    loop {
        // Draw the current state of the app
        terminal.draw(|f| {
            let mut app = tokio::task::block_in_place(|| app.blocking_lock());
            ui::draw(f, &mut *app)
        })?;

        // Check if we need to handle input or perform a tick
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
                    if key.code == KeyCode::Char('q') && app.state == app::AppState::MainMenu {
                        return Ok(());
                    }
                    app.on_key(key);
                }
            }
        }

        // Check if we should quit
        if app.lock().await.should_quit {
            return Ok(());
        }
    }
}
