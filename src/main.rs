mod config;
mod ssh;
use crate::config::Config;
use crate::config::{
    find_config_in_cwd, find_config_in_user_dir, prompt_create_default_config, read_config,
};
use crate::ssh::run_ssh_command;

use ansi_term::Color::{Blue, Green, Red};
use argh::FromArgs;

use crate::ssh::ServerResult;

use std::io::{self, IsTerminal, Write}; // Use std::io::Write and others
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

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

// Assuming `prompt_create_default_config` returns a Result<Option<PathBuf>, Error>

fn load_config(config_file: &Option<String>) -> Result<Config> {
    let config_path = match config_file {
        Some(path) => PathBuf::from(path),
        None => find_config_in_cwd()
            .or_else(find_config_in_user_dir)
            .or_else(|| match prompt_create_default_config() {
                Ok(Some(path)) => Some(path),
                Ok(None) => None, // User chose not to create a config
                Err(e) => {
                    eprintln!("Error during configuration creation: {}", e);
                    None
                }
            })
            .ok_or_else(|| AppError::Generic("Configuration file not found".to_string()))?,
    };

    let config_path_str = config_path
        .to_str()
        .ok_or_else(|| AppError::Generic("Invalid configuration file path".to_string()))?;

    read_config(config_path_str).map_err(|e| {
        eprintln!("Failed to read configuration file: {}", e);
        AppError::Generic("Failed to read configuration file".to_string())
    })
}

fn run_application(cli: Cli) -> Result<()> {
    let (tx, rx): (mpsc::Sender<ServerResult>, Receiver<ServerResult>) = mpsc::channel();
    let mut handles = Vec::new();

    // Start a thread for displaying outputs
    thread::spawn(move || {
        display_outputs(rx);
    });

    // Load configuration
    let config = load_config(&cli.config_file)?;

    println!("Processing commands...");
    for server in &config.servers {
        for command in &cli.commands {
            // Clone the values inside the loop before passing them to the thread
            let server_clone = server.clone();
            let user_clone = config.users.get(server).unwrap_or(&String::new()).clone();
            let ssh_options_clone = config
                .ssh_options
                .get(server)
                .unwrap_or(&String::new())
                .clone();
            let command_clone = command.clone();
            let tx_clone = tx.clone();

            let handle = thread::spawn(move || {
                run_ssh_command(
                    &server_clone,
                    &user_clone,
                    &command_clone,
                    &ssh_options_clone,
                    tx_clone,
                );
            });
            handles.push(handle);
        }
    }

    // Wait for all threads to complete
    for handle in handles {
        if let Err(e) = handle.join() {
            eprintln!("Failed to join thread: {:?}", e);
        }
    }

    // Final summary or any other post-processing can be done here
    println!("Execution completed.");

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
