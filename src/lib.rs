// /lib.rs

pub mod ai;
pub mod app;
pub mod assistant;
pub mod audio;
pub mod character;
pub mod context;
pub mod dice;
pub mod error;
pub mod game_state;
pub mod imager;
pub mod logging;
pub mod message;
pub mod rig;
pub mod save;
pub mod settings;
pub mod settings_state;
pub mod tui;
pub mod ui;

// Re-export commonly used items for easier access
pub use ai::*;
pub use character::*;
pub use error::*;
pub use game_state::*;
pub use message::*;
pub use rig::*;
pub use ui::*;
