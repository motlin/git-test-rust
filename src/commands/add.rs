use crate::{commands, git};
use anyhow::Result;
use std::io::Write;
use std::path::PathBuf;

pub fn cmd_add<W: Write>(
    repo_root: &PathBuf,
    test: &str,
    forget: bool,
    _keep: bool,
    command: &str,
    verbosity: i8,
    writer: &mut W,
) -> Result<()> {
    // Check if the test already exists
    let existing_command = git::get_test_command(&repo_root, test)?;

    if let Some(existing_cmd) = existing_command {
        if existing_cmd != command {
            writeln!(writer, "WARNING: there are already results stored for the test named '{}'. Those results will be considered valid for the new test. If that is not what you want, please re-run this command with the '--forget' option.", test)?;
        } else if verbosity > 0 {
            writeln!(
                writer,
                "Test '{}' already exists with the same command. No changes made.",
                test
            )?;
            return Ok(());
        }
    }

    git::set_test_command(&repo_root, test, command)?;

    if verbosity > 0 {
        writeln!(writer, "Added test '{}' with command: {}", test, command)?;
    }

    // If --forget is specified, forget any existing results
    if forget {
        commands::forget_results::forget_results(&repo_root, test)?;
    }

    Ok(())
}
