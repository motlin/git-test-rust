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
