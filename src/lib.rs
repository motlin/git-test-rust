use anyhow::Result;
use clap::Parser;
use std::io::{self, Write};

mod cli;
pub mod commands;
pub mod git;

use cli::{Cli, Commands};
use commands::*;

pub fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up colored output
    colored::control::set_override(cli.color && !cli.no_color);

    // Calculate verbosity
    let verbosity = cli.verbose as i8 - cli.quiet as i8;

    // Get the repository root
    let repo_root = git::get_repo_root()?;

    // Create a writer for stdout
    let mut writer = io::stdout();

    match &cli.command {
        Commands::Add(args) => cmd_add(
            &repo_root,
            &args.test,
            args.forget,
            args.keep,
            &args.command,
            verbosity,
            &mut writer,
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
            verbosity,
            &mut writer,
        ),
        Commands::Results(args) => cmd_results(
            &repo_root,
            &args.test,
            args.stdin,
            &args.commits,
            verbosity,
            &mut writer,
        ),
        Commands::ForgetResults(args) => {
            cmd_forget_results(&repo_root, &args.test, verbosity, &mut writer)
        }
        Commands::List => cmd_list(&repo_root, verbosity, &mut writer),
        Commands::Remove(args) => cmd_remove(&repo_root, &args.test, verbosity, &mut writer),
    }
}
