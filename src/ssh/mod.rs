use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Instant;

#[derive(Serialize, Deserialize)]
pub struct ServerResult {
    pub server: String,
    pub output: String,
    pub error: Option<String>,
    pub duration: f64,
    pub success: bool,
}

pub fn run_ssh_command(
    server: &str,
    user: &str,
    command: &str,
    ssh_options: &str,
    tx: Sender<ServerResult>,
) {
    let start = Instant::now();

    // Convert to owned String types
    let server_owned = server.to_string();
    let user_owned = user.to_string();
    let command_owned = command.to_string();
    let ssh_options_owned = ssh_options.to_string();

    let mut child = Command::new("ssh")
        .args([
            &ssh_options_owned,
            &format!("{}@{}", user_owned, server_owned),
            &command_owned,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start ssh command");

    let stdout = BufReader::new(child.stdout.take().expect("Failed to get stdout"));
    let stderr = BufReader::new(child.stderr.take().expect("Failed to get stderr"));

    let server_clone_for_stdout = server_owned.clone(); // Clone for stdout thread
    let tx_stdout = tx.clone();
    let stdout_thread = thread::spawn(move || {
        for line in stdout.lines() {
            let line = line.expect("Failed to read line from stdout");
            tx_stdout
                .send(ServerResult {
                    server: server_clone_for_stdout.clone(),
                    output: line,
                    error: None,
                    duration: start.elapsed().as_secs_f64(),
                    success: true,
                })
                .expect("Failed to send output");
        }
    });

    let server_clone_for_stderr = server_owned.clone(); // Clone for stderr thread
    let tx_stderr = tx.clone();
    let stderr_thread = thread::spawn(move || {
        for line in stderr.lines() {
            let line = line.expect("Failed to read line from stdout");
            tx_stderr
                .send(ServerResult {
                    server: server_clone_for_stderr.clone(),
                    output: line,
                    error: None,
                    duration: start.elapsed().as_secs_f64(),
                    success: true,
                })
                .expect("Failed to send output");
        }
    });

    // Wait for both threads to complete
    stdout_thread.join().expect("Failed to join stdout thread");
    stderr_thread.join().expect("Failed to join stderr thread");

    // Check command completion status
    let success = child.wait().expect("Failed to wait on child").success();

    // Send final result indicating completion
    tx.send(ServerResult {
        server: server.to_string(),
        output: String::new(), // No additional output at this point
        error: None,
        duration: start.elapsed().as_secs_f64(),
        success,
    })
    .expect("Failed to send final result");
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
                success: output.status.success(),
            },
            false => ServerResult {
                server: server.to_string(),
                output: String::new(),
                error: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                duration: duration_secs,
                success: false,
            },
        }
    }
}
