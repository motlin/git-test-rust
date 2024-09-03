use std::path::PathBuf;

pub fn cmd_results(
    repo_root: &PathBuf,
    test: &str,
    stdin: bool,
    commits: &[String],
) -> anyhow::Result<()> {
    // Implement results command
    println!(
        "Showing results for test '{}' on commits: {:?}",
        test, commits
    );
    Ok(())
}
