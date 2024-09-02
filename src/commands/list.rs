use std::io::Write;
use std::path::PathBuf;

pub fn cmd_list<W: Write>(
    repo_root: &PathBuf,
    verbosity: i8,
    writer: &mut W,
) -> anyhow::Result<()> {
    // Implement list command
    println!("Listing defined tests");
    Ok(())
}
