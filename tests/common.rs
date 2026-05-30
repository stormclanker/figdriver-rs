#![allow(dead_code)]

use std::io::Write;
use std::process::{Command, Stdio};

/// Create a figlet command with the given arguments.
/// Automatically includes `-d fonts` for font directory.
pub fn cmd_figlet(args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_figlet"));
    cmd.arg("-d").arg("fonts");
    cmd.args(args);
    cmd
}

/// Assert that the command succeeded, printing stdout and stderr on failure.
fn assert_success(output: &std::process::Output) {
    if !output.status.success() {
        panic!(
            "Command failed.\nStdout: {}\nStderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// Run a command and return output lines with trailing whitespace removed.
/// Filters out blank lines. Asserts that the command succeeded.
pub fn run(args: &[&str]) -> Vec<String> {
    let mut cmd = cmd_figlet(args);
    let output = cmd.output().unwrap();
    assert_success(&output);
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| line.trim_end().to_string())
        .filter(|line| !line.trim().is_empty())
        .collect()
}

/// Run a command and return output lines without any trimming.
/// Filters out blank lines. Asserts that the command succeeded.
/// Useful for alignment tests where whitespace is significant.
pub fn run_no_trim(args: &[&str]) -> Vec<String> {
    let mut cmd = cmd_figlet(args);
    let output = cmd.output().unwrap();
    assert_success(&output);
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| line.to_string())
        .filter(|line| !line.trim().is_empty())
        .collect()
}

/// Run a command with stdin input and return trimmed, non-empty output lines.
/// Asserts that the command succeeded.
pub fn run_with_input(args: &[&str], input: &str) -> Vec<String> {
    let mut cmd = cmd_figlet(args);
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_success(&output);
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| line.trim_end().to_string())
        .filter(|line| !line.trim().is_empty())
        .collect()
}
