use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

use git_test::commands::add::cmd_add;

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

fn init_git_repo(temp_dir: &Path) {
    Command::new("git")
        .args(&["init"])
        .current_dir(temp_dir)
        .status()
        .unwrap();
}

fn get_git_config(repo_path: &Path) -> String {
    fs::read_to_string(repo_path.join(".git/config")).unwrap()
}

fn setup_test() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().to_path_buf();
    init_git_repo(&repo_path);
    (temp_dir, repo_path)
}

#[test]
fn test_add_new_test() {
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
    )
    .unwrap();

    let config = get_git_config(&repo_path);
    assert!(config.contains("[test \"default\"]"));
    assert!(config.contains("command = just default"));
}

#[test]
fn test_add_multiple_tests() {
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
    )
    .unwrap();
    cmd_add(
        &repo_path,
        "spotless-formats",
        false,
        false,
        "just spotless formats",
        0,
        &mut writer,
    )
    .unwrap();
    cmd_add(
        &repo_path,
        "spotless-java-sort-imports",
        false,
        false,
        "just spotless java-sort-imports",
        0,
        &mut writer,
    )
    .unwrap();

    let config = get_git_config(&repo_path);
    assert!(config.contains("[test \"default\"]"));
    assert!(config.contains("command = just default"));
    assert!(config.contains("[test \"spotless-formats\"]"));
    assert!(config.contains("command = just spotless formats"));
    assert!(config.contains("[test \"spotless-java-sort-imports\"]"));
    assert!(config.contains("command = just spotless java-sort-imports"));
}

#[test]
fn test_add_existing_test() {
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
    )
    .unwrap();
    cmd_add(
        &repo_path,
        "default",
        false,
        false,
        "new command",
        0,
        &mut writer,
    )
    .unwrap();

    let output = writer.contents();
    assert!(
        output.contains("WARNING: there are already results stored for the test named 'default'")
    );

    let config = get_git_config(&repo_path);
    assert!(config.contains("[test \"default\"]"));
    assert!(config.contains("command = new command"));
    assert!(!config.contains("command = just default"));
}

#[test]
fn test_add_existing_test_with_forget() {
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
    )
    .unwrap();
    cmd_add(
        &repo_path,
        "default",
        true,
        false,
        "new command",
        0,
        &mut writer,
    )
    .unwrap();

    let config = get_git_config(&repo_path);
    assert!(config.contains("[test \"default\"]"));
    assert!(config.contains("command = new command"));
    assert!(!config.contains("command = just default"));
}
#[test]
fn test_add_existing_test_with_same_command() {
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
    )
    .unwrap();
    cmd_add(
        &repo_path,
        "default",
        false,
        false,
        "just default",
        1,
        &mut writer,
    )
    .unwrap();

    let output = writer.contents();
    assert!(
        output.contains("Test 'default' already exists with the same command. No changes made.")
    );

    let config = get_git_config(&repo_path);
    assert!(config.contains("[test \"default\"]"));
    assert!(config.contains("command = just default"));
}
