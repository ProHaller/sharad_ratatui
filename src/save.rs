use crate::game_state::GameState;

use chrono::Local;
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions, create_dir_all, read_dir, remove_dir_all, remove_file, write},
    io::Write,
    path::PathBuf,
};

use dir;

pub fn get_save_base_dir() -> PathBuf {
    let mut path = dir::home_dir().expect("Failed to get home directory");
    path.push("sharad");
    path.push("save");
    if !&path.exists() {
        let _ = create_dir_all(&path);
    }
    path
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveManager {
    pub available_saves: Vec<PathBuf>,
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

    pub fn scan_save_files() -> Vec<PathBuf> {
        let save_dir = get_save_base_dir();
        Self::get_save_paths(save_dir)
    }

    fn get_save_paths(last_dir: PathBuf) -> Vec<PathBuf> {
        let mut path_vec: Vec<PathBuf> = Vec::new();
        let reccursive_path_vec: Vec<PathBuf> = read_dir(last_dir)
            .expect("Expected a ReadDir directory content")
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.is_file() && path.extension()? == "json" {
                    Some(path)
                } else if path.is_dir() {
                    path_vec.extend(Self::get_save_paths(path));
                    None
                } else {
                    None
                }
            })
            .collect();
        path_vec.extend(reccursive_path_vec);
        path_vec
    }

    pub fn load_from_file(
        mut self,
        save_path: &PathBuf,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(save_path).map_err(|e| {
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
        if let Some(save_path) = current_save.save_path.clone() {
            create_dir_all(save_path.parent().expect("Expected a parent path"))?;
            let serialized = serde_json::to_string_pretty(&current_save)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            write(save_path, serialized)?;
        } else {
            let save_dir = get_save_base_dir();
            let game_save_dir = save_dir.join(&current_save.save_name);
            create_dir_all(&game_save_dir)?;
            let mut current_save = current_save;
            current_save.save_path =
                Some(game_save_dir.join(format!("{}.json", current_save.save_name)));
            let serialized = serde_json::to_string_pretty(&current_save)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            write(
                current_save.save_path.expect("Expected Valide save_path"),
                serialized,
            )?;
        }

        Ok(())
    }

    pub fn delete_save(mut self, save_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(save_dir) = save_path.parent() {
            if save_dir != get_save_base_dir() {
                remove_dir_all(save_dir)?;
                self.available_saves = Self::scan_save_files();
                Ok(())
            } else {
                remove_file(save_path)?;
                Ok(())
            }
        } else {
            panic!("This save should not be here! {}", save_path.display());
        }
    }
}

#[test]
fn test_get_save_paths() {
    let base_save_dir = get_save_base_dir();
    let save_files = SaveManager::get_save_paths(base_save_dir);
    println!("{:#?}", save_files);
    assert!(!save_files.is_empty());
}
