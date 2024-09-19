//! A library for managing multiple Git repositories.
//!
//! This library provides functionalities to register, unregister, list, and perform Git operations on multiple repositories.
//! It supports filtering repositories based on their state and provides utilities to execute commands across repositories.

use anyhow::{anyhow, Context, Result};
use colored_markup::{println_markup, StyleSheet};
use inquire::Confirm;
use path_absolutize::Absolutize;
use patharg::InputArg;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::io::Read;
use std::path::{Display, Path, PathBuf};
use std::process::Command;
use tabled::{Table, Tabled};
use walkdir::WalkDir;

/// Represents an entry for a single Git repository.
#[derive(Debug, Deserialize, Serialize)]
pub struct RepositoryEntry {
    /// The path to the repository.
    pub path: PathBuf,
}

impl RepositoryEntry {
    fn current_branch(&self) -> Result<String> {
        let repo = git2::Repository::open(&self.path)?;
        let head = repo.head()?;
        let branch = head.shorthand().unwrap();
        Ok(branch.to_string())
    }

    fn behind_remote(&self) -> Result<Option<bool>> {
        let repo = git2::Repository::open(&self.path)?;
        let head = repo.head()?;
        let branch = head.shorthand().unwrap();
        let branch = repo.find_branch(branch, git2::BranchType::Local)?;
        if branch.upstream().is_err() {
            return Ok(None);
        }
        let upstream = branch.upstream()?;
        let (_, behind) = repo.graph_ahead_behind(
            branch.get().target().unwrap(),
            upstream.get().target().unwrap(),
        )?;
        Ok(Some(behind > 0))
    }

    fn ahead_remote(&self) -> Result<Option<bool>> {
        let repo = git2::Repository::open(&self.path)?;
        let head = repo.head()?;
        let branch = head.shorthand().unwrap();
        let branch = repo.find_branch(branch, git2::BranchType::Local)?;
        // if no upstream is set, return None
        if branch.upstream().is_err() {
            return Ok(None);
        }
        let upstream = branch.upstream()?;
        let (ahead, _) = repo.graph_ahead_behind(
            branch.get().target().unwrap(),
            upstream.get().target().unwrap(),
        )?;
        Ok(Some(ahead > 0))
    }

    fn has_stashes(&self) -> Result<bool> {
        let mut repo = git2::Repository::open(&self.path)?;
        let mut has_stashes = false;
        repo.stash_foreach(|_, _, _| {
            has_stashes = true;
            false
        })?;
        Ok(has_stashes)
    }
}

/// Represents an entry for a directory containing Git repositories.
#[derive(Debug, Deserialize, Serialize)]
pub struct DirectoryEntry {
    /// The path to the directory.
    pub path: PathBuf,
}

impl RepositoryEntry {
    /// Retrieves the state of the repository.
    ///
    /// Returns a `RepositoryState` containing information about the repository's status.
    pub fn state(&self) -> Result<RepositoryState> {
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
                git2::Status::INDEX_NEW
                | git2::Status::INDEX_MODIFIED
                | git2::Status::INDEX_DELETED
                | git2::Status::INDEX_RENAMED
                | git2::Status::INDEX_TYPECHANGE
                | git2::Status::WT_NEW
                | git2::Status::WT_MODIFIED
                | git2::Status::WT_DELETED
                | git2::Status::WT_TYPECHANGE
                | git2::Status::WT_RENAMED
                | git2::Status::CONFLICTED => {
                    state.entries.insert(EntryState::Dirty);
                }
                _ => {}
            }
        }
        anyhow::Ok(state)
    }

    #[allow(dead_code)]
    fn is_dirty(&self) -> bool {
        let state = self.state().unwrap();
        state.entries.contains(&EntryState::Dirty)
    }
}

/// Configuration data for the application, including registered repositories and directories.
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Config {
    /// A map of repository names to their entries.
    #[serde(default = "HashMap::new")]
    pub repositories: HashMap<String, RepositoryEntry>,

    /// A map of directory names to their entries.
    #[serde(default = "HashMap::new")]
    pub directories: HashMap<String, DirectoryEntry>,
}

impl Config {
    /// Loads the configuration from the default config file.
    pub fn load(path: InputArg) -> Result<Self> {
        let content = match path {
            InputArg::Stdin => {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer)?;
                buffer
            }
            InputArg::Path(path) => {
                let expanded_path = shellexpand::tilde(path.to_str().unwrap());
                let config_path = PathBuf::from(expanded_path.to_string());
                fs::read_to_string(config_path)
                    .map_err(|e| anyhow!("Failed to read config file: {}", e))?
            }
        };

