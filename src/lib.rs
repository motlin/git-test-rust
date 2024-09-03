use anyhow::{Context, Result};
use clap::Parser;
use log::{debug, LevelFilter};
use simple_logger::SimpleLogger;
use std::process::{Command, Output};

fn log_and_run_command(command: &mut Command) -> Result<Output> {
    debug!("Executing command: {:?}", command);
    command.output().context("Failed to execute command")
}

pub mod git {
    use crate::log_and_run_command;
    use anyhow::{Context, Result};
    use regex::Regex;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    pub struct GitRepository {
        root: PathBuf,
    }

    pub struct GitTestCommand<'a> {
        repo: &'a GitRepository,
        key: String,
        value: String,
    }

    impl GitRepository {
        pub fn new(root: PathBuf) -> Self {
            GitRepository { root }
        }

        pub fn get_test_command(&self, test: &str) -> Result<GitTestCommand> {
            let key = format!("test.{}.command", test);
            let value = self.get_config_value(&key)?;
            Ok(GitTestCommand {
                repo: self,
                key,
                value,
            })
        }

        fn get_config_value(&self, key: &str) -> Result<String> {
            let output = log_and_run_command(
                Command::new("git")
                    .arg("-C")
                    .arg(&self.root)
                    .args(&["config", "--get", key]),
            )
            .context("Failed to execute git config --get")?;

            if output.status.success() {
                Ok(String::from_utf8(output.stdout)?.trim().to_string())
            } else {
                Err(anyhow::anyhow!("Config value not found for key: {}", key))
            }
        }

        pub fn set_config_value(&self, key: &str, value: &str) -> Result<()> {
            log_and_run_command(
                Command::new("git")
                    .arg("-C")
                    .arg(&self.root)
                    .args(&["config", key, value]),
            )
            .with_context(|| format!("Failed to set git config value for key '{}'", key))?;
            Ok(())
        }

        pub fn set_test_command(&self, test: &str, command: &str) -> Result<()> {
            self.set_config_value(&format!("test.{}.command", test), command)
        }

        pub fn list_tests(&self) -> Result<Vec<(String, String)>> {
            let output =
                log_and_run_command(Command::new("git").arg("-C").arg(&self.root).args(&[
                    "config",
                    "--get-regexp",
                    "--null",
                    r"^test\..*\.command$",
                ]))
                .context("Failed to execute git config --get-regexp")?;

            if output.status.success() {
                let config_output = String::from_utf8(output.stdout)?;
                let test_config_re = Regex::new(r"^test\.(?P<name>.*)\.command$").unwrap();

                let vec = config_output
                    .split('\0')
                    .filter_map(|entry| {
                        let mut parts = entry.splitn(2, '\n');
                        match (parts.next(), parts.next()) {
                            (Some(key), Some(value)) => {
                                test_config_re.captures(key).map(|captures| {
                                    let name = captures.name("name").unwrap().as_str().to_string();
                                    (name, value.to_string())
                                })
                            }
                            _ => None,
                        }
                    })
                    .collect();
                Ok(vec)
            } else {
                Ok(Vec::new()) // Return an empty vector if no tests are found
            }
        }
    }

    impl<'a> GitTestCommand<'a> {
        pub fn key(&self) -> &str {
            &self.key
        }

        pub fn value(&self) -> &str {
            &self.value
        }
    }

    pub fn get_repo_root(dir: &Path) -> Result<GitRepository> {
        let output = log_and_run_command(
            Command::new("git")
                .arg("-C")
                .arg(dir)
                .args(&["rev-parse", "--show-toplevel"]),
        )
        .context("Failed to execute git rev-parse --show-toplevel")?;

        if output.status.success() {
            let root = PathBuf::from(String::from_utf8(output.stdout)?.trim());
            Ok(GitRepository::new(root))
        } else {
            Err(anyhow::anyhow!("Not in a git repository"))
        }
    }
}

mod cli {
    use clap::{Args, Parser, Subcommand};

