mod config;
mod ssh;
use crate::config::{find_config_in_cwd, find_config_in_user_dir, prompt_create_default_config, read_config};
use crate::ssh::run_ssh_command;

use clap::{App, Arg};
use std::fs::File;
use std::io;
use std::io::{BufWriter, IsTerminal, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use thiserror::Error;

#[derive(Error, Debug)]
enum AppError {
    #[error("configuration error: {0}")]
    ConfigError(#[from] serde_json::Error),
    #[error("file error: {0}")]
    FileError(#[from] std::io::Error),
    // Add other error types as needed
}

type Result<T> = std::result::Result<T, AppError>;

fn main() {
    if !io::stdout().is_terminal() {
        writeln!(io::stderr(), "This application must be run in a terminal.").unwrap();
        std::process::exit(1);
    }
    let matches = App::new("ruSSH")
        .version("0.1.0")
        .author("Eric Tossell")
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
