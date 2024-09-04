use anyhow::Result;
use clap::Parser;

pub mod log_util {
    use anyhow::Context;
    use clap::ColorChoice;
    use colored::Colorize;
    use log::{debug, LevelFilter};
    use simple_logger::SimpleLogger;
    use std::process::Output;
    use tokio::process::Command;

    pub(crate) async fn log_and_run_command(command: &mut Command) -> anyhow::Result<Output> {
        // Get the program and arguments
        let program = command.as_std().get_program().to_str().unwrap_or("");
        let args: Vec<String> = command
            .as_std()
            .get_args()
            .map(|arg| {
                let arg_str = arg.to_str().unwrap_or("");
                if arg_str.contains(' ') {
                    format!("'{}'", arg_str)
                } else {
                    arg_str.to_string()
                }
            })
            .collect();

        // Construct the full command string
        let full_command = format!("{} {}", program, args.join(" "));

        // Log the command
        debug!("{} {}", "❯".green(), full_command);

        // Execute the command
        let output = command
            .output()
            .await
            .context("Failed to execute command")?;

        // Log the output
        if !output.stdout.is_empty() {
            debug!("{}", String::from_utf8_lossy(&output.stdout).dimmed());
        }
        if !output.stderr.is_empty() {
            debug!("{}", String::from_utf8_lossy(&output.stderr).red());
        }

        Ok(output)
    }

    struct CustomLogger;

    impl log::Log for CustomLogger {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            true
        }

        fn log(&self, record: &log::Record) {
            if self.enabled(record.metadata()) {
                println!("{}", record.args());
            }
        }

        fn flush(&self) {}
    }

    pub fn init_logging(verbosity: i8, choice: ColorChoice) -> anyhow::Result<()> {
        let log_level = match verbosity {
            i8::MIN..=-2 => LevelFilter::Error,
            -1 => LevelFilter::Warn,
            0 => LevelFilter::Info,
            1 => LevelFilter::Debug,
            2..=i8::MAX => LevelFilter::Trace,
        };

        if verbosity >= 3 {
            SimpleLogger::new().with_level(log_level).init()?;
        } else {
            log::set_boxed_logger(Box::new(CustomLogger))?;
            log::set_max_level(log_level);
        }

        use colored::control::set_override;

        let colorize = match choice {
            ColorChoice::Auto => atty::is(atty::Stream::Stdout),
            ColorChoice::Always => true,
            ColorChoice::Never => false,
        };
        set_override(colorize);

        Ok(())
    }
}

pub mod git {
    use crate::log_util::log_and_run_command;
    use anyhow::{Context, Result};
    use regex::Regex;
    use std::path::{Path, PathBuf};
    use std::process::Output;
    use tokio::process::Command;

    async fn run_git_with_output(root: &Path, args: &[&str]) -> Result<Output> {
        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(root).args(args);
        log_and_run_command(&mut cmd).await
    }

    pub async fn run_git_with_string(root: &Path, args: &[&str]) -> Result<String> {
        let output = run_git_with_output(root, args).await?;
        if output.status.success() {
            Ok(String::from_utf8(output.stdout)?.trim().to_string())
        } else {
            Err(anyhow::anyhow!("Git command failed: {:?}", args))
        }
    }

    #[derive(Clone)]
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

        pub async fn get_repo_root(dir: &Path) -> Result<Self> {
            let root = run_git_with_string(dir, &["rev-parse", "--show-toplevel"])
                .await
                .context("Failed to execute git rev-parse --show-toplevel")?;

            let root = PathBuf::from(root.trim());
            Ok(GitRepository::new(root))
        }

        pub fn root(&self) -> &Path {
            &self.root
        }

        pub async fn run_git(&self, args: &[&str]) -> Result<String> {
            run_git_with_string(self.root(), args).await
        }

        pub async fn get_config_value(&self, key: &str) -> Result<String> {
            self.run_git(&["config", "--get", key])
                .await
                .context("Failed to get git config value")
        }

        pub async fn set_config_value(&self, key: &str, value: &str) -> Result<()> {
            self.run_git(&["config", key, value])
                .await
                .with_context(|| format!("Failed to set git config value for key '{}'", key))?;
            Ok(())
        }

        pub async fn get_test_command(&self, test: &str) -> Result<GitTestCommand<'_>> {
            let key = format!("test.{}.command", test);
            let command = self.get_config_value(&key).await;

