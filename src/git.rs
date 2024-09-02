use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

pub fn get_repo_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(&["rev-parse", "--show-toplevel"])
        .output()
        .context("Failed to execute git rev-parse --show-toplevel")?;

    if output.status.success() {
        Ok(PathBuf::from(String::from_utf8(output.stdout)?.trim()))
    } else {
        Err(anyhow::anyhow!("Not in a git repository"))
    }
}

pub fn get_test_command(repo_root: &PathBuf, test: &str) -> Result<Option<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(&["config", "--get", &format!("test.{}.command", test)])
        .output()
        .context("Failed to execute git config --get")?;

    if output.status.success() {
        Ok(Some(String::from_utf8(output.stdout)?.trim().to_string()))
    } else {
        Ok(None)
    }
}

pub fn set_test_command(repo_root: &PathBuf, test: &str, command: &str) -> Result<()> {
    Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(&["config", &format!("test.{}.command", test), command])
        .status()
        .context("Failed to execute git config")?;

    Ok(())
}
