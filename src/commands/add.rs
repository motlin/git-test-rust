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

    match existing_command {
        Ok(existing_cmd) => {
            info!("Existing command for test '{}': {}", test, existing_cmd);
            info!("New command for test '{}': {}", test, command);

            if !forget && !keep {
                warn!(
                    "Overwriting existing test '{}'. Use --forget to delete stored results or --keep to preserve them.",
                    test
                );
            }

            if forget {
                forget_results(repo_root, test)
                    .with_context(|| format!("Failed to delete stored results for '{}'", test))?;
                info!("Deleted stored results for test '{}'", test);
            }
        }
        Err(_) => {
            info!("Creating new test '{}'", test);
        }
    }

    // Set the new test command
    git::set_test_command(repo_root, test, command)
        .with_context(|| format!("Failed to set test command for '{}'", test))?;

    info!("Set test '{}' with command: {}", test, command);

    Ok(())
}