        toml::from_str(&content)
            .map_err(|e| anyhow!("Failed to parse config: {}", e))
            .or_else(|e| {
                println!("Failed to load config: {}. Using default configuration.", e);
                Ok(Config::default())
            })
    }

    /// Saves the current configuration to the default config file.
    pub fn save(&self) -> Result<()> {
        let config_path = "~/.config/multigit/config.toml";
        let config_path = shellexpand::tilde(config_path);
        let config_path = config_path.to_string();
        let config_content = toml::to_string(&self)?;
        std::fs::write(config_path, config_content)?;
        anyhow::Ok(())
    }

    /// Registers a path as a repository or directory.
    ///
    /// If the path is a Git repository, it is added to the repositories map.
    /// If the path is a directory containing repositories, it is added to the directories map.
    pub fn register(&mut self, path: &Path) -> Result<()> {
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
        anyhow::Ok(())
    }

    /// Unregisters a repository or directory.
    pub fn unregister(&mut self, path: &PathBuf) -> Result<()> {
        let absolute_path = path.absolutize().context("Failed to get absolute path")?;
        let name = absolute_path
            .to_str()
            .context("Failed to convert path to string")?;
        self.directories.remove(name);
        self.repositories.remove(name);
        self.save()?;
        anyhow::Ok(())
    }
}

/// Represents the main application handling multiple repositories.
#[derive(Debug)]
pub struct Multigit {
    /// The configuration containing repositories and directories.
    pub config: Config,

    pub directory: Option<PathBuf>,

    /// The stylesheet used for colored output.
    pub style_sheet: StyleSheet<'static>,
}

impl Multigit {
    /// Creates a new instance of `Multigit`.
    pub fn new(config: Config, directory: Option<PathBuf>) -> Result<Self> {
        let style_sheet = StyleSheet::parse(
            "
            repository { foreground: cyan; }
            status { foreground: yellow; }
            command { foreground: green; }
            divider { foreground: red; }
            ",
        )
        .unwrap();

        anyhow::Ok(Self {
            config,
            directory,
            style_sheet,
        })
    }

    /// Retrieves all repositories, optionally filtering them.
    fn all_repositories(&self, filter: Option<&Vec<Filter>>) -> Result<Vec<RepositoryEntry>> {
        let mut repositories: Vec<RepositoryEntry> = Vec::new();

        if self.directory.is_some() {
            let directory = self.directory.as_ref().unwrap();
            let directory_repositories = find_repositories(directory)?;
            let mut repositories: Vec<RepositoryEntry> = Vec::new();
            for repository in directory_repositories {
                let repository = RepositoryEntry { path: repository };
                repositories.push(repository);
            }
            return Ok(repositories);
        } else {
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
        }

        if let Some(filter) = filter {
            if !filter.is_empty() {
                repositories.retain(|repository| {
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
                    false
                });
            }
        }

        repositories.sort_by(|a, b| a.path.cmp(&b.path));
        anyhow::Ok(repositories)
    }

    #[allow(dead_code)]
    fn iter_repositories(
        &self,
        filter: Option<&Vec<Filter>>,
    ) -> Result<impl Iterator<Item = RepositoryEntry>> {
        let repositories = self.all_repositories(filter)?;
        Ok(repositories.into_iter())
    }

    fn process_repositories<F>(
        &self,
        repositories: &[RepositoryEntry],
        mut process: F,
    ) -> Result<()>
    where
        F: FnMut(&RepositoryEntry) -> Result<()>,
    {
        let mut errors = Vec::new();

        for repository in repositories {
            if let Err(e) = process(repository) {
                eprintln!("Error processing repository {:?}: {}", repository.path, e);
                errors.push(RepositoryError {
                    path: repository.path.clone(),
                    error: e,
                });
            }
        }

        if errors.is_empty() {
            anyhow::Ok(())
        } else {
            Err(anyhow!("Errors occurred in {} repositories", errors.len()))
        }
    }

    /// Registers paths as repositories or directories.
    pub fn register(&mut self, paths: &Vec<PathBuf>) -> Result<()> {
        if paths.is_empty() {
            self.config.register(&std::env::current_dir()?)?;
        } else {
            for path in paths {
                self.config.register(path)?;
            }
        }
        self.config.save()?;
        anyhow::Ok(())
    }

    /// Unregisters repositories or directories.
    pub fn unregister(&mut self, paths: &Vec<PathBuf>, all: &bool) -> Result<()> {
        if *all {
            let ans = Confirm::new("Unregister all repositories and directories??")
                .with_default(false)
                .prompt()?;
            match ans {
                true => {
                    self.config.repositories.clear();
                    self.config.directories.clear();
                }
                false => {
                    return anyhow::Ok(());
                }
            }
        } else if paths.is_empty() {
            self.config.unregister(&std::env::current_dir()?)?;
        } else {
            for path in paths {
                self.config.unregister(path)?;
            }
        }
        self.config.save()?;
        anyhow::Ok(())
    }

