mod config;
mod ssh;
use crate::config::{
    find_config_in_cwd, find_config_in_user_dir, prompt_create_default_config, read_config,
};
use crate::ssh::run_ssh_command;

use ansi_term::Color::{Blue, Green, Red, Yellow};
use argh::FromArgs;

use crate::ssh::ServerResult;
use std::fs::File;
use std::io::{self, BufWriter, IsTerminal, Write}; // Use std::io::Write and others
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use thiserror::Error;
#[derive(Error, Debug)]
enum AppError {
    #[error("file error: {0}")]
    File(#[from] std::io::Error),
    #[error("generic error: {0}")]
    Generic(String),
    #[error("toml error: {0}")]
    TomlDeserializationError(toml::de::Error),
    #[error("toml error: {0}")]
    TomlSerializationError(toml::ser::Error),
    // Add other error types as needed
}

/// executes SSH commands on multiple servers.
/// This is the main configuration for the command line interface.
#[derive(FromArgs, PartialEq, Debug)]
struct Cli {
    /// specify the commands that should be executed on the remote servers.
    /// These are the actual SSH commands that will be run on each server.
    #[argh(positional)]
    commands: Vec<String>,

    /// optional: specify the relative path to the russh.json file.
    /// If not provided, a default path or other logic will be used.
    #[argh(option, short = 'c')]
    config_file: Option<String>,
}

type Result<T> = std::result::Result<T, AppError>;

fn parse_cli_args() -> Cli {
    argh::from_env()
}

fn run_application(cli: Cli) -> Result<()> {
    let commands = cli.commands;

    let (tx, rx): (mpsc::Sender<ServerResult>, Receiver<ServerResult>) = mpsc::channel();
    thread::spawn(move || {
        display_outputs(rx);
    });
    let config_path = if let Some(config_path) = cli.config_file {
        let path = PathBuf::from(&config_path);
        if path.exists() {
            path
        } else {
            eprintln!("Specified configuration file not found: {}", config_path);
            return Err(AppError::Generic(
                "Configuration file not found".to_string(),
            ));
        }
    } else {
        match find_config_in_cwd()
            .or_else(find_config_in_user_dir)
            .or_else(|| match prompt_create_default_config() {
                Ok(Some(path)) => Some(path),
                Ok(None) => {
                    eprintln!("Configuration file not found. Exiting.");
                    None
                }
                Err(e) => {
                    eprintln!("Error creating default configuration: {}", e);
                    None
                }
            }) {
            Some(path) => path,
            None => {
                return Err(AppError::Generic(
                    "Configuration path not found".to_string(),
                ))
            }
        }
    };

    let config_path_str = config_path.to_str().unwrap_or_else(|| {
        eprintln!("Invalid path.");
        std::process::exit(1);
    });
    let config = match read_config(config_path_str) {
        Ok(cfg) => Arc::new(cfg),
        Err(e) => {
            eprintln!("Failed to read configuration file: {}", e);
            std::process::exit(1);
        }
    };

    println!("Processing commands...");
    let results = Arc::new(Mutex::new(Vec::<ServerResult>::new()));
    //let results_clone_for_display = Arc::clone(&results);
    let mut handles = Vec::new();

    let mut all_success = true;
    let mut any_success = false;
    for server in &config.servers {
        let server_arc = Arc::new(server.clone());
        let ssh_options_arc = Arc::new(
            config
                .ssh_options
                .get(server)
                .unwrap_or(&String::new())
                .clone(),
        );
        let user_arc = Arc::new(config.users.get(server).unwrap_or(&String::new()).clone());

        for command in &commands {
            let command_arc = Arc::new(command.clone());
            //let results_arc = Arc::clone(&results);

            let server_ref = Arc::clone(&server_arc);
            let ssh_options_ref = Arc::clone(&ssh_options_arc);
            let user_ref = Arc::clone(&user_arc);
            let command_ref = Arc::clone(&command_arc);

            let tx_clone = tx.clone();
            let handle = thread::spawn(move || {
                run_ssh_command(
                    &server_ref,
                    &user_ref,
                    &command_ref,
                    &ssh_options_ref,
                    tx_clone,
                );
            });
            handles.push(handle);
        }
    }

    for handle in handles {
        handle.join().unwrap();
    }

    //let mut results = results.lock().unwrap();

    //results.sort_by(|a, b| a.server.cmp(&b.server));

    let mut log_path = dirs::config_dir()
        .ok_or_else(|| AppError::Generic("Unable to find the config directory".to_string()))?;
    log_path.push("russh");
    std::fs::create_dir_all(&log_path).map_err(AppError::File)?;
    log_path.push("russh.log");

    // Create or open the log file
    let log_file = File::create(log_path).map_err(AppError::File)?;

    let mut log_writer = BufWriter::new(log_file);
    let results_guard = results.lock().unwrap();
    for result in results_guard.iter() {
        if result.success {
            any_success = true;
        } else {
            all_success = false;
        }
        let formatted_duration = format!("{:.2}s", result.duration);

        let duration_color = if result.duration <= 3.0 {
            Green
        } else if result.duration <= 10.0 {
            Yellow
        } else {
            Red
        };

        println!(
            "{} - {}: ",
            Blue.paint(&result.server),
            duration_color.paint(&formatted_duration)
        );

        println!("{}", &result.output);

        // Writing to log file (without color)
        writeln!(
            log_writer,
            "{} - {}:\n{}",
            result.server, formatted_duration, result.output
        )
        .expect("Unable to write to log file");
    }

    if all_success {
        println!(
            "{}",
            Blue.paint("Execution completed successfully on all servers.")
        );
    } else if any_success {
        println!(
            "{}",
            Yellow.paint("Execution completed with errors on some servers.")
        );
    } else {
        println!("{}", Red.paint("Execution failed on all servers."));
    }

    Ok(())
}

fn display_outputs(rx: Receiver<ServerResult>) {
    for result in rx {
        println!("{} - Output: {}", result.server, result.output);
        std::io::stdout().flush().unwrap();

        // Handle keyboard inputs for scrolling here
        // ...

        thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn main() {
    if !io::stdout().is_terminal() {
        eprint!("This application must be run in a terminal.");
        std::process::exit(1);
    }

    println!("{}", Blue.paint("russh - Multi-Host SSH Client"));
    println!("-----------------------------");
    println!("{}", Green.paint("Author: Eric Tossell"));
    println!(
        "{}",
        Red.paint("GitHub: https://github.com/erictossell/russh")
    );

    let cli = parse_cli_args();
    if let Err(e) = run_application(cli) {
        eprintln!("Application error: {}", e);
        std::process::exit(1); // Use an appropriate exit code
    }
}
