use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs;
use std::process::Command;
use std::time::Instant;
use std::env;
use std::thread;
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::io::{Write, BufWriter};
use std::path::Path;
use std::path::PathBuf;
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

fn get_config_path(args: &[String]) -> PathBuf {
    args.get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::config_dir()
                .expect("Failed to find config directory")
                .join("russh/russh.json")
        })
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
    let output = if cfg!(target_os = "windows") {
        Command::new("powershell")
            .args(&["-ExecutionPolicy", "Bypass", "-Command", &format!("ssh {} {}@{} \"{}\"", ssh_options, user, server, command)])
            .output()
    } else {
        Command::new("ssh")
            .args(&[ssh_options, &format!("{}@{}", user, server), command])
            .output()
    };

    let duration = start.elapsed().as_secs_f64();

    match output {
        Ok(output) => ServerResult {
            server: server.to_string(),
            output: String::from_utf8_lossy(&output.stdout).to_string(),
            error: output.status.success().then(|| None).unwrap_or(Some(String::from_utf8_lossy(&output.stderr).to_string())),
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
    let args: Vec<String> = env::args().collect();
    let config_path = get_config_path(&args);
    
    // Check if the config file exists, otherwise create a default one
    if !config_path.exists() {
        create_default_config(config_path.to_str().unwrap())
            .expect("Failed to create default config");
        println!("Default configuration file created at {:?}. Please edit it and run the program again.", config_path);
        return;
    }
    let _config = read_config(config_path.to_str().unwrap()).expect("Failed to read config");

    let args: Vec<String> = env::args().collect();
    let config_path = args.get(1).expect("Usage: program [config_path] [command1] [command2] ...");
    let commands = args.iter().skip(2).cloned().collect::<Vec<_>>();

    if !Path::new(config_path).exists() {
        create_default_config(config_path).expect("Failed to create default config");
        println!("Default configuration file created at {}. Please edit it and run the program again.", config_path);
        return;
    }

    let config = read_config(config_path).expect("Failed to read config");

    // To fix the temporary value issue, define empty defaults outside the loop
    let default_ssh_option = "".to_string();
    let default_user = "".to_string();

    let results = Arc::new(Mutex::new(Vec::new()));

    let mut handles = Vec::new();

    for server in config.servers.iter() {
    let ssh_options = config.ssh_options.get(server).unwrap_or(&default_ssh_option);
    let user = config.users.get(server).unwrap_or(&default_user);

    for command in &commands {
        let server_clone = server.clone();
        let ssh_options_clone = ssh_options.clone();
        let user_clone = user.clone();
        let command_clone = command.clone();

        // Clone the Arc for each thread
        let results_clone = Arc::clone(&results);

        let handle = thread::spawn(move || {
            let result = run_ssh_command(&server_clone, &user_clone, &command_clone, &ssh_options_clone);
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

    // Sort the results if needed
    // For example, sort by server name
    results.sort_by(|a, b| a.server.cmp(&b.server));

    // Setup log file
    let log_file = File::create("output.log").expect("Unable to create log file");
    let mut log_writer = BufWriter::new(log_file);

    // Print and log results
    for result in results.iter() {
        if let Some(error) = &result.error {
            println!("Error from {}: {} (Duration: {:.2}s)", result.server, error, result.duration);
            writeln!(log_writer, "Error from {}: {} (Duration: {:.2}s)", result.server, error, result.duration).expect("Unable to write to log file");
        } else {
            println!("Output from {}:\n{}(Duration: {:.2}s)", result.server, result.output, result.duration);
            writeln!(log_writer, "Output from {}:\n{}(Duration: {:.2}s)", result.server, result.output, result.duration).expect("Unable to write to log file");
        }
    }

    println!("Execution completed on all servers.");
    // Continue with sorting and printing results
}
