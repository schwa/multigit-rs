use anyhow::{anyhow, Context, Ok, Result};
use clap::{Parser, Subcommand};
use colored_markup::{println_markup, StyleSheet};
use inquire::Confirm;
use path_absolutize::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use multigit_rs::*;

#[derive(Parser)]
#[clap(name = "My CLI Program")]
#[clap(author = "Your Name")]
#[clap(version = "1.0")]
#[clap(about = "A multi-command CLI example", long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Add {
        #[arg(short, long)]
        filter: Vec<Filter>,

        passthrough: Vec<String>,
    },
    Commit {
        #[arg(short, long)]
        filter: Vec<Filter>,

        passthrough: Vec<String>,
    },
    Push {
        #[arg(short, long)]
        filter: Vec<Filter>,

        passthrough: Vec<String>,
    },
    Pull {
        #[arg(short, long)]
        filter: Vec<Filter>,

        passthrough: Vec<String>,
    },
    // Config {},
    // Edit {},
    Exec {
        #[arg(short, long)]
        filter: Vec<Filter>,

        command: Vec<String>,
    },
    // GC {},
    // Info {},
    /// List repositories.
    List {
        #[arg(short, long)]
        filter: Vec<Filter>,
    },
    // Pull {},
    // Push {},
    /// Register a git repositories or directories of git repositories
    Register { paths: Vec<PathBuf> },
    // Reveal {},
    Status {
        #[arg(short, long)]
        filter: Vec<Filter>,
    },
    /// Open the configured git ui program for the selected repositories.
    UI {
        #[arg(short, long)]
        filter: Vec<Filter>,
    },
    /// Unregister a git repositories or directories of git repositories
    Unregister {
        #[arg(short, long)]
        all: bool,

        paths: Vec<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut multigit = Multigit::new().unwrap();

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
