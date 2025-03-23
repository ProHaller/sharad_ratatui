use tokio::sync::{Mutex, MutexGuard};

pub fn blocking_lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    tokio::runtime::Handle::current().block_on(async { mutex.lock().await })
}

// Constants for minimum terminal size.
pub const MIN_WIDTH: u16 = 40;
pub const MIN_HEIGHT: u16 = 20;
