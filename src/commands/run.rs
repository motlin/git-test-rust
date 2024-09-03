use std::path::PathBuf;

pub fn cmd_run(
    repo_root: &PathBuf,
    test: &str,
    force: bool,
    forget: bool,
    retest: bool,
    keep_going: bool,
    dry_run: bool,
    stdin: bool,
    commits: &[String],
) -> anyhow::Result<()> {
    // Implement run command
    println!("Running test '{}' on commits: {:?}", test, commits);
    Ok(())
}
