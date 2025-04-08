// /main.rs
// Import necessary modules from the local crate and external crates.
use crate::app::App;

use core::cmp::Ordering;
use crossterm::{
    execute,           // Helper macro to execute terminal commands.
    terminal::SetSize, // Terminal manipulation utilities.
};
use self_update::backends::github::{ReleaseList, Update};
use semver::Version;
use std::io;
use ui::{MIN_HEIGHT, MIN_WIDTH};

mod ai;
mod ai_response;
mod app;
mod assistant;
mod audio;
mod character;
mod context;
mod dice;
mod error;
mod game_state;
mod imager;
mod message;
mod save;
mod settings;
mod settings_state;
mod tui;
mod ui;

// Entry point for the Tokio runtime.
#[tokio::main]
async fn main() -> io::Result<()> {
    let update_result = tokio::task::spawn_blocking(check_for_updates).await?;
    if let Err(e) = update_result {
        eprintln!("Failed to check for updates: {}", e);
    }
    // Ensure terminal dimensions are correct.
    ensure_minimum_terminal_size()?;

    // Run the application and handle errors.
    if let Err(err) = App::new().await.run().await {
        eprintln!("Error: {:#?}", err);
    }

    Ok(())
}

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
