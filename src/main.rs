use crate::event::DisableMouseCapture;
use crossterm::{
    event::{self, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    error::Error,
    io,
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
    time::Duration,
};

mod app;
mod ui;

use crate::app::App;

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create a flag to track if we should quit
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Set up SIGINT handler
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;

    // Create app and run it
    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app, running);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    running: Arc<AtomicBool>,
) -> io::Result<()> {
    while running.load(Ordering::SeqCst) {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
                app.on_key(key);
                if app.should_quit {
                    break;
                }
            }
        }
    }
    Ok(())
}
