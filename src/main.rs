use clap::{App, Arg};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
use thiserror::Error;

#[derive(Error, Debug)]
enum AppError {
    #[error("configuration error: {0}")]
    ConfigError(#[from] serde_json::Error),
    #[error("file error: {0}")]
    FileError(#[from] std::io::Error),
    // Add other error types as needed
}

#[derive(Serialize, Deserialize)]
struct ServerResult {
    server: String,
    output: String,
    error: Option<String>,
    duration: f64,
}

#[derive(Serialize, Deserialize)]
struct Config {
    servers: Vec<String>,
    ssh_options: HashMap<String, String>,
    users: HashMap<String, String>,
    // Add other configuration fields here
}

type Result<T> = std::result::Result<T, AppError>;

fn read_config(file_path: &str) -> Result<Config> {
    let file = fs::read_to_string(file_path)?;
    let config: Config = serde_json::from_str(&file)?;
    Ok(config)
}

fn find_config_in_cwd() -> Option<PathBuf> {
    let cwd = env::current_dir().expect("Failed to get current working directory");
    let config_path = cwd.join("russh.json");
    if config_path.exists() {
        Some(config_path)
    } else {
        None
    }
}

fn find_config_in_user_dir() -> Option<PathBuf> {
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

fn prompt_create_default_config() -> Result<Option<PathBuf>> {
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

fn create_default_config(file_path: &str) -> Result<()> {
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

fn run_ssh_command(server: &str, user: &str, command: &str, ssh_options: &str) -> ServerResult {
    let start = Instant::now();
    let output = Command::new("ssh")
            .args(&[ssh_options, &format!("{}@{}", user, server), command])
            .output()
    

    let duration = start.elapsed().as_secs_f64();

    match output {
        Ok(output) => ServerResult {
            server: server.to_string(),
            output: String::from_utf8_lossy(&output.stdout).to_string(),
            error: output
                .status
                .success()
                .then(|| None)
                .unwrap_or(Some(String::from_utf8_lossy(&output.stderr).to_string())),
            duration,
        },
        Err(e) => ServerResult {
            server: server.to_string(),
            output: String::new(),
            error: Some(e.to_string()),
            duration,
        },
    }
}

fn main() {
    let matches = App::new("ruSSH")
        .version("0.1.0")
        .author("Your Name")
        .about("Executes SSH commands on multiple servers")
        .arg(
            Arg::with_name("commands")
                .help("Commands to execute on the servers")
                .required(true)
                .multiple(true),
        )
        .get_matches();

    let commands: Vec<String> = matches
        .values_of("commands")
        .unwrap()
        .map(|s| s.to_string())
        .collect();

    let config_path = find_config_in_cwd()
        .or_else(find_config_in_user_dir)
        .or_else(|| {
            prompt_create_default_config().expect("Failed to handle configuration file creation")
        });

    if let Some(path) = config_path {
        let config =
            read_config(path.to_str().unwrap()).expect("Failed to read configuration file");

        let results = Arc::new(Mutex::new(Vec::new()));
        let mut handles = Vec::new();
        let default_ssh_option = String::new();
        let default_user = String::new();

        for server in &config.servers {
            let ssh_options = config
                .ssh_options
                .get(server)
                .unwrap_or(&default_ssh_option);
            let user = config.users.get(server).unwrap_or(&default_user);

            for command in &commands {
                let server_clone = server.clone();
                let ssh_options_clone = ssh_options.clone();
                let user_clone = user.clone();
                let command_clone = command.clone();
                let results_clone = Arc::clone(&results);

                let handle = thread::spawn(move || {
                    let result = run_ssh_command(
                        &server_clone,
                        &user_clone,
                        &command_clone,
                        &ssh_options_clone,
                    );
                    let mut results = results_clone.lock().unwrap();
                    results.push(result);
                });

                handles.push(handle);
            }
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let mut results = results.lock().unwrap();
        results.sort_by(|a, b| a.server.cmp(&b.server));

        let log_file = File::create("output.log").expect("Unable to create log file");
        let mut log_writer = BufWriter::new(log_file);

        for result in results.iter() {
            if let Some(error) = &result.error {
                println!(
                    "Error from {}: {} (Duration: {:.2}s)",
                    result.server, error, result.duration
                );
                writeln!(
                    log_writer,
                    "Error from {}: {} (Duration: {:.2}s)",
                    result.server, error, result.duration
                )
                .expect("Unable to write to log file");
            } else {
                println!(
                    "Output from {}:\n{}(Duration: {:.2}s)",
                    result.server, result.output, result.duration
                );
                writeln!(
                    log_writer,
                    "Output from {}:\n{}(Duration: {:.2}s)",
                    result.server, result.output, result.duration
                )
                .expect("Unable to write to log file");
            }
        }
    } else {
        println!("Execution completed on all servers.");
    }
}
