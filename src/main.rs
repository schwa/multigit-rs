use anyhow::{anyhow, Context, Ok, Result};
use clap::{Parser, Subcommand};
use colored_markup::{println_markup, StyleSheet};
use git2::RepositoryInitMode;
use path_absolutize::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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
    List {
        #[arg(short, long)]
        filter: Vec<Filter>,
    },
    // Pull {},
    // Push {},
    /// Register a git repositories or a directories of git repositories
    Register { paths: Vec<PathBuf> },
    // Reveal {},
    Status {
        #[arg(short, long)]
        filter: Vec<Filter>,
    },
    UI {
        #[arg(short, long)]
        filter: Vec<Filter>,
    },
    /// Unregister a git repositories or a directories of git repositories
    Unregister { paths: Vec<PathBuf> },
}

fn main() {
    let cli = Cli::parse();

    let mut multigit = Multigit::new().unwrap();

    match &cli.command {
        Commands::List { filter } => {
            multigit.list(noneify(filter)).unwrap();
        }
        Commands::Register { paths } => {
            multigit.register(paths).unwrap();
        }
        Commands::Status { filter } => {
            multigit.status(noneify(filter)).unwrap();
        }
        Commands::Unregister { paths } => {
            multigit.unregister(paths).unwrap();
        }
        Commands::UI { filter } => {
            multigit.ui(noneify(filter)).unwrap();
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct RepositoryEntry {
    path: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
struct DirectoryEntry {
    path: PathBuf,
}

impl RepositoryEntry {
    fn state(&self) -> Result<RepositoryState> {
        let mut state = RepositoryState {
            entries: HashSet::new(),
        };

        let git_repo = git2::Repository::open(&self.path)?;
        let mut status_options = git2::StatusOptions::new();
        status_options.include_untracked(true);
        status_options.include_ignored(false);
        let statuses = git_repo.statuses(Some(&mut status_options))?;
        for status in statuses.into_iter() {
            match status.status() {
                git2::Status::INDEX_NEW => {
                    state.entries.insert(EntryState::Dirty);
                }
                git2::Status::INDEX_MODIFIED => {
                    state.entries.insert(EntryState::Dirty);
                }
                git2::Status::INDEX_DELETED => {
                    state.entries.insert(EntryState::Dirty);
                }
                git2::Status::INDEX_RENAMED => {
                    state.entries.insert(EntryState::Dirty);
                }
                git2::Status::INDEX_TYPECHANGE => {
                    state.entries.insert(EntryState::Dirty);
                }
                git2::Status::WT_NEW => {
                    state.entries.insert(EntryState::Dirty);
                }
                git2::Status::WT_MODIFIED => {
                    state.entries.insert(EntryState::Dirty);
                }
                git2::Status::WT_DELETED => {
                    state.entries.insert(EntryState::Dirty);
                }
                git2::Status::WT_TYPECHANGE => {
                    state.entries.insert(EntryState::Dirty);
                }
                git2::Status::WT_RENAMED => {
                    state.entries.insert(EntryState::Dirty);
                }
                git2::Status::CONFLICTED => {
                    state.entries.insert(EntryState::Dirty);
                }
                _ => {}
            }
        }
        Ok(state)
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct Config {
    #[serde(default = "HashMap::new")]
    repositories: HashMap<String, RepositoryEntry>,

    #[serde(default = "HashMap::new")]
    directories: HashMap<String, DirectoryEntry>,
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

    fn register(&mut self, path: &Path) -> Result<()> {
        let absolute_path = path.absolutize().context("Failed to get absolute path")?;
        let name = absolute_path
            .to_str()
            .context("Failed to convert path to string")?;

        if !is_git_repository(path) {
            let entry = DirectoryEntry {
                path: path.to_path_buf(),
            };
            self.directories.insert(name.to_string(), entry);
        } else {
            let entry = RepositoryEntry {
                path: path.to_path_buf(),
            };
            self.repositories.insert(name.to_string(), entry);
        }
        self.save()?;
        Ok(())
    }

    fn unregister(&mut self, path: &PathBuf) -> Result<()> {
        let absolute_path = path.absolutize().context("Failed to get absolute path")?;
        let name = absolute_path
            .to_str()
            .context("Failed to convert path to string")?;
        self.directories.remove(name);
        self.repositories.remove(name);
        self.save()?;
        Ok(())
    }
}

#[derive(Debug)]
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

    fn all_repositories(&self, filter: Option<&Vec<Filter>>) -> Result<Vec<RepositoryEntry>> {
        let mut repositories: Vec<RepositoryEntry> = Vec::new();
        for (_, repository) in self.config.repositories.iter() {
            repositories.push(RepositoryEntry {
                path: repository.path.clone(),
            });
        }
        for (_, directory) in self.config.directories.iter() {
            let directory_repositories = find_repositories(&directory.path)?;
            for repository in directory_repositories {
                let repository = RepositoryEntry { path: repository };
                repositories.push(repository);
            }
        }

        if let Some(filter) = filter {
            if !filter.is_empty() {
                repositories = repositories
                    .into_iter()
                    .filter(|repository| {
                        let state = repository.state().unwrap();
                        for f in filter {
                            match f {
                                Filter::Dirty => {
                                    if state.entries.contains(&EntryState::Dirty) {
                                        return true;
                                    }
                                }
                            }
                        }
                        return false;
                    })
                    .collect();
            }
        }

        repositories.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(repositories)
    }

    fn register(&mut self, paths: &Vec<PathBuf>) -> Result<()> {
        if paths.is_empty() {
            self.config.register(&std::env::current_dir()?)?;
        } else {
            for path in paths {
                self.config.register(path)?;
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
                self.config.unregister(path)?;
            }
        }
        self.config.save()?;
        Ok(())
    }

    fn list(&self, filter: Option<&Vec<Filter>>) -> Result<()> {
        for repository in self.all_repositories(filter)?.iter() {
            println_markup!(
                &self.style_sheet,
                "<repopath>{}</repopath>",
                repository.path.to_str().unwrap(),
            );
        }
        Ok(())
    }

    fn status(&self, filter: Option<&Vec<Filter>>) -> Result<()> {
        let mut status_options = git2::StatusOptions::new();
        status_options.include_untracked(true);
        status_options.include_ignored(false);

        for repository in self.all_repositories(filter)?.iter() {
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

    fn ui(&self, filter: Option<&Vec<Filter>>) -> Result<()> {
        for repository in self.all_repositories(filter)?.iter() {
            println!("Opening git ui for {}", repository.path.to_str().unwrap());
            //open_in_git_ui(&repository.path)?;
        }
        Ok(())
    }
}

#[derive(clap::ValueEnum, Clone, Debug, Serialize)]
enum Filter {
    Dirty,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
enum EntryState {
    Clean,
    Dirty,
}

struct RepositoryState {
    entries: HashSet<EntryState>,
}

fn open_in_git_ui(path: &Path) -> Result<()> {
    let editor = "gitup";
    let status = std::process::Command::new(editor).arg(path).status()?;
    if !status.success() {
        return Err(anyhow!("Failed to open git ui"));
    }
    Ok(())
}

fn find_repositories(path: &Path) -> Result<Vec<PathBuf>> {
    let mut repositories = Vec::new();
    let walker = WalkDir::new(path).into_iter().filter_entry(|e| {
        e.file_type().is_dir() && !is_hidden(e.path()) && e.path().file_name().unwrap() != ".git"
    });
    for entry in walker {
        let entry = entry?;
        if is_git_repository(entry.path()) {
            let path = entry.path();
            repositories.push(path.to_path_buf());
            // walker.skip_current_dir();
        }
    }
    Ok(repositories)
}

fn is_git_repository(path: &Path) -> bool {
    path.join(".git").exists()
}

fn is_hidden(path: &Path) -> bool {
    path.file_name().unwrap().to_str().unwrap().starts_with(".")
}

fn noneify<T>(v: &Vec<T>) -> Option<&Vec<T>> {
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}
