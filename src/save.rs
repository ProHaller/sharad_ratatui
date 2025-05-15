// /save.rs
use crate::{assistant::delete_assistant, error::Result, game_state::GameState};

use async_openai::{Client, config::OpenAIConfig};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, create_dir_all, read_dir, remove_dir_all, remove_file, write},
    path::PathBuf,
};

pub fn get_game_data_dir() -> PathBuf {
    let path = get_game_dir().join("data");
    if !&path.exists() {
        if let Err(e) = create_dir_all(&path) {
            log::error!("Could not create path: {e:#?}");
        }
    }
    path
}

pub fn clean_recording_temp_dir() {
    let path = get_game_data_dir().join("temp_logs");
    if let Err(e) = fs::remove_dir_all(path) {
        log::error!("Failed to delete temp_logs: {e:#?}");
    }
}

pub fn get_save_base_dir() -> PathBuf {
    let path = get_game_dir().join("save");
    if !&path.exists() {
        if let Err(e) = create_dir_all(&path) {
            log::error!("Could not create path: {e:#?}");
        }
    }
    path
}
fn get_game_dir() -> PathBuf {
    let path = dir::home_dir()
        .expect("Failed to get home directory")
        .join("sharad");
    if !&path.exists() {
        if let Err(e) = create_dir_all(&path) {
            log::error!("Could not create path: {e:#?}");
        }
    }
    path
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveManager {
    pub available_saves: Vec<PathBuf>,
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

    pub fn load_from_file(&self, save_path: &PathBuf) -> Result<GameState> {
        let file = File::open(save_path).map_err(|e| {
            log::error!("Failed to open file: {e:#?}");
            e
        })?;

        let save: GameState = serde_json::from_reader(file)?;
        Ok(save)
    }

    pub fn save(&self, current_save: &GameState) -> Result<()> {
        if let Some(save_path) = current_save.save_path.clone() {
            create_dir_all(save_path.parent().expect("Expected a parent path"))?;
            let serialized = serialize_save(current_save)?;
            write(save_path, serialized)?;
        } else {
            let save_dir = get_save_base_dir();
            let game_save_dir = save_dir.join(&current_save.save_name);
            create_dir_all(&game_save_dir)?;
            let mut current_save = current_save.clone();
            current_save.save_path =
                Some(game_save_dir.join(format!("{}.json", current_save.save_name)));
            let serialized = serialize_save(&current_save)?;
            write(
                current_save.save_path.expect("Expected Valide save_path"),
                serialized,
            )?;
        }

        Ok(())
    }

    pub fn delete_save(mut self, save_path: &PathBuf, api_key: &str) -> Result<()> {
        if let Ok(game) = self.load_from_file(save_path) {
            let client = Client::with_config(OpenAIConfig::new().with_api_key(api_key));
            tokio::spawn(async move {
                delete_assistant(&client, &game.assistant_id).await;
            });
        };
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

fn serialize_save(current_save: &GameState) -> Result<String> {
    let serialized = serde_json::to_string_pretty(&current_save)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    Ok(serialized)
}

#[test]
fn test_get_save_paths() {
    let base_save_dir = get_save_base_dir();
    let save_files = SaveManager::get_save_paths(base_save_dir);
    println!("{:#?}", save_files);
    assert!(!save_files.is_empty());
}
