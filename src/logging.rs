use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};
use once_cell::sync::OnceCell;
use tokio::fs;
use tokio::io::AsyncWriteExt;

#[derive(Debug)]
struct SimpleLogger {
    log_path: PathBuf,
}

static LOGGER: OnceCell<SimpleLogger> = OnceCell::new();

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let log_entry = format!("{} - {}\n", record.level(), record.args());
            let log_path = self.log_path.clone();

            // Spawn a task to not block
            tokio::spawn(async move {
                if let Err(e) = append_log(&log_path, &log_entry).await {
                    eprintln!("Failed to write log: {:?}", e);
                }
            });
        }
    }

    fn flush(&self) {}
}

async fn append_log(log_dir: &Path, entry: &str) -> std::io::Result<()> {
    let log_file = log_dir.join("log.txt");

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)
        .await?;

    file.write_all(entry.as_bytes()).await?;
    Ok(())
}

pub fn init() -> Result<(), SetLoggerError> {
    let log_path = dir::home_dir()
        .expect("Failed to get home directory")
        .join("sharad")
        .join("data");

    create_dir_all(&log_path).expect("Could not create log path");

    LOGGER
        .set(SimpleLogger { log_path })
        .expect("Logger already set");

    log::set_logger(LOGGER.get().unwrap()).map(|()| log::set_max_level(LevelFilter::Debug))
}