    #[derive(Parser)]
    #[command(
        name = "git-test",
        author,
        version,
        about,
        long_about = "Run tests within a Git project and remember the test results.\n\
    `git test` consists of a few things:\n\
    * A way of defining tests for a Git project. The commands to be run\n\
      for a particular test are stored in the repository's Git\n\
      configuration.\n\
    * Tools for running such tests against single Git commits or against\n\
      ranges of commits.\n\
    * A scheme for storing the results of such tests as git notes. The\n\
      results are connected to the tree of the commit that was tested, so\n\
      the test results remain valid across some types of merges, rebases,\n\
      etc.\n\
    * The intelligence not to re-run a test whose results are already\n\
      known.\n\
    \n\
    Example: make sure that all commits on a feature branch pass the tests\n\
    implied by `make -j16 test` (run the tests in a worktree to avoid\n\
    tying up your main repository):\n\
        $ git config test.full.command 'make -j16 test'\n\
        $ git worktree add --detach ../tests feature\n\
        $ cd ../tests\n\
        $ git test run --test=full master..feature\n\
    Any time you make changes to the feature branch in your main\n\
    repository, you can re-run the last command in the `tests` worktree.\n\
    It will only test commits with trees that it hasn't been seen before."
    )]
    pub struct Cli {
        #[command(subcommand)]
        pub command: Commands,

        #[arg(short, long, global = true, action = clap::ArgAction::Count, help = "generate more verbose output (may be specified multiple times)")]
        pub verbose: u8,

        #[arg(short, long, global = true, action = clap::ArgAction::Count, help = "generate less verbose output (may be specified multiple times)")]
        pub quiet: u8,

        #[arg(long, global = true, help = "print status with colors")]
        pub color: bool,

        #[arg(long, global = true, help = "print status without colors")]
        pub no_color: bool,
    }

    #[derive(Subcommand)]
    pub enum Commands {
        #[command(about = "define a new test")]
        Add(AddArgs),

        #[command(about = "run a test against one or more commits")]
        Run(RunArgs),

        #[command(about = "obsolete command; please use \"git test run\" instead")]
        Range(RunArgs),

        #[command(about = "show any stored test results for the specified commits")]
        Results(ResultsArgs),

        #[command(about = "permanently forget stored results for a test")]
        ForgetResults(ForgetResultsArgs),

        #[command(about = "list the tests that are currently defined")]
        List,

        #[command(about = "remove a test definition and all of its stored results")]
        Remove(RemoveArgs),
    }

    #[derive(Args)]
    pub struct AddArgs {
        #[arg(
            short,
            long,
            default_value = "default",
            help = "name of test to add (default is 'default')"
        )]
        pub test: String,

        #[arg(long, help = "forget any existing results", conflicts_with = "keep")]
        pub forget: bool,

        #[arg(
            long,
            help = "keep any existing results (default)",
            conflicts_with = "forget",
            default_value_t = true
        )]
        pub keep: bool,

        #[arg(help = "command to run")]
        pub command: String,
    }

    #[derive(Args)]
    pub struct RunArgs {
        #[arg(
            short,
            long,
            default_value = "default",
            help = "name of test (default is 'default')"
        )]
        pub test: String,

        #[arg(
            short,
            long,
            help = "forget any existing test results for the specified commits and test them again"
        )]
        pub force: bool,

        #[arg(
            long,
            help = "forget any existing test results for the specified commits"
        )]
        pub forget: bool,

        #[arg(
            long,
            help = "if a commit is already marked as \"bad\", try testing it again"
        )]
        pub retest: bool,

        #[arg(
            short,
            long,
            help = "if a commit fails the test, continue testing other commits rather than aborting"
        )]
        pub keep_going: bool,

        #[arg(
            short = 'n',
            long,
            help = "show known results, without running any new tests"
        )]
        pub dry_run: bool,

        #[arg(
            long,
            help = "read the list of commits to test from standard input, one per line"
        )]
        pub stdin: bool,

        #[arg(help = "commits or ranges of commits to test")]
        pub commits: Vec<String>,
    }

    #[derive(Args)]
    pub struct ResultsArgs {
        #[arg(
            short,
            long,
            default_value = "default",
            help = "name of test (default is 'default')"
        )]
        pub test: String,

        #[arg(
            long,
            help = "read the list of commits from standard input, one per line"
        )]
        pub stdin: bool,

        #[arg(help = "commits or ranges of commits")]
        pub commits: Vec<String>,
    }

    #[derive(Args)]
    pub struct ForgetResultsArgs {
        #[arg(
            short,
            long,
            default_value = "default",
            help = "name of test whose results should be forgotten (default is 'default')"
        )]
        pub test: String,
    }

    #[derive(Args)]
    pub struct RemoveArgs {
        #[arg(
            short,
            long,
            default_value = "default",
            help = "name of test to remove (default is 'default')"
        )]
        pub test: String,
    }
}

