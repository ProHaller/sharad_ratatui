use tokio::sync::{Mutex, MutexGuard};

pub fn blocking_lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    tokio::runtime::Handle::current().block_on(async { mutex.lock().await })
}
