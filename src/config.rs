use serde::{Deserialize, Serialize};
use serde_json;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

// If you're using a custom Result type or error types from main.rs
use crate::{AppError, Result};

#[derive(Serialize, Deserialize)]
pub struct Config {
    servers: Vec<String>,
    ssh_options: HashMap<String, String>,
    users: HashMap<String, String>,
    // Add other configuration fields here
}

pub fn read_config(file_path: &str) -> Result<Config> {
    let file = fs::read_to_string(file_path)?;
    let config: Config = serde_json::from_str(&file)?;
    Ok(config)
}

pub fn find_config_in_cwd() -> Option<PathBuf> {
    let cwd = env::current_dir().expect("Failed to get current working directory");
    let config_path = cwd.join("russh.json");
    if config_path.exists() {
        Some(config_path)
    } else {
        None
    }
}

pub fn find_config_in_user_dir() -> Option<PathBuf> {
    dirs::config_dir().and_then(|path| {
        let russh_dir = path.join("russh");
        if russh_dir.is_dir() {
            std::fs::read_dir(russh_dir).ok()?.find_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.is_file() && path.file_name()?.to_str()?.starts_with("russh.json") {
                    Some(path)
                } else {
                    None
                }
            })
        } else {
            None
        }
    })
}

pub fn prompt_create_default_config() -> Result<Option<PathBuf>> {
    let default_path = dirs::config_dir()
        .ok_or(AppError::FileError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Config directory not found",
        )))?
        .join("russh/russh.json");

    println!(
        "Configuration file not found. Do you want to create a default one at {:?}? [Y/n]",
        default_path
    );
    let mut response = String::new();
    io::stdin()
        .read_line(&mut response)
        .map_err(|e| AppError::FileError(e))?; // Changed this line

    if response.trim().to_lowercase().starts_with('y') {
        create_default_config(default_path.to_str().ok_or(AppError::FileError(
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Failed to convert path to string",
            ),
        ))?)?;
        Ok(Some(default_path))
    } else {
        Ok(None)
    }
}

pub fn create_default_config(file_path: &str) -> Result<()> {
    let path = PathBuf::from(file_path);

    // Create directories if they do not exist
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let example_config = Config {
        servers: vec!["example.server.com".to_string()],
        ssh_options: HashMap::from([("example.server.com".to_string(), "-p 22".to_string())]),
        users: HashMap::from([("example.server.com".to_string(), "example".to_string())]),
        // Add other configuration fields here
    };
    let example_config_bytes = serde_json::to_vec_pretty(&example_config)?;
    fs::write(file_path, example_config_bytes)?;
    Ok(())
}
