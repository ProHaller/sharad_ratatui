use crate::game_state::GameState;

use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions, create_dir_all, read_dir, remove_file, write};
use std::io::Write;

use dir;
use std::path::PathBuf;

pub fn get_save_dir() -> PathBuf {
    let mut path = dir::home_dir().unwrap_or("./".into());
    path.push("sharad");
    path.push("save");
    if !&path.exists() {
        let _ = create_dir_all(&path);
    }
    path
}

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
        let save_dir = get_save_dir();
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
        let file_name = format!("{}.json", save_name);
        let path = get_save_dir();
        path.join(file_name);
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

    pub fn save(self) -> Result<(), std::io::Error> {
        let current_save = self.current_save.ok_or(std::io::Error::new(
            std::io::ErrorKind::Other,
            "There is no game to save",
        ))?;
        let save_file = format!("{}.json", current_save.save_name);
        let save_dir = get_save_dir();
        let save_path = save_dir.join(&current_save.save_name).join(save_file);

        let serialized = serde_json::to_string_pretty(&current_save)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        write(save_path, serialized)?;
        Ok(())
    }

    pub fn delete_save(mut self, save_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let save_file = format!("{}.json", save_name);
        let save_dir = get_save_dir();
        let save_path = save_dir.join(save_file);
        let save_logs = save_dir.join("logs");
        let audio_folder_path = save_logs.join("audio");

        match remove_file(save_path) {
            Ok(()) => {
                delete_folder_contents(&audio_folder_path.to_str().unwrap_or(""))?;
                self.available_saves = Self::scan_save_files();
                Ok(())
            }
            Err(e) => Err(Box::new(e)),
        }
    }
}

fn delete_folder_contents(folder_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(folder_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            delete_folder_contents(path.to_str().unwrap())?;
            std::fs::remove_dir(path)?;
        } else {
            std::fs::remove_file(path)?;
        }
    }
    std::fs::remove_dir(folder_path)?;
    Ok(())
}
