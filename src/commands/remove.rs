use std::path::PathBuf;

pub fn cmd_remove(repo_root: &PathBuf, test: &str) -> anyhow::Result<()> {
    // Implement remove command
    println!("Removing test '{}'", test);
    Ok(())
}