pub mod commands {
    use crate::git::GitRepository;
    use anyhow::{Context, Result};
    use log::{info, warn};

    pub mod add {
        use super::*;
        use crate::commands::forget_results::forget_results;
        use crate::git;
        use anyhow::{Context, Result};
        use log::{info, warn};
        use std::path::PathBuf;

        pub fn cmd_add(
            repo: &GitRepository,
            test: &str,
            forget: bool,
            keep: bool,
            command: &str,
        ) -> Result<()> {
            // Check if the test already exists
            let existing_command = repo.get_test_command(test);
            let had_existing_command = existing_command.is_ok();

            let old_command = existing_command
                .map(|cmd| cmd.value().to_string())
                .unwrap_or_else(|_| "<empty>".to_string());

            if !forget && !keep && had_existing_command {
                warn!(
                    "Overwriting existing test '{}'. Use --forget to delete stored results or --keep to preserve them.",
                    test
                );
            }

            if forget {
                forget_results(repo, test)
                    .with_context(|| format!("Failed to delete stored results for '{}'", test))?;
            }

            // Set the new test command
            repo.set_test_command(test, command)
                .with_context(|| format!("Failed to set test command for '{}'", test))?;

            info!(
                "Changing test '{}' from '{}' to '{}'",
                test, old_command, command
            );

            Ok(())
        }
    }

    pub mod forget_results {
        use super::*;

        pub fn cmd_forget_results(repo: &GitRepository, test: &str) -> Result<()> {
            // Implement forget-results command
            println!("Forgetting results for test '{}'", test);
            Ok(())
        }

        pub(crate) fn forget_results(repo: &GitRepository, test: &str) -> Result<()> {
            // This is a placeholder for the forget-results logic
            // Implement the actual forget-results functionality here
            println!("Forgetting results for test '{}'", test);
            Ok(())
        }
    }

    pub mod list {
        use super::*;
        use colored::*;
        use std::collections::HashMap;

        pub fn cmd_list(repo: &GitRepository) -> Result<()> {
            let tests = repo.list_tests()?;

            if tests.is_empty() {
                warn!("No tests defined.");
            } else {
                for (test_name, command) in tests {
                    info!("{}:", test_name.bold());
                    info!("    command = {}", command.green());
                }
            }

            Ok(())
        }
    }

    pub mod remove {
        use super::*;

        pub fn cmd_remove(repo: &GitRepository, test: &str) -> Result<()> {
            // Implement remove command
            println!("Removing test '{}'", test);
            Ok(())
        }
    }

    pub mod results {
        use super::*;

        pub fn cmd_results(
            repo: &GitRepository,
            test: &str,
            stdin: bool,
            commits: &[String],
        ) -> Result<()> {
            // Implement results command
            println!(
                "Showing results for test '{}' on commits: {:?}",
                test, commits
            );
            Ok(())
        }
    }

    pub mod run {
        use super::*;

        pub fn cmd_run(
            repo: &GitRepository,
            test: &str,
            force: bool,
            forget: bool,
            retest: bool,
            keep_going: bool,
            dry_run: bool,
            stdin: bool,
            commits: &[String],
        ) -> Result<()> {
            // Implement run command
            println!("Running test '{}' on commits: {:?}", test, commits);
            Ok(())
        }
    }

    pub use add::cmd_add;
    pub use forget_results::cmd_forget_results;
    pub use list::cmd_list;
    pub use remove::cmd_remove;
    pub use results::cmd_results;
    pub use run::cmd_run;
}

use crate::git::get_repo_root;
use cli::{Cli, Commands};

pub fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up colored output
    colored::control::set_override(cli.color && !cli.no_color);

    // Calculate verbosity and set up logger
    let verbosity = cli.verbose as i8 - cli.quiet as i8;
    let log_level = match verbosity {
        i8::MIN..=-2 => LevelFilter::Error,
        -1 => LevelFilter::Warn,
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        2..=i8::MAX => LevelFilter::Trace,
    };
    SimpleLogger::new().with_level(log_level).init()?;

    // Get the repository root
    let current_dir = std::env::current_dir()?;
    let repo = get_repo_root(&current_dir)?;

    match &cli.command {
        Commands::Add(args) => {
            commands::cmd_add(&repo, &args.test, args.forget, args.keep, &args.command)
        }
        Commands::List => commands::cmd_list(&repo),
        _ => unimplemented!("Other commands need to be updated"),
    }
}
