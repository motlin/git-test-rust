use anyhow::Result;
use clap::Parser;
use log::LevelFilter;
use simple_logger::SimpleLogger;

pub mod git {
    use anyhow::{Context, Result};
    use std::path::PathBuf;
    use std::process::Command;

    pub fn get_repo_root() -> Result<PathBuf> {
        let output = Command::new("git")
            .args(&["rev-parse", "--show-toplevel"])
            .output()
            .context("Failed to execute git rev-parse --show-toplevel")?;

        if output.status.success() {
            Ok(PathBuf::from(String::from_utf8(output.stdout)?.trim()))
        } else {
            Err(anyhow::anyhow!("Not in a git repository"))
        }
    }

    pub fn get_config_value(repo_root: &PathBuf, key: &str) -> Result<String> {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo_root)
            .args(&["config", "--get", key])
            .output()
            .context("Failed to execute git config --get")?;

        if output.status.success() {
            Ok(String::from_utf8(output.stdout)?.trim().to_string())
        } else {
            Err(anyhow::anyhow!("Config value not found for key: {}", key))
        }
    }

    pub fn get_test_command(repo_root: &PathBuf, test: &str) -> Result<String> {
        get_config_value(repo_root, &format!("test.{}.command", test))
    }

    pub fn set_test_command(repo_root: &PathBuf, test: &str, command: &str) -> Result<()> {
        Command::new("git")
            .arg("-C")
            .arg(repo_root)
            .args(&["config", &format!("test.{}.command", test), command])
            .status()
            .context("Failed to execute git config")?;

        Ok(())
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
    pub mod add {
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
            let had_existing_command = existing_command.is_ok();

            let old_command = existing_command.unwrap_or_else(|_| "<empty>".to_string());

            if !forget && !keep && had_existing_command {
                warn!(
                    "Overwriting existing test '{}'. Use --forget to delete stored results or --keep to preserve them.",
                    test
                );
            }

            if forget {
                forget_results(repo_root, test)
                    .with_context(|| format!("Failed to delete stored results for '{}'", test))?;
            }

            // Set the new test command
            git::set_test_command(repo_root, test, command)
                .with_context(|| format!("Failed to set test command for '{}'", test))?;

            info!(
                "Changing test '{}' from '{}' to '{}'",
                test, old_command, command
            );

            Ok(())
        }
    }
    pub mod forget_results {
        use std::path::PathBuf;

        pub fn cmd_forget_results(repo_root: &PathBuf, test: &str) -> anyhow::Result<()> {
            // Implement forget-results command
            println!("Forgetting results for test '{}'", test);
            Ok(())
        }

        pub(crate) fn forget_results(repo_root: &PathBuf, test: &str) -> anyhow::Result<()> {
            // This is a placeholder for the forget-results logic
            // Implement the actual forget-results functionality here
            println!("Forgetting results for test '{}'", test);
            Ok(())
        }
    }
    pub mod list {
        use std::path::PathBuf;

        pub fn cmd_list(repo_root: &PathBuf) -> anyhow::Result<()> {
            // Implement list command
            println!("Listing defined tests");
            Ok(())
        }
    }
    pub mod remove {
        use std::path::PathBuf;

        pub fn cmd_remove(repo_root: &PathBuf, test: &str) -> anyhow::Result<()> {
            // Implement remove command
            println!("Removing test '{}'", test);
            Ok(())
        }
    }
    pub mod results {
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
    }
    pub mod run {
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
    }

    pub use add::cmd_add;
    pub use forget_results::cmd_forget_results;
    pub use list::cmd_list;
    pub use remove::cmd_remove;
    pub use results::cmd_results;
    pub use run::cmd_run;
}

use cli::{Cli, Commands};
use commands::*;

pub fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up colored output
    colored::control::set_override(cli.color && !cli.no_color);

    // Calculate verbosity and set up logger
    let verbosity = cli.verbose as i8 - cli.quiet as i8;
    let log_level = match verbosity {
        i8::MIN..=-1 => LevelFilter::Error,
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        3..=i8::MAX => LevelFilter::Trace,
    };
    SimpleLogger::new().with_level(log_level).init()?;

    // Get the repository root
    let repo_root = git::get_repo_root()?;

    match &cli.command {
        Commands::Add(args) => cmd_add(
            &repo_root,
            &args.test,
            args.forget,
            args.keep,
            &args.command,
        ),
        Commands::Run(args) | Commands::Range(args) => cmd_run(
            &repo_root,
            &args.test,
            args.force,
            args.forget,
            args.retest,
            args.keep_going,
            args.dry_run,
            args.stdin,
            &args.commits,
        ),
        Commands::Results(args) => cmd_results(&repo_root, &args.test, args.stdin, &args.commits),
        Commands::ForgetResults(args) => cmd_forget_results(&repo_root, &args.test),
        Commands::List => cmd_list(&repo_root),
        Commands::Remove(args) => cmd_remove(&repo_root, &args.test),
    }
}