            match command {
                Ok(command) => Ok(GitTestCommand {
                    repo: self,
                    key: test.to_string(),
                    value: command,
                }),
                _ => Err(anyhow::anyhow!("Test '{}' is not defined", test)),
            }
        }

        pub async fn set_test_command(&self, test: &str, command: &str) -> Result<()> {
            self.set_config_value(&format!("test.{}.command", test), command)
                .await
        }

        pub async fn list_tests(&self) -> Result<Vec<(String, String)>> {
            let output = self
                .run_git(&["config", "--get-regexp", "--null", r"^test\..*\.command$"])
                .await?;

            let test_config_re = Regex::new(r"^test\.(?P<name>.*)\.command$").unwrap();

            let tests = output
                .split('\0')
                .filter_map(|entry| {
                    let mut parts = entry.splitn(2, '\n');
                    match (parts.next(), parts.next()) {
                        (Some(key), Some(value)) => test_config_re.captures(key).map(|captures| {
                            let name = captures.name("name").unwrap().as_str().to_string();
                            (name, value.to_string())
                        }),
                        _ => None,
                    }
                })
                .collect();

            Ok(tests)
        }

        pub async fn get_head_commit(&self) -> Result<String> {
            self.run_git(&["rev-parse", "HEAD"])
                .await
                .context("Failed to get HEAD commit")
        }

        pub async fn add_note(&self, ref_name: &str, object: &str, content: &str) -> Result<()> {
            self.run_git(&[
                "notes", "--ref", ref_name, "add", "-f", "-m", content, object,
            ])
            .await?;
            Ok(())
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

    pub async fn get_repo_root(dir: &Path) -> Result<GitRepository> {
        let mut cmd = Command::new("git");
        cmd.arg("-C")
            .arg(dir)
            .args(&["rev-parse", "--show-toplevel"]);

        let output = log_and_run_command(&mut cmd)
            .await
            .context("Failed to execute git rev-parse --show-toplevel")?;

        if output.status.success() {
            let root = PathBuf::from(String::from_utf8(output.stdout)?.trim());
            Ok(GitRepository::new(root))
        } else {
            Err(anyhow::anyhow!("Not in a git repository"))
        }
    }
}

pub mod cli {
    use clap::{Args, ColorChoice, Parser, Subcommand};
    use std::path::PathBuf;

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