    /// Lists all registered repositories.
    pub fn list(&self, filter: Option<&Vec<Filter>>, detailed: &bool) -> Result<()> {
        let repositories = self.all_repositories(filter)?;

        #[derive(Tabled)]
        struct Row<'a> {
            name: String,
            #[tabled(skip)]
            path: Display<'a>,
            state: RepositoryState,
            current_branch: String,
            #[tabled(display_with = "display_option")]
            behind_remote: Option<bool>,
            #[tabled(display_with = "display_option")]
            ahead_remote: Option<bool>,
            has_stashes: bool,
        }

        let rows = repositories.iter().map(|repository| {
            let name = repository
                .path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            let path = repository.path.display();
            Row {
                name,
                path,
                state: repository.state().unwrap(),
                current_branch: repository.current_branch().unwrap(),
                behind_remote: repository.behind_remote().ok().flatten(),
                ahead_remote: repository.ahead_remote().ok().flatten(),
                has_stashes: repository.has_stashes().unwrap(),
            }
        });

        if !detailed {
            for row in rows {
                println_markup!(&self.style_sheet, "<repository>{}</repository>", row.path);
            }
        } else {
            let table = Table::new(rows).to_string();
            println!("{}", table);
        }

        Ok(())
    }

    /// Shows the status of all repositories.
    pub fn status(&self, filter: Option<&Vec<Filter>>) -> Result<()> {
        let repositories = self.all_repositories(filter)?;
        self.process_repositories(&repositories, |repository| {
            let mut status_options = git2::StatusOptions::new();
            status_options.include_untracked(true);
            status_options.include_ignored(false);
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
                    match entry.status() {
                        git2::Status::INDEX_NEW => index_new = true,
                        git2::Status::INDEX_MODIFIED => index_modified = true,
                        git2::Status::INDEX_DELETED => index_deleted = true,
                        git2::Status::INDEX_RENAMED => index_renamed = true,
                        git2::Status::INDEX_TYPECHANGE => index_typechange = true,
                        git2::Status::WT_NEW => wt_new = true,
                        git2::Status::WT_MODIFIED => wt_modified = true,
                        git2::Status::WT_DELETED => wt_deleted = true,
                        git2::Status::WT_TYPECHANGE => wt_typechange = true,
                        git2::Status::WT_RENAMED => wt_renamed = true,
                        git2::Status::IGNORED => ignored = true,
                        git2::Status::CONFLICTED => conflicted = true,
                        _ => {}
                    }
                }

                let mut status_string = String::new();

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
                    "<repository>{}</repository><status>{}</status>",
                    repository.path.to_str().unwrap(),
                    status_string
                );
            }
            anyhow::Ok(())
        })
    }

    /// Opens the configured Git UI for the selected repositories.
    pub fn ui(&self, filter: Option<&Vec<Filter>>) -> Result<()> {
        let paths_to_open = self.all_repositories(filter)?;
        if paths_to_open.len() > 1 {
            let ans = Confirm::new(format!("Open {} repositories?", paths_to_open.len()).as_str())
                .with_default(false)
                .prompt()?;
            if !ans {
                return anyhow::Ok(());
            }
        }
        for repository in paths_to_open.iter() {
            println_markup!(
                &self.style_sheet,
                "Opening git ui for {}",
                repository.path.to_str().unwrap()
            );
            open_in_git_ui(&repository.path)?;
        }
        anyhow::Ok(())
    }

    /// Executes a custom command in the selected repositories.
    pub fn exec(&self, filter: Option<&Vec<Filter>>, commands: &[String]) -> Result<()> {
        let repositories = self.all_repositories(filter)?;
        self.process_repositories(&repositories, |repository| {
            let mut command = std::process::Command::new(&commands[0]);
            command.args(&commands[1..]);
            command.current_dir(&repository.path);
            let status = command.status()?;
            if !status.success() {
                return Err(anyhow!("Failed to execute command"));
            }
            Ok(())
        })
    }

    /// Executes a Git command with optional arguments in the selected repositories.
    pub fn git_command(
        &self,
        git_command: &str,
        filter: Option<&Vec<Filter>>,
        passthrough: &[String],
    ) -> Result<()> {
        let repositories = self.all_repositories(filter)?;

        let width = termsize::get().unwrap().cols as usize;

        let divider = "#".repeat(width);

        let mut first_repository = true;

        self.process_repositories(&repositories, |repository| {
            if !first_repository {
                println_markup!(&self.style_sheet, "\n<divider>{}</divider>\n", divider);
            }
            first_repository = false;
            println_markup!(
                &self.style_sheet,
                "Running `<command>{}</command>` in <repository>{}</repository>\n",
                git_command,
                repository.path.to_str().unwrap()
            );
            let mut args = vec![git_command];
            args.extend(passthrough.iter().map(|s| s.as_str()));
            let mut command = std::process::Command::new("git");
            command.args(&args);
            command.current_dir(&repository.path);

            // Execute the command and capture the status
            let status = command.status()?;

            // Check if the command was successful
            if !status.success() {
                return Err(anyhow!(
                    "Git command {} failed in repository `{}` with exit code {:?}",
                    git_command,
                    repository.path.display(),
                    status.code()
                ));
            }
            Ok(())
        })
    }

    /// Commits changes in the selected repositories.
    pub fn commit(&self, filter: Option<&Vec<Filter>>, passthrough: &[String]) -> Result<()> {
        self.git_command("commit", filter, passthrough)
    }

    /// Adds files to the staging area in the selected repositories.
    pub fn add(&self, filter: Option<&Vec<Filter>>, passthrough: &[String]) -> Result<()> {
        self.git_command("add", filter, passthrough)
    }

    /// Pushes changes to remote repositories.
    pub fn push(&self, filter: Option<&Vec<Filter>>, passthrough: &[String]) -> Result<()> {
        self.git_command("push", filter, passthrough)
    }

    /// Pulls changes from remote repositories.
    pub fn pull(&self, filter: Option<&Vec<Filter>>, passthrough: &[String]) -> Result<()> {
        self.git_command("pull", filter, passthrough)
    }

    /// Fetchs changes from remote repositories.
    pub fn fetch(&self, filter: Option<&Vec<Filter>>, passthrough: &[String]) -> Result<()> {
        self.git_command("fetch", filter, passthrough)
    }

    pub fn config(&self) -> Result<()> {
        let editor = env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
        let config_path = "~/.config/multigit/config.toml";
        let config_path = shellexpand::tilde(config_path);
        let full_command = format!("{} {}", editor, config_path);
        let args = shell_words::split(&full_command)?;
        let (cmd, args) = args.split_first().ok_or("Empty command").unwrap();
        let status = Command::new(cmd).args(args).status()?;
        if !status.success() {
            return Err(anyhow!("Failed to execute command"));
        }
        Ok(())
    }
}

