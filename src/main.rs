//! A multi-command CLI program to manage multiple Git repositories.
//!
//! This program allows users to perform Git operations across multiple repositories simultaneously.
//! It supports commands like `add`, `commit`, `push`, `pull`, `exec`, `list`, `register`, `status`, `ui`, and `unregister`.

use anyhow::Result;
use clap::{Parser, Subcommand};
use multigit_rs::*;
use std::path::PathBuf;

/// The main CLI struct that defines the command-line interface.
#[derive(Parser)]
#[clap(name = "My CLI Program")]
#[clap(author = "Your Name")]
#[clap(version = "1.0")]
#[clap(about = "A multi-command CLI example", long_about = None)]
struct Cli {
    /// The subcommand to execute.
    #[clap(subcommand)]
    command: Commands,
}

/// Enum representing the possible commands.
#[derive(Subcommand)]
enum Commands {
    /// Add files to the staging area in the selected repositories.
    Add {
        /// Filters to select specific repositories.
        #[arg(short, long)]
        filter: Vec<Filter>,

        /// Additional arguments to pass through to the `git add` command.
        passthrough: Vec<String>,
    },
    /// Commit changes in the selected repositories.
    Commit {
        /// Filters to select specific repositories.
        #[arg(short, long)]
        filter: Vec<Filter>,

        /// Additional arguments to pass through to the `git commit` command.
        #[clap(trailing_var_arg = true, allow_hyphen_values = true)]
        passthrough: Vec<String>,
    },
    /// Push changes to remote repositories.
    Push {
        /// Filters to select specific repositories.
        #[arg(short, long)]
        filter: Vec<Filter>,

        /// Additional arguments to pass through to the `git push` command.
        #[clap(trailing_var_arg = true, allow_hyphen_values = true)]
        passthrough: Vec<String>,
    },
    /// Pull changes from remote repositories.
    Pull {
        /// Filters to select specific repositories.
        #[arg(short, long)]
        filter: Vec<Filter>,

        /// Additional arguments to pass through to the `git pull` command.
        #[clap(trailing_var_arg = true, allow_hyphen_values = true)]
        passthrough: Vec<String>,
    },
    /// Execute a custom command in the selected repositories.
    Exec {
        /// Filters to select specific repositories.
        #[arg(short, long)]
        filter: Vec<Filter>,

        /// The command to execute.
        #[clap(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    /// List registered repositories.
    ///
    /// Shows the list of repositories currently managed by the tool.
    List {
        /// Filters to select specific repositories.
        #[arg(short, long)]
        filter: Vec<Filter>,
    },
    /// Register git repositories or directories of git repositories.
    ///
    /// Adds new repositories to be managed by the tool.
    Register {
        /// Paths to repositories or directories containing repositories.
        paths: Vec<PathBuf>,
    },
    /// Show the status of repositories.
    Status {
        /// Filters to select specific repositories.
        #[arg(short, long)]
        filter: Vec<Filter>,
    },
    /// Open the configured git UI program for the selected repositories.
    UI {
        /// Filters to select specific repositories.
        #[arg(short, long)]
        filter: Vec<Filter>,
    },
    /// Unregister git repositories or directories of git repositories.
    ///
    /// Removes repositories from being managed by the tool.
    Unregister {
        /// Unregister all repositories.
        #[arg(short, long)]
        all: bool,

        /// Paths to repositories or directories to unregister.
        paths: Vec<PathBuf>,
    },
}

/// The main entry point of the program.
fn main() -> Result<()> {
    // Parse command-line arguments into the `Cli` struct.
    let cli = Cli::parse();

    // Create a new instance of `Multigit`.
    let mut multigit = Multigit::new().unwrap();

    // Match the provided command and execute the corresponding action.
    match &cli.command {
        Commands::List { filter } => multigit.list(noneify(filter)),
        Commands::Register { paths } => multigit.register(paths),
        Commands::Status { filter } => multigit.status(noneify(filter)),
        Commands::Unregister { paths, all } => multigit.unregister(paths, all),
        Commands::UI { filter } => multigit.ui(noneify(filter)),
        Commands::Exec { filter, command } => multigit.exec(noneify(filter), command),
        Commands::Add {
            filter,
            passthrough,
        } => multigit.add(noneify(filter), passthrough),
        Commands::Commit {
            filter,
            passthrough,
        } => multigit.commit(noneify(filter), passthrough),
        Commands::Push {
            filter,
            passthrough,
        } => multigit.push(noneify(filter), passthrough),
        Commands::Pull {
            filter,
            passthrough,
        } => multigit.pull(noneify(filter), passthrough),
    }
}
