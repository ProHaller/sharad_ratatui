// /main.rs
// Import necessary modules from the local crate and external crates.
use crate::{
    app::App,
    cleanup::cleanup,
    error::{Result, ShadowrunError},
    message::{AIMessage, Message, MessageType},
};

use app::Action;
use core::cmp::Ordering;
use crossterm::{
    event::{Event, KeyEventKind}, // Event handling from crossterm for input events.
    execute,                      // Helper macro to execute terminal commands.
    terminal::{EnterAlternateScreen, SetSize, enable_raw_mode}, // Terminal manipulation utilities.
};
use ratatui::{Terminal, backend::CrosstermBackend};
use self_update::backends::github::{ReleaseList, Update};
use semver::Version;
use std::{io, panic, path::PathBuf, rc::Rc, sync::Arc, time::Duration};
use tokio::{
    fs::copy,
    sync::{Mutex, mpsc},
    time::{Instant, sleep},
};
use ui::{MIN_HEIGHT, MIN_WIDTH, draw};

mod ai;
mod ai_response;
mod app;
mod assistant;
mod audio;
mod character;
mod cleanup;
mod context;
mod dice;
mod error;
mod game_state;
mod imager;
mod message;
mod save;
mod settings;
mod settings_state;
mod ui;

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
async fn main() -> io::Result<()> {
    let update_result = tokio::task::spawn_blocking(check_for_updates).await?;
    if let Err(e) = update_result {
        println!("Failed to check for updates: {}", e);
    }
    // Set up the terminal in raw mode.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?; // Enter an alternate screen.
    // Ensure terminal dimensions are correct. ensure_minimum_terminal_size()?;

    // Initialize terminal backend.
    let terminal = ratatui::init();

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

    // Initialize the application.
    let mut app = App::new(terminal).await;

    // Run the application and handle errors.
    if let Err(err) = app.run().await {
        eprintln!("Error: {:#?}", err);
    }

    Ok(())
}

fn check_for_updates() -> core::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Checking for updates...");

    let repo_owner = "ProHaller";
    let repo_name = "sharad_ratatui";
    let binary_name = "sharad";
    let current_version = env!("CARGO_PKG_VERSION");

    let releases = ReleaseList::configure()
        .repo_owner(repo_owner)
        .repo_name(repo_name)
        .build()?
        .fetch()?;

    if let Some(release) = releases.first() {
        println!("Newest version found: {}", release.version);

        let latest_version = Version::parse(&release.version)?;
        let current_version = Version::parse(current_version)?;

        match latest_version.cmp(&current_version) {
            Ordering::Greater => {
                println!("Updating to new version: {}", release.version);
                Update::configure()
                    .repo_owner(repo_owner)
                    .repo_name(repo_name)
                    .bin_name(binary_name)
                    .target(self_update::get_target())
                    .show_download_progress(true)
                    .show_output(true)
                    .bin_install_path(
                        std::env::current_exe()?
                            .parent()
                            .expect("Expected a parent Path"),
                    )
                    .current_version(&current_version.to_string())
                    .target_version_tag(&release.version)
                    .build()?
                    .update()?;
            }
            Ordering::Equal => println!("Current version is up to date."),
            Ordering::Less => println!("You're in the future."),
        }
    } else {
        println!("No new updates found.");
    }

    println!();
    Ok(())
}
