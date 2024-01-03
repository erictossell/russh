use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

// If you're using a custom Result type or error types from main.rs
use crate::{AppError, Result};

impl From<toml::de::Error> for AppError {
    fn from(err: toml::de::Error) -> Self {
        AppError::TomlDeserializationError(err)
    }
}

impl From<toml::ser::Error> for AppError {
    fn from(err: toml::ser::Error) -> Self {
        AppError::TomlSerializationError(err)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub servers: Vec<String>,
    pub ssh_options: HashMap<String, String>,
    pub users: HashMap<String, String>,
    // Add other configuration fields here
}

pub fn read_config(file_path: &str) -> Result<Config> {
    let file = fs::read_to_string(file_path)?;
    let config: Config = toml::from_str(&file)?;
    Ok(config)
}

pub fn find_config_in_cwd() -> Option<PathBuf> {
    let cwd = env::current_dir().expect("Failed to get current working directory");
    let config_path = cwd.join("russh.toml");
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
                if path.is_file() && path.file_name()?.to_str()?.starts_with("russh.toml") {
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
        .ok_or(AppError::File(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Config directory not found",
        )))?
        .join("russh/russh.toml");

    println!(
        "Configuration file not found. Do you want to create a default user file at {:?}? [Y/n]",
        default_path
    );
    let mut response = String::new();
    io::stdin()
        .read_line(&mut response)
        .map_err(AppError::File)?;

    if response.trim().to_lowercase().starts_with('y') {
        create_default_config(default_path.to_str().ok_or(AppError::File(
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
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let example_config = Config {
        servers: vec!["example.server.com".to_string()],
        ssh_options: HashMap::from([("example.server.com".to_string(), "-p 22".to_string())]),
        users: HashMap::from([("example.server.com".to_string(), "example".to_string())]),
    };
    let example_config_bytes = toml::to_string_pretty(&example_config)?;
    fs::write(file_path, example_config_bytes)?;
    Ok(())
}

#[cfg(test)]
mod config_tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn create_temp_config(file_name: &str, content: &str) -> String {
        let path = Path::new(file_name);
        fs::write(path, content).expect("Failed to write temp config file");
        path.to_str().unwrap().to_string()
    }

    #[test]
    fn test_read_config() {
        let config_content = r#"
            servers = ["test.server.com"]
            [ssh_options]
            "test.server.com" = "-p 22"
            [users]
            "test.server.com" = "user"
        "#;
        let file_path = create_temp_config("russh.toml", config_content);
        let config = read_config(&file_path).expect("Failed to read config");
        assert_eq!(config.servers, vec!["test.server.com"]);
        assert_eq!(config.ssh_options["test.server.com"], "-p 22");
        assert_eq!(config.users["test.server.com"], "user");
    }
    #[test]
    fn test_find_config_in_cwd() {
        let config_content = r#"
servers = ["test.server.com"]
[ssh_options]
"test.server.com" = "-p 22"
[users]
"test.server.com" = "user"
"#;
        let _ = create_temp_config("russh.toml", config_content);

        let config_path = find_config_in_cwd().expect("Failed to find config in CWD");
        assert!(config_path.exists());
    }
}