/// Enum representing possible filters for repositories.
#[derive(clap::ValueEnum, Clone, Debug, Serialize)]
pub enum Filter {
    /// Filter repositories that have uncommitted changes.
    Dirty,
}

/// Enum representing the state of repository entries.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum EntryState {
    /// Indicates that the repository has uncommitted changes.
    Dirty,
}

/// Represents the state of a repository.
pub struct RepositoryState {
    /// A set of entry states.
    pub entries: HashSet<EntryState>,
}

/// Opens the configured Git UI for a given repository path.
pub fn open_in_git_ui(path: &Path) -> Result<()> {
    let editor = "gitup";
    let status = std::process::Command::new(editor)
        .current_dir(path)
        .status()?;
    if !status.success() {
        return Err(anyhow!("Failed to open git ui"));
    }
    Ok(())
}

/// Finds all Git repositories within a given path.
pub fn find_repositories(path: &Path) -> Result<Vec<PathBuf>> {
    let mut repositories = Vec::new();
    let walker = WalkDir::new(path).into_iter().filter_entry(|e| {
        e.file_type().is_dir() && !is_hidden(e.path()) && e.path().file_name().unwrap() != ".git"
    });
    for entry in walker {
        let entry = entry?;
        if is_git_repository(entry.path()) {
            let path = entry.path();
            repositories.push(path.to_path_buf());
        }
    }
    Ok(repositories)
}

/// Checks if a path is a Git repository.
pub fn is_git_repository(path: &Path) -> bool {
    path.join(".git").exists()
}

/// Checks if a path is hidden (starts with a dot).
pub fn is_hidden(path: &Path) -> bool {
    path.file_name().unwrap().to_str().unwrap().starts_with('.')
}

/// Returns `None` if the vector is empty, otherwise returns `Some(&Vec<T>)`.
pub fn noneify<T>(v: &Vec<T>) -> Option<&Vec<T>> {
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

#[allow(dead_code)]
struct RepositoryError {
    path: PathBuf,
    error: anyhow::Error,
}

impl fmt::Display for EntryState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntryState::Dirty => write!(f, "Dirty"),
        }
    }
}

impl fmt::Display for RepositoryState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.entries.is_empty() {
            write!(f, "Clean")
        } else {
            let states: Vec<String> = self.entries.iter().map(|state| state.to_string()).collect();
            write!(f, "{}", states.join(", "))
        }
    }
}

fn display_option(o: &Option<bool>) -> String {
    match o {
        Some(s) => format!("{}", s),
        None => "".to_string(),
    }
}
