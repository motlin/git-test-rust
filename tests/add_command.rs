use anyhow::Result;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

use git_test::commands::add::cmd_add;
use git_test::git;

struct TestWriter {
    buffer: Vec<u8>,
}

impl TestWriter {
    fn new() -> Self {
        TestWriter { buffer: Vec::new() }
    }

    fn contents(&self) -> String {
        String::from_utf8_lossy(&self.buffer).to_string()
    }
}

impl Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn init_git_repo(temp_dir: &PathBuf) {
    Command::new("git")
        .args(&["init"])
        .current_dir(temp_dir)
        .status()
        .unwrap();
}

fn get_test_command(repo_path: &PathBuf, test_name: &str) -> Result<String> {
    git::get_test_command(repo_path, test_name)
}

fn setup_test() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().to_path_buf();
    init_git_repo(&repo_path);
    (temp_dir, repo_path)
}

#[test]
fn test_add_new_test() -> Result<()> {
    let (_temp_dir, repo_path) = setup_test();

    let mut writer = TestWriter::new();
    cmd_add(
        &repo_path,
        "default",
        false,
        false,
        "just default",
        0,
        &mut writer,
    )?;

    let command = get_test_command(&repo_path, "default")?;
    assert_eq!(command, "just default");
    Ok(())
}

#[test]
fn test_add_multiple_tests() -> Result<()> {
    let (_temp_dir, repo_path) = setup_test();

    let mut writer = TestWriter::new();
    cmd_add(
        &repo_path,
        "default",
        false,
        false,
        "just default",
        0,
        &mut writer,
    )?;
    cmd_add(
        &repo_path,
        "spotless-formats",
        false,
        false,
        "just spotless formats",
        0,
        &mut writer,
    )?;
    cmd_add(
        &repo_path,
        "spotless-java-sort-imports",
        false,
        false,
        "just spotless java-sort-imports",
        0,
        &mut writer,
    )?;

    assert_eq!(get_test_command(&repo_path, "default")?, "just default");
    assert_eq!(
        get_test_command(&repo_path, "spotless-formats")?,
        "just spotless formats"
    );
    assert_eq!(
        get_test_command(&repo_path, "spotless-java-sort-imports")?,
        "just spotless java-sort-imports"
    );
    Ok(())
}

#[test]
fn test_add_existing_test() -> Result<()> {
    let (_temp_dir, repo_path) = setup_test();

    let mut writer = TestWriter::new();
    cmd_add(
        &repo_path,
        "default",
        false,
        false,
        "just default",
        0,
        &mut writer,
    )?;
    cmd_add(
        &repo_path,
        "default",
        false,
        false,
        "new command",
        0,
        &mut writer,
    )?;

    let output = writer.contents();
    assert!(
        output.contains("WARNING: there are already results stored for the test named 'default'")
    );

    assert_eq!(get_test_command(&repo_path, "default")?, "new command");
    Ok(())
}

#[test]
fn test_add_existing_test_with_forget() -> Result<()> {
    let (_temp_dir, repo_path) = setup_test();

    let mut writer = TestWriter::new();
    cmd_add(
        &repo_path,
        "default",
        false,
        false,
        "just default",
        0,
        &mut writer,
    )?;
    cmd_add(
        &repo_path,
        "default",
        true,
        false,
        "new command",
        0,
        &mut writer,
    )?;

    assert_eq!(get_test_command(&repo_path, "default")?, "new command");
    Ok(())
}
#[test]
fn test_add_existing_test_with_same_command() -> Result<()> {
    let (_temp_dir, repo_path) = setup_test();

    let mut writer = TestWriter::new();
    cmd_add(
        &repo_path,
        "default",
        false,
        false,
        "just default",
        1,
        &mut writer,
    )?;
    cmd_add(
        &repo_path,
        "default",
        false,
        false,
        "just default",
        1,
        &mut writer,
    )?;

    let output = writer.contents();
    assert!(
        output.contains("Test 'default' already exists with the same command. No changes made.")
    );

    assert_eq!(get_test_command(&repo_path, "default")?, "just default");
    Ok(())
}

#[test]
fn test_add_nonexistent_test() {
    let (_temp_dir, repo_path) = setup_test();

    let result = get_test_command(&repo_path, "nonexistent");
    assert!(result.is_err());
}
