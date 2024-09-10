use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use colored_markup::{println_markup, StyleSheet};
use git2;
use path_absolutize::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

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
    // Add {},
    // Commit {},
    // Config {},
    // Edit {},
    // Exec {},
    // GC {},
    // Info {},
    /// List repositories.
    List {},
    // Pull {},
    // Push {},
    /// Register a git repositories or a directories of git repositories
    Register {
        paths: Vec<PathBuf>,
    },
    // Reveal {},
    Status {},
    // UI {},
    /// Unregister a git repositories or a directories of git repositories
    Unregister {
        paths: Vec<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    let mut multigit = Multigit::new().unwrap();

    match &cli.command {
        Commands::List {} => {
            multigit.list().unwrap();
        }
        Commands::Register { paths } => {
            multigit.register(paths).unwrap();
        }
        Commands::Status {} => {
            multigit.status().unwrap();
        }
        Commands::Unregister { paths } => {
            multigit.unregister(paths).unwrap();
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct RepositoryEntry {
    path: PathBuf,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct Config {
    #[serde(default = "HashMap::new")]
    repositories: HashMap<String, RepositoryEntry>,

    #[serde(default = "HashSet::new")]
    directories: HashSet<PathBuf>,
}

impl Config {
    fn load() -> Result<Self> {
        let config_path = "~/.config/multigit/config.toml";
        let config_path = shellexpand::tilde(config_path);
        let config_path = config_path.to_string();
        let config_content = std::fs::read_to_string(config_path)?;
        toml::from_str(&config_content).map_err(|e| anyhow!(e))
    }

    fn save(&self) -> Result<()> {
        let config_path = "~/.config/multigit/config.toml";
        let config_path = shellexpand::tilde(config_path);
        let config_path = config_path.to_string();
        let config_content = toml::to_string(&self)?;
        std::fs::write(config_path, config_content)?;
        Ok(())
    }

    fn register(&mut self, path: &PathBuf) -> Result<()> {
        let absolute_path = path.absolutize().context("Failed to get absolute path")?;
        let name = absolute_path
            .to_str()
            .context("Failed to convert path to string")?;
        let repository = RepositoryEntry {
            path: path.to_path_buf(),
        };
        self.repositories.insert(name.to_string(), repository);
        self.save()?;
        Ok(())
    }

    fn unregister(&mut self, path: &PathBuf) -> Result<()> {
        let absolute_path = path.absolutize().context("Failed to get absolute path")?;
        let name = absolute_path
            .to_str()
            .context("Failed to convert path to string")?;
        self.repositories.remove(name);
        self.save()?;
        Ok(())
    }
}

struct Multigit {
    config: Config,
    style_sheet: StyleSheet<'static>,
}

impl Multigit {
    fn new() -> Result<Self> {
        let config = Config::load()?;

        let style_sheet = StyleSheet::parse(
            "
        repopath { foreground: cyan; }
        status { foreground: yellow; }
        ",
        )
        .unwrap();

        Ok(Self {
            config,
            style_sheet,
        })
    }

    fn register(&mut self, paths: &Vec<PathBuf>) -> Result<()> {
        if paths.is_empty() {
            self.config.register(&std::env::current_dir()?)?;
        } else {
            for path in paths {
                self.config.register(&path)?;
            }
        }
        self.config.save()?;
        Ok(())
    }

    fn unregister(&mut self, paths: &Vec<PathBuf>) -> Result<()> {
        if paths.is_empty() {
            self.config.unregister(&std::env::current_dir()?)?;
        } else {
            for path in paths {
                self.config.unregister(&path)?;
            }
        }
        self.config.save()?;
        Ok(())
    }

    fn list(&self) -> Result<()> {
        for (_, repository) in self.config.repositories.iter() {
            println!("{:?}", repository.path);
        }
        Ok(())
    }

    fn status(&self) -> Result<()> {
        let mut status_options = git2::StatusOptions::new();
        status_options.include_untracked(true);
        status_options.include_ignored(false);

        for (_, repository) in self.config.repositories.iter() {
            let repo = git2::Repository::open(&repository.path)?;
            let status = repo.statuses(Some(&mut status_options))?;
            if !status.is_empty() {
                let mut index_new: bool = false;
                let mut index_modified: bool = false;
                let mut index_deleted: bool = false;
                let mut index_renamed: bool = false;
                let mut index_typechange: bool = false;
                let mut wt_new: bool = false;
                let mut wt_modified: bool = false;
                let mut wt_deleted: bool = false;
                let mut wt_typechange: bool = false;
                let mut wt_renamed: bool = false;
                let mut ignored: bool = false;
                let mut conflicted: bool = false;

                for entry in status.iter() {
                    if entry.status() == git2::Status::INDEX_NEW {
                        index_new = true;
                    }
                    if entry.status() == git2::Status::INDEX_MODIFIED {
                        index_modified = true;
                    }
                    if entry.status() == git2::Status::INDEX_DELETED {
                        index_deleted = true;
                    }
                    if entry.status() == git2::Status::INDEX_RENAMED {
                        index_renamed = true;
                    }
                    if entry.status() == git2::Status::INDEX_TYPECHANGE {
                        index_typechange = true;
                    }
                    if entry.status() == git2::Status::WT_NEW {
                        wt_new = true;
                    }
                    if entry.status() == git2::Status::WT_MODIFIED {
                        wt_modified = true;
                    }
                    if entry.status() == git2::Status::WT_DELETED {
                        wt_deleted = true;
                    }
                    if entry.status() == git2::Status::WT_TYPECHANGE {
                        wt_typechange = true;
                    }
                    if entry.status() == git2::Status::WT_RENAMED {
                        wt_renamed = true;
                    }
                    if entry.status() == git2::Status::IGNORED {
                        ignored = true;
                    }
                    if entry.status() == git2::Status::CONFLICTED {
                        conflicted = true;
                    }
                }

                let mut status_string = "".to_string();

                if index_new {
                    status_string.push_str(" [new]");
                }
                if index_modified {
                    status_string.push_str(" [modified]");
                }
                if index_deleted {
                    status_string.push_str(" [deleted]");
                }
                if index_renamed {
                    status_string.push_str(" [renamed]");
                }
                if index_typechange {
                    status_string.push_str(" [typechange]");
                }
                if wt_new {
                    status_string.push_str(" [wt-new]");
                }
                if wt_modified {
                    status_string.push_str(" [wt-modified]");
                }
                if wt_deleted {
                    status_string.push_str(" [wt-deleted]");
                }
                if wt_typechange {
                    status_string.push_str(" [wt-typechange]");
                }
                if wt_renamed {
                    status_string.push_str(" [wt-renamed]");
                }
                if ignored {
                    status_string.push_str(" [ignored]");
                }
                if conflicted {
                    status_string.push_str(" [conflicted]");
                }

                println_markup!(
                    &self.style_sheet,
                    "<repopath>{}</repopath><status>{}</status>",
                    repository.path.to_str().unwrap(),
                    status_string
                );
            }
        }
        Ok(())
    }
}
