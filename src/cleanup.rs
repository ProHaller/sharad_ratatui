use crossterm::{
    event::DisableMouseCapture,
    execute,
    terminal::{disable_raw_mode, LeaveAlternateScreen},
};
use std::io::stdout;
use std::sync::Once;

static CLEANUP: Once = Once::new();

pub fn cleanup() {
    CLEANUP.call_once(|| {
        let mut stdout = stdout();
        let _ = disable_raw_mode();
        let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
    });
}

pub fn register_cleanup_on_exit() {
    // This will call cleanup when the program exits normally or is interrupted
    unsafe {
        libc::atexit(cleanup_on_exit);
    }
}

extern "C" fn cleanup_on_exit() {
    cleanup();
}

#[cfg(all(unix, not(target_os = "macos")))]
#[link_section = ".init_array"]
pub static CLEANUP_ON_EXIT: extern "C" fn() = cleanup_on_exit;

#[cfg(target_os = "macos")]
#[link_section = "__DATA,__mod_init_func"]
pub static CLEANUP_ON_EXIT: extern "C" fn() = cleanup_on_exit;
