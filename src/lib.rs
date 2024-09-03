use anyhow::Result;
use clap::Parser;
use log::LevelFilter;
use simple_logger::SimpleLogger;

mod cli;
pub mod commands;
pub mod git;

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
