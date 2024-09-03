use anyhow::Result;
use log::{Level, LevelFilter, Metadata, Record};
use std::cell::RefCell;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Once;
use tempfile::TempDir;

use git_test::commands::add::cmd_add;
use git_test::git;

static INIT: Once = Once::new();

thread_local! {
    static LOG_CONTENTS: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

struct TestLogger;

impl log::Log for TestLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let log_entry = format!("{} - {}", record.level(), record.args());
            LOG_CONTENTS.with(|contents| {
                contents.borrow_mut().push(log_entry);
            });
        }
    }

    fn flush(&self) {}
}

fn setup_logger() {
    INIT.call_once(|| {
        log::set_boxed_logger(Box::new(TestLogger))
            .map(|()| log::set_max_level(LevelFilter::Info))
            .unwrap();
    });
}

fn clear_log_contents() {
    LOG_CONTENTS.with(|contents| {
        contents.borrow_mut().clear();
    });
}

fn get_log_contents() -> Vec<String> {
    LOG_CONTENTS.with(|contents| contents.borrow().clone())
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
    setup_logger();
    clear_log_contents();
    let (_temp_dir, repo_path) = setup_test();

    cmd_add(&repo_path, "default", false, false, "just default")?;

    let command = get_test_command(&repo_path, "default")?;
    assert_eq!(command, "just default");
    assert!(get_log_contents()
        .iter()
        .any(|log| log.contains("INFO - Creating new test 'default'")));
    Ok(())
}

#[test]
fn test_add_multiple_tests() -> Result<()> {
    setup_logger();
    clear_log_contents();
    let (_temp_dir, repo_path) = setup_test();

    cmd_add(&repo_path, "default", false, false, "just default")?;
    cmd_add(
        &repo_path,
        "spotless-formats",
        false,
        false,
        "just spotless formats",
    )?;
    cmd_add(
        &repo_path,
        "spotless-java-sort-imports",
        false,
        false,
        "just spotless java-sort-imports",
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
fn test_add_existing_test_no_flags() -> Result<()> {
    setup_logger();
    clear_log_contents();
    let (_temp_dir, repo_path) = setup_test();

    cmd_add(&repo_path, "default", false, false, "old command")?;
    cmd_add(&repo_path, "default", false, false, "new command")?;

    let logs = get_log_contents();
    assert!(logs
        .iter()
        .any(|log| log.contains("WARN - Overwriting existing test 'default'")));
    assert!(logs
        .iter()
        .any(|log| log.contains("INFO - Existing command for test 'default': old command")));
    assert!(logs
        .iter()
        .any(|log| log.contains("INFO - New command for test 'default': new command")));
    assert_eq!(get_test_command(&repo_path, "default")?, "new command");
    Ok(())
}

#[test]
fn test_add_existing_test_with_forget() -> Result<()> {
    setup_logger();
    clear_log_contents();
    let (_temp_dir, repo_path) = setup_test();

    cmd_add(&repo_path, "default", false, false, "old command")?;
    cmd_add(&repo_path, "default", true, false, "new command")?;

    let logs = get_log_contents();
    assert!(!logs
        .iter()
        .any(|log| log.contains("WARN - Overwriting existing test 'default'")));
    assert!(logs
        .iter()
        .any(|log| log.contains("INFO - Deleted stored results for test 'default'")));
    assert_eq!(get_test_command(&repo_path, "default")?, "new command");
    Ok(())
}

#[test]
fn test_add_existing_test_with_keep() -> Result<()> {
    setup_logger();
    clear_log_contents();
    let (_temp_dir, repo_path) = setup_test();

    cmd_add(&repo_path, "default", false, false, "old command")?;
    cmd_add(&repo_path, "default", false, true, "new command")?;

    let logs = get_log_contents();
    assert!(!logs
        .iter()
        .any(|log| log.contains("WARN - Overwriting existing test 'default'")));
    assert!(!logs
        .iter()
        .any(|log| log.contains("INFO - Deleted stored results for test 'default'")));
    assert_eq!(get_test_command(&repo_path, "default")?, "new command");
    Ok(())
}

#[test]
fn test_add_existing_test_with_forget_and_keep() -> Result<()> {
    setup_logger();
    clear_log_contents();
    let (_temp_dir, repo_path) = setup_test();

    cmd_add(&repo_path, "default", false, false, "old command")?;
    cmd_add(&repo_path, "default", true, true, "new command")?;

    let logs = get_log_contents();
    assert!(!logs
        .iter()
        .any(|log| log.contains("WARN - Overwriting existing test 'default'")));
    assert!(logs
        .iter()
        .any(|log| log.contains("INFO - Deleted stored results for test 'default'")));
    assert_eq!(get_test_command(&repo_path, "default")?, "new command");
    Ok(())
}

#[test]
fn test_add_existing_test_with_same_command() -> Result<()> {
    setup_logger();
    clear_log_contents();
    let (_temp_dir, repo_path) = setup_test();

    cmd_add(&repo_path, "default", false, false, "same command")?;
    cmd_add(&repo_path, "default", false, false, "same command")?;

    let logs = get_log_contents();
    assert!(logs
        .iter()
        .any(|log| log.contains("INFO - Existing command for test 'default': same command")));
    assert!(logs
        .iter()
        .any(|log| log.contains("INFO - New command for test 'default': same command")));
    assert_eq!(get_test_command(&repo_path, "default")?, "same command");
    Ok(())
}

#[test]
fn test_add_nonexistent_test() {
    setup_logger();
    clear_log_contents();
    let (_temp_dir, repo_path) = setup_test();

    let result = get_test_command(&repo_path, "nonexistent");
    assert!(result.is_err());
}