        #[arg(
        long,
        global = true,
        help = "Control when to use colors: auto, always, never",
        default_value_t = ColorChoice::Auto
        )]
        pub color: ColorChoice,
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
            help = "name of test (default is 'default')",
            conflicts_with = "all"
        )]
        pub test: Option<String>,

        #[arg(long, help = "run all defined tests", conflicts_with = "test")]
        pub all: bool,

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

        #[arg(
            long,
            help = "run tests in git worktrees",
            default_value = ".worktrees"
        )]
        pub worktree: Option<PathBuf>,

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
    use anyhow::Result;
    use log::{info, warn};

    pub mod add {
        use super::*;
        use crate::commands::forget_results::forget_results;

        use anyhow::{Context, Result};
        use log::{info, warn};

        pub async fn cmd_add(
            repo: &GitRepository,
            test: &str,
            forget: bool,
            keep: bool,
            command: &str,
        ) -> Result<()> {
            let existing_command = repo.get_test_command(test).await;
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
                    .await
                    .with_context(|| format!("Failed to delete stored results for '{}'", test))?;
            }

            repo.set_test_command(test, command)
                .await
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

        pub async fn cmd_forget_results(repo: &GitRepository, test: &str) -> Result<()> {
            // Implement forget-results command
            println!("Forgetting results for test '{}'", test);
            Ok(())
        }

        pub(crate) async fn forget_results(repo: &GitRepository, test: &str) -> Result<()> {
            // This is a placeholder for the forget-results logic
            // Implement the actual forget-results functionality here
            println!("Forgetting results for test '{}'", test);
            Ok(())
        }
    }

    pub mod list {
        use super::*;
        use colored::*;

        pub async fn cmd_list(repo: &GitRepository) -> Result<()> {
            let tests = repo.list_tests().await?;

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
        use crate::log_util::log_and_run_command;
        use std::path::Path;
        use tokio::process::Command;

        pub async fn cmd_run(
            repo: &GitRepository,
            test: Option<&str>,
            all: bool,
            force: bool,
            forget: bool,
            retest: bool,
            keep_going: bool,
            dry_run: bool,
            stdin: bool,
            commits: &[String],
            worktree: Option<&Path>,
        ) -> Result<()> {
            if test.is_some() && all {
                anyhow::bail!("Cannot specify both --test and --all");
            }

            let tests = if all {
                repo.list_tests().await?
            } else if let Some(test_name) = test {
                vec![(
                    test_name.to_string(),
                    repo.get_test_command(test_name).await?.value().to_string(),
                )]
            } else {
                anyhow::bail!("Must specify either --test or --all");
            };

            let worktree_base = worktree.unwrap_or_else(|| Path::new(".worktrees"));
            let worktree_base = if worktree_base.is_relative() {
                repo.root().join(worktree_base)
            } else {
                worktree_base.to_path_buf()
            };

            let commits = if commits.is_empty() {
                vec![repo.get_head_commit().await?]
            } else {
                commits.to_vec()
            };

            for commit in commits {
                let test_results =
                    run_tests_for_commit(repo, &commit, &tests, &worktree_base).await?;
                update_git_notes(repo, &commit, &test_results).await?;
            }

            Ok(())
        }

        async fn run_tests_for_commit(
            repo: &GitRepository,
            commit: &str,
            tests: &[(String, String)],
            worktree_base: &Path,
        ) -> Result<Vec<TestResult>> {
            let tasks: Vec<_> = tests
                .iter()
                .map(|(test_name, test_command)| {
                    let repo = repo.clone();
                    let commit = commit.to_string();
                    let test_name = test_name.clone();
                    let test_command = test_command.clone();
                    let worktree_base = worktree_base.to_path_buf();

                    tokio::spawn(async move {
                        run_single_test(&repo, &commit, &test_name, &test_command, &worktree_base)
                            .await
                    })
                })
                .collect();

            let results = futures::future::join_all(tasks).await;
            results
                .into_iter()
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| anyhow::anyhow!("Task join error: {}", e))?
                .into_iter()
                .collect::<Result<Vec<_>, _>>()
        }

        async fn run_single_test(
            repo: &GitRepository,
            commit: &str,
            test_name: &str,
            test_command: &str,
            worktree_base: &Path,
        ) -> Result<TestResult> {
            let worktree_path = worktree_base.join(format!("{}/{}", commit, test_name));
            create_worktree(repo, commit, &worktree_path).await?;

            let mut cmd = Command::new("sh");
            cmd.arg("-c").arg(test_command).current_dir(&worktree_path);

            let output = log_and_run_command(&mut cmd).await?;

            let success = output.status.success();
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

            // Clean up the worktree after the test
            cleanup_worktree(repo, &worktree_path).await?;

            Ok(TestResult {
                test_name: test_name.to_string(),
                success,
                stdout,
                stderr,
            })
        }

        struct TestResult {
            test_name: String,
            success: bool,
            stdout: String,
            stderr: String,
        }

        async fn create_worktree(repo: &GitRepository, commit: &str, path: &Path) -> Result<()> {
            tokio::fs::create_dir_all(path).await?;
            repo.run_git(&[
                "worktree",
                "add",
                "--detach",
                path.to_str().unwrap(),
                commit,
            ])
            .await?;
            Ok(())
        }

        async fn cleanup_worktree(repo: &GitRepository, path: &Path) -> Result<()> {
            repo.run_git(&["worktree", "remove", "--force", path.to_str().unwrap()])
                .await?;
            Ok(())
        }

        async fn update_git_notes(
            repo: &GitRepository,
            commit: &str,
            results: &[TestResult],
        ) -> Result<()> {
            for result in results {
                let status = if result.success { "✓" } else { "✗" };
                repo.add_note(
                    &format!("refs/notes/tests/{}", result.test_name),
                    &format!("{}^{{tree}}", commit),
                    status,
                )
                .await?;
            }

            let summary = results
                .iter()
                .map(|r| format!("{}: {}", r.test_name, if r.success { "✓" } else { "✗" }))
                .collect::<Vec<_>>()
                .join("\n");

            repo.add_note("refs/notes/commits", commit, &summary)
                .await?;

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

#[tokio::main]
pub async fn main() -> Result<()> {
    use crate::git::get_repo_root;
    use crate::log_util::init_logging;
    use cli::{Cli, Commands};

    let cli = Cli::parse();

    // Calculate verbosity and set up logger
    let verbosity = cli.verbose as i8 - cli.quiet as i8;
    init_logging(verbosity, cli.color)?;

    // Get the repository root
    let current_dir = std::env::current_dir()?;
    let repo = get_repo_root(&current_dir).await?;

    match &cli.command {
        Commands::Add(args) => {
            commands::cmd_add(&repo, &args.test, args.forget, args.keep, &args.command).await
        }
        Commands::List => commands::cmd_list(&repo).await,
        Commands::Run(args) => {
            commands::cmd_run(
                &repo,
                args.test.as_deref(),
                args.all,
                args.force,
                args.forget,
                args.retest,
                args.keep_going,
                args.dry_run,
                args.stdin,
                &args.commits,
                args.worktree.as_deref(),
            )
            .await
        }
        _ => unimplemented!("Other commands need to be updated"),
    }
}
