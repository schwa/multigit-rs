use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
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
    Register { paths: Vec<PathBuf> },
    // Reveal {},
    // Status {},
    // UI {},
    /// Unregister a git repositories or a directories of git repositories
    Unregister { paths: Vec<PathBuf> },
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
        Commands::Unregister { paths } => {
            multigit.unregister(paths).unwrap();
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Repository {
    path: PathBuf,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct Config {
    repositories: std::collections::HashMap<String, Repository>,
}

impl Config {
    fn load() -> Result<Self> {
        let config_path = "~/.config/multigit/config.toml";
        let config_path = shellexpand::tilde(config_path);
        let config_path = config_path.to_string();
        let config_content = std::fs::read_to_string(config_path).unwrap();
        toml::from_str(&config_content).map_err(|e| anyhow!(e))
    }

    fn save(&self) -> Result<()> {
        let config_path = "~/.config/multigit/config.toml";
        let config_path = shellexpand::tilde(config_path);
        let config_path = config_path.to_string();
        let config_content = toml::to_string(&self).unwrap();
        std::fs::write(config_path, config_content).unwrap();
        Ok(())
    }
}

struct Multigit {
    config: Config,
}

impl Multigit {
    fn new() -> Result<Self> {
        let config = Config::load()?;
        Ok(Self { config })
    }

    fn register(&mut self, paths: &Vec<PathBuf>) -> Result<()> {
        if paths.is_empty() {
            // add current directory
            let path = std::env::current_dir()?;
            let name = path.to_string_lossy().to_string();
            let repository = Repository { path: path.clone() };
            self.config.repositories.insert(name, repository);
            self.config.save()?;
        } else {
            for path in paths {
                let name = path.to_string_lossy().to_string();
                let repository = Repository { path: path.clone() };
                self.config.repositories.insert(name, repository);
            }
            self.config.save()?;
        }
        Ok(())
    }

    fn unregister(&mut self, paths: &Vec<PathBuf>) -> Result<()> {
        if paths.is_empty() {
            // remove current directory
            let path = std::env::current_dir()?;
            let name = path.to_string_lossy().to_string();
            self.config.repositories.remove(&name);
            self.config.save()?;
        } else {
            for path in paths {
                let name = path.to_string_lossy().to_string();
                self.config.repositories.remove(&name);
            }
            self.config.save()?;
        }
        Ok(())
    }

    fn list(self) -> Result<()> {
        for (_, repository) in self.config.repositories.iter() {
            println!("{:?}", repository.path);
        }
        Ok(())
    }
}
