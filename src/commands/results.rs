use std::io::Write;
use std::path::PathBuf;

pub fn cmd_results<W: Write>(
    repo_root: &PathBuf,
    test: &str,
    stdin: bool,
    commits: &[String],
    verbosity: i8,
    writer: &mut W,
) -> anyhow::Result<()> {
    // Implement results command
    println!(
        "Showing results for test '{}' on commits: {:?}",
        test, commits
    );
    Ok(())
}
