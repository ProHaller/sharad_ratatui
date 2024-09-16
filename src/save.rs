use crate::game_state::GameState;

use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, read_dir, remove_file, write, File, OpenOptions};
use std::io::Write;
use std::path::Path;

pub const SAVE_DIR: &str = "./data/save";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveManager {
    pub available_saves: Vec<String>,
    pub current_save: Option<GameState>,
}

impl Default for SaveManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SaveManager {
    pub fn new() -> Self {
        Self {
            available_saves: Self::scan_save_files(),
            current_save: None,
        }
    }

    pub fn scan_save_files() -> Vec<String> {
        let save_dir = Path::new(SAVE_DIR);
        if !save_dir.exists() {
            return Vec::new();
        }

        read_dir(save_dir)
            .unwrap()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.is_file() && path.extension()? == "json" {
                    path.file_stem()?.to_str().map(String::from)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn load_from_file(mut self, save_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let path = format!("{}/{}.json", SAVE_DIR, save_name);
        let file = File::open(path).map_err(|e| {
            eprintln!("Failed to open file: {}", e);
            e
        })?;

        self.current_save = serde_json::from_reader(file)?;
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("sharad_debug.log")
        {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            let _ = writeln!(
                file,
                "[{}] load_from_file:\n {:#?}",
                timestamp, self.current_save
            );
        }
        Ok(self)
    }

    pub fn save(self) -> Result<(), Box<dyn std::error::Error>> {
        create_dir_all(SAVE_DIR)?;
        let current_save = self.current_save.ok_or("There is no game to save")?;
        let save_path = format!("{}/{}.json", SAVE_DIR, current_save.save_name);
        let serialized = serde_json::to_string_pretty(&current_save)?;
        write(save_path, serialized)?;
        Ok(())
    }

    pub fn delete_save(mut self, save_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let path = format!("{}/{}", SAVE_DIR, save_name);
        match remove_file(path) {
            Ok(()) => {
                self.available_saves = Self::scan_save_files();
                Ok(())
            }
            Err(e) => Err(Box::new(e)),
        }
    }
}
