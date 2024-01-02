mod config;
mod ssh;
use crate::config::{
    find_config_in_cwd, find_config_in_user_dir, prompt_create_default_config, read_config,
};
use crate::ssh::run_ssh_command;

use ansi_term::Color::{Blue, Green, Red, Yellow};
use argh::FromArgs;

use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::io::{self, BufWriter, IsTerminal, Write}; // Use std::io::Write and others
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use thiserror::Error;

#[derive(Error, Debug)]
enum AppError {
    #[error("configuration error: {0}")]
    ConfigError(#[from] serde_json::Error),
    #[error("file error: {0}")]
    FileError(#[from] std::io::Error),
    #[error("generic error: {0}")]
    GenericError(String),
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

    let config_path = if let Some(config_path) = cli.config_file {
        let path = PathBuf::from(&config_path);
        if path.exists() {
            path
        } else {
            eprintln!("Specified configuration file not found: {}", config_path);
            return Err(AppError::GenericError(
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
                return Err(AppError::GenericError(
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

    let results = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();
    // Create a progress bar for tracking overall progress
    let overall_progress = ProgressBar::new(commands.len() as u64 * config.servers.len() as u64);

    // Configure the progress bar style
    overall_progress.set_style(
        ProgressStyle::default_bar()
            .template("[{bar:40}] {percent}% ({pos}/{len})")
            .expect("Failed to set progress bar style"),
    );

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
            // Create a progress bar for tracking individual command progress
            let progress = ProgressBar::new_spinner();
            let server_label = format!("Server: {}", server);
            let command_label = format!("Command: {}", command);
            let message = format!("{} - {}", server_label, command_label);
            progress.set_message(message);

            let command_arc = Arc::new(command.clone());
            let results_arc = Arc::clone(&results);

            let server_ref = Arc::clone(&server_arc);
            let ssh_options_ref = Arc::clone(&ssh_options_arc);
            let user_ref = Arc::clone(&user_arc);
            let command_ref = Arc::clone(&command_arc);
            let progress_ref = progress.clone();

            let handle = thread::spawn(move || {
                let result =
                    run_ssh_command(&server_ref, &user_ref, &command_ref, &ssh_options_ref);
                let mut results = results_arc.lock().unwrap();
                results.push(result);

                // Finish the progress bar
                progress_ref.finish_and_clear();
            });

            handles.push(handle);
            overall_progress.inc(1);
        }
    }

    for handle in handles {
        handle.join().unwrap();
    }

    overall_progress.finish();

    let mut results = results.lock().unwrap();
    results.sort_by(|a, b| a.server.cmp(&b.server));

    let log_file = File::create("output.log").expect("Unable to create log file");
    let mut log_writer = BufWriter::new(log_file);

    for result in results.iter() {
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

        println!("{}", Blue.paint("Execution completed on all servers."));
    }
    Ok(())
}

fn main() {
    if !io::stdout().is_terminal() {
        writeln!(io::stderr(), "This application must be run in a terminal.").unwrap();
        std::process::exit(1);
    }

    let cli = parse_cli_args();

    if let Err(e) = run_application(cli) {
        eprintln!("Application error: {}", e);
        std::process::exit(1); // Use an appropriate exit code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_application() {
        let cli = Cli {
            commands: vec!["command1".to_string(), "command2".to_string()],
            config_file: Some("russh.json".to_string()),
        };

        assert!(run_application(cli).is_ok());
    }
}
