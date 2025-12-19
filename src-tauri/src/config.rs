use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OutputConfig {
    pub name: String,
    pub volume: f32,
    pub muted: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub input_volume: f32,
    pub input_muted: bool,
    pub outputs: Vec<OutputConfig>,
}

impl AppConfig {
    fn default_config() -> Self {
        Self {
            input_volume: 1.0,
            input_muted: false,
            outputs: Vec::new(),
        }
    }
}

pub fn get_config_path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|p| p.join("config.json"))
}

pub fn save_config(app: &AppHandle, config: AppConfig) -> Result<(), String> {
    let path = get_config_path(app).ok_or("Failed to get config path")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_config(app: &AppHandle) -> AppConfig {
    let path = match get_config_path(app) {
        Some(p) => p,
        None => return AppConfig::default_config(),
    };

    if !path.exists() {
        return AppConfig::default_config();
    }

    match fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or(AppConfig::default_config()),
        Err(_) => AppConfig::default_config(),
    }
}
