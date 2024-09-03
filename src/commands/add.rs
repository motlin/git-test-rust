use crate::commands::forget_results::forget_results;
use crate::git;
use anyhow::{Context, Result};
use log::{info, warn};
use std::path::PathBuf;

pub fn cmd_add(
    repo_root: &PathBuf,
    test: &str,
    forget: bool,
    keep: bool,
    command: &str,
) -> Result<()> {
    // Check if the test already exists
    let existing_command = git::get_test_command(repo_root, test);
    let had_existing_command = existing_command.is_ok();

    let old_command = existing_command.unwrap_or_else(|_| "<empty>".to_string());

    if !forget && !keep && had_existing_command {
        warn!(
                    "Overwriting existing test '{}'. Use --forget to delete stored results or --keep to preserve them.",
                    test
                );
    }

    if forget {
        forget_results(repo_root, test)
            .with_context(|| format!("Failed to delete stored results for '{}'", test))?;
    }

    // Set the new test command
    git::set_test_command(repo_root, test, command)
        .with_context(|| format!("Failed to set test command for '{}'", test))?;

    info!(
        "Changing test '{}' from '{}' to '{}'",
        test, old_command, command
    );

    Ok(())
}
