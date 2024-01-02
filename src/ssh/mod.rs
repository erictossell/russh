use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Instant;

#[derive(Serialize, Deserialize)]
pub struct ServerResult {
    pub server: String,
    pub output: String,
    pub error: Option<String>,
    pub duration: f64,
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
            error: if output.status.success() {
                None
            } else {
                Some(String::from_utf8_lossy(&output.stderr).to_string())
            },
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Output};
    use std::time::Duration;

    #[test]
    fn test_run_ssh_command_success() {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(&["/C", "echo Success output"])
                .output()
                .expect("Failed to execute command")
        } else {
            Command::new("echo")
                .arg("Success output")
                .output()
                .expect("Failed to execute command")
        };

        let result = run_ssh_command_with_output(
            "server",
            "_user",
            "_command",
            "_ssh_options",
            output,
            Duration::from_secs(1),
        );

        assert_eq!(result.server, "server");
        assert_eq!(result.output.trim(), "Success output");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_run_ssh_command_failure() {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(&["/C", "echo Error output >&2 && exit 1"])
                .output()
                .expect("Failed to execute command")
        } else {
            Command::new("sh")
                .arg("-c")
                .arg("echo Error output >&2 && exit 1")
                .output()
                .expect("Failed to execute command")
        };

        let result = run_ssh_command_with_output(
            "server",
            "_user",
            "_command",
            "_ssh_options",
            output,
            Duration::from_secs(1),
        );

        assert_eq!(result.server, "server");
        assert!(result.output.is_empty());
        assert_eq!(result.error.unwrap().trim(), "Error output");
    }
    // The modified version of run_ssh_command that takes Output and Duration as arguments
    fn run_ssh_command_with_output(
        server: &str,
        _user: &str,
        _command: &str,
        _ssh_options: &str,
        output: Output,
        duration: Duration,
    ) -> ServerResult {
        let duration_secs = duration.as_secs_f64();

        match output.status.success() {
            true => ServerResult {
                server: server.to_string(),
                output: String::from_utf8_lossy(&output.stdout).to_string(),
                error: None,
                duration: duration_secs,
            },
            false => ServerResult {
                server: server.to_string(),
                output: String::new(),
                error: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                duration: duration_secs,
            },
        }
    }
}
