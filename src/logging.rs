use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};
use once_cell::sync::OnceCell;
use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::PathBuf;

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
            let log_file = self.log_path.join("log.txt");

            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_file) {
                let _ = file.write_all(log_entry.as_bytes());
            }
        }
    }

    fn flush(&self) {}
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

