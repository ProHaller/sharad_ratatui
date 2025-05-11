use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};
use once_cell::sync::OnceCell;
use std::collections::HashSet;
use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::save::get_game_data_dir;

#[derive(Debug)]
struct SimpleLogger {
    log_path: PathBuf,
    seen_inputs: Mutex<HashSet<String>>, // Track logged inputs
}

static LOGGER: OnceCell<SimpleLogger> = OnceCell::new();

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let msg = format!("{}", record.args());

            // Only log unique messages (per input)
            let mut seen = self.seen_inputs.lock().unwrap();
            if !seen.insert(msg.clone()) {
                return; // already logged, skip
            }

            let mut log_entry = String::new();
            if record.level() >= Level::Debug {
                log_entry.push_str(
                    &chrono::Local::now()
                        .format("%d/%m/%Y %H:%M::%S \n")
                        .to_string(),
                );
            }
            log_entry.push_str(&format!("{} - {}\n", record.level(), msg));
            let log_file = self.log_path.join("log.txt");

            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_file) {
                let _ = file.write_all(log_entry.as_bytes());
            }
        }
    }

    fn flush(&self) {}
}

pub fn init() -> Result<(), SetLoggerError> {
    let log_path = get_game_data_dir();

    create_dir_all(&log_path).expect("Could not create log path");

    LOGGER
        .set(SimpleLogger {
            log_path,
            seen_inputs: Mutex::new(HashSet::new()),
        })
        .expect("Logger already set");

    log::set_logger(LOGGER.get().unwrap()).map(|()| log::set_max_level(LevelFilter::Debug))
}

