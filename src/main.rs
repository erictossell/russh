mod config;
mod ssh;
use crate::config::{find_config_in_cwd, find_config_in_user_dir, prompt_create_default_config, read_config};
use crate::ssh::run_ssh_command;
use std::fs::File;
use std::io;
use std::io::{BufWriter, IsTerminal, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use thiserror::Error;
use structopt::StructOpt;


#[derive(Error, Debug)]
enum AppError {
    #[error("configuration error: {0}")]
    ConfigError(#[from] serde_json::Error),
    #[error("file error: {0}")]
    FileError(#[from] std::io::Error),
    // Add other error types as needed
}

#[derive(StructOpt, Debug)]
#[structopt(name = "ruSSH")]
struct Cli {
    /// Commands to execute on the servers
    #[structopt(name = "COMMAND", required = true)]
    commands: Vec<String>,
}

type Result<T> = std::result::Result<T, AppError>;

fn main() {

    if !io::stdout().is_terminal() {
        writeln!(io::stderr(), "This application must be run in a terminal.").unwrap();
        std::process::exit(1);
    }

    let cli = Cli::from_args();
    let commands = cli.commands;
    
    let config_path = find_config_in_cwd()
        .or_else(find_config_in_user_dir)
        .or_else(|| {
            match prompt_create_default_config() {
                Ok(Some(path)) => Some(path),
                Ok(None) => {
                    eprintln!("Configuration file not found. Exiting.");
                    None
                },
                Err(e) => {
                    eprintln!("Error creating default configuration: {}", e);
                    None
                }
            }
        });

    if let Some(path) = config_path {
        let config_path_str = path.to_str().unwrap_or_else(|| {
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

        for server in &config.servers {
            let server_arc = Arc::new(server.clone());
            let ssh_options_arc = Arc::new(config.ssh_options.get(server).unwrap_or(&String::new()).clone());
            let user_arc = Arc::new(config.users.get(server).unwrap_or(&String::new()).clone());

            for command in &commands {
                let command_arc = Arc::new(command.clone());
                let results_arc = Arc::clone(&results);

                let server_ref = Arc::clone(&server_arc);
                let ssh_options_ref = Arc::clone(&ssh_options_arc);
                let user_ref = Arc::clone(&user_arc);
                let command_ref = Arc::clone(&command_arc);

                let handle = thread::spawn(move || {
                    let result = run_ssh_command(
                        &server_ref,
                        &user_ref,
                        &command_ref,
                        &ssh_options_ref,
                    );
                    let mut results = results_arc.lock().unwrap();
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
