use std::io::Write;
use std::path::PathBuf;

pub fn cmd_remove<W: Write>(
    repo_root: &PathBuf,
    test: &str,
    verbosity: i8,
    writer: &mut W,
) -> anyhow::Result<()> {
    // Implement remove command
    println!("Removing test '{}'", test);
    Ok(())
}
