use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Instant;

#[derive(Serialize, Deserialize)]
pub struct ServerResult {
    server: String,
    output: String,
    error: Option<String>,
    duration: f64,
}

pub fn run_ssh_command(server: &str, user: &str, command: &str, ssh_options: &str) -> ServerResult {
    let start = Instant::now();
    let output = Command::new("ssh")
        .args(&[ssh_options, &format!("{}@{}", user, server), command])
        .output();

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
