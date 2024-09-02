use crate::commands::forget_results::forget_results;
use crate::git;
use anyhow::{Context, Result};
use std::io::Write;
use std::path::PathBuf;

pub fn cmd_add<W: Write>(
    repo_root: &PathBuf,
    test: &str,
    forget: bool,
    keep: bool,
    command: &str,
    verbosity: i8,
    writer: &mut W,
) -> Result<()> {
    // Check if the test already exists
    let existing_command = git::get_test_command(repo_root, test);

    match existing_command {
        Ok(existing_cmd) => {
            if existing_cmd == command {
                writeln!(
                    writer,
                    "Test '{}' already exists with the same command. No changes made.",
                    test
                )?;
                return Ok(());
            }

            if !forget {
                writeln!(
                    writer,
                    "WARNING: there are already results stored for the test named '{}'",
                    test
                )?;
                writeln!(
                    writer,
                    "Use --forget to overwrite the existing test and delete all stored results"
                )?;
            }

            if !keep {
                // TODO: Implement logic to delete stored results
                writeln!(writer, "Deleting stored results for test '{}'", test)?;
                forget_results(repo_root, test)
                    .with_context(|| format!("Failed to delete stored results for '{}'", test))?;
            }
        }
        Err(_) => {
            if verbosity > 0 {
                writeln!(writer, "Creating new test '{}'", test)?;
            }
        }
    }

    // Set the new test command
    git::set_test_command(repo_root, test, command)
        .with_context(|| format!("Failed to set test command for '{}'", test))?;

    if verbosity > 0 {
        writeln!(writer, "Added test '{}' with command: {}", test, command)?;
    }

    Ok(())
}
