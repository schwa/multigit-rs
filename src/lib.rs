use anyhow::{anyhow, Context, Ok, Result};
use colored_markup::{println_markup, StyleSheet};
use inquire::Confirm;
use path_absolutize::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::repeat;
use std::path::{Path, PathBuf};
use termsize;
use walkdir::WalkDir;

#[derive(Debug, Deserialize, Serialize)]
pub struct RepositoryEntry {
    pub path: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DirectoryEntry {
    pub path: PathBuf,
}

impl RepositoryEntry {
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
pub struct Config {
    #[serde(default = "HashMap::new")]
    pub repositories: HashMap<String, RepositoryEntry>,

    #[serde(default = "HashMap::new")]
    pub directories: HashMap<String, DirectoryEntry>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = "~/.config/multigit/config.toml";
        let config_path = shellexpand::tilde(config_path);
        let config_path = config_path.to_string();
        let config_content = std::fs::read_to_string(config_path)?;
        toml::from_str(&config_content).map_err(|e| anyhow!(e))
    }

    pub fn save(&self) -> Result<()> {
        let config_path = "~/.config/multigit/config.toml";
        let config_path = shellexpand::tilde(config_path);
        let config_path = config_path.to_string();
        let config_content = toml::to_string(&self)?;
        std::fs::write(config_path, config_content)?;
        Ok(())
    }

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
        Ok(())
    }

    pub fn unregister(&mut self, path: &PathBuf) -> Result<()> {
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
pub struct Multigit {
    pub config: Config,
    pub style_sheet: StyleSheet<'static>,
}

impl Multigit {
    pub fn new() -> Result<Self> {
        let config = Config::load()?;

        let style_sheet = StyleSheet::parse(
            "
        repository { foreground: cyan; }
        status { foreground: yellow; }
        command { foreground: green; }
        divider { foreground: red; }
        ",
        )
        .unwrap();

        Ok(Self {
            config,
            style_sheet,
        })
    }

    pub fn all_repositories(&self, filter: Option<&Vec<Filter>>) -> Result<Vec<RepositoryEntry>> {
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

    pub fn register(&mut self, paths: &Vec<PathBuf>) -> Result<()> {
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
                    return Ok(());
                }
            }
        } else {
            if paths.is_empty() {
                self.config.unregister(&std::env::current_dir()?)?;
            } else {
                for path in paths {
                    self.config.unregister(path)?;
                }
            }
        }
        self.config.save()?;
        Ok(())
    }

    pub fn list(&self, filter: Option<&Vec<Filter>>) -> Result<()> {
        for repository in self.all_repositories(filter)?.iter() {
            println_markup!(
                &self.style_sheet,
                "<repository>{}</repository>",
                repository.path.to_str().unwrap(),
            );
        }
        Ok(())
    }

    pub fn status(&self, filter: Option<&Vec<Filter>>) -> Result<()> {
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
                    "<repository>{}</repository><status>{}</status>",
                    repository.path.to_str().unwrap(),
                    status_string
                );
            }
        }
        Ok(())
    }

    pub fn ui(&self, filter: Option<&Vec<Filter>>) -> Result<()> {
        let paths_to_open = self.all_repositories(filter)?;
        if paths_to_open.len() > 1 {
            let ans = Confirm::new(format!("Open {} repositories?", paths_to_open.len()).as_str())
                .with_default(false)
                .prompt()?;
            match ans {
                true => {}
                false => {
                    return Ok(());
                }
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
        Ok(())
    }

    pub fn exec(&self, filter: Option<&Vec<Filter>>, commands: &Vec<String>) -> Result<()> {
        let repositories = self.all_repositories(filter)?;
        for repository in repositories.iter() {
            let mut command = std::process::Command::new(&commands[0]);
            command.args(&commands[1..]);
            command.current_dir(&repository.path);
            let status = command.status()?;
            if !status.success() {
                return Err(anyhow!("Failed to execute command"));
            }
        }
        Ok(())
    }

    pub fn git_command(
        &self,
        command: &str,
        filter: Option<&Vec<Filter>>,
        passthrough: &Vec<String>,
    ) -> Result<()> {
        let repositories = self.all_repositories(filter)?;

        let width = termsize::get().unwrap().cols as usize;

        let divider = std::iter::repeat("#").take(width).collect::<String>();

        for (index, repository) in repositories.iter().enumerate() {
            if index != 0 {
                println_markup!(&self.style_sheet, "\n<divider>{}</divider>\n", divider);
            }
            println_markup!(
                &self.style_sheet,
                "Running `<command>{}</command>` in <repository>{}</repository>\n",
                command,
                repository.path.to_str().unwrap()
            );
            let mut args = vec![command];
            args.extend(passthrough.iter().map(|s| s.as_str()));
            let mut command = std::process::Command::new("git");
            command.args(&args);
            command.current_dir(&repository.path);
            _ = command.status()?; // TODO: Fix status checking
        }
        Ok(())
    }

    pub fn commit(&self, filter: Option<&Vec<Filter>>, passthrough: &Vec<String>) -> Result<()> {
        self.git_command("commit", filter, passthrough)
    }

    pub fn add(&self, filter: Option<&Vec<Filter>>, passthrough: &Vec<String>) -> Result<()> {
        self.git_command("add", filter, passthrough)
    }

    pub fn push(&self, filter: Option<&Vec<Filter>>, passthrough: &Vec<String>) -> Result<()> {
        self.git_command("push", filter, passthrough)
    }

    pub fn pull(&self, filter: Option<&Vec<Filter>>, passthrough: &Vec<String>) -> Result<()> {
        self.git_command("pull", filter, passthrough)
    }
}

#[derive(clap::ValueEnum, Clone, Debug, Serialize)]
pub enum Filter {
    Dirty,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum EntryState {
    Dirty,
}

pub struct RepositoryState {
    pub entries: HashSet<EntryState>,
}

pub fn open_in_git_ui(path: &Path) -> Result<()> {
    let editor = "gitup";
    let status = std::process::Command::new(editor).arg(path).status()?;
    if !status.success() {
        return Err(anyhow!("Failed to open git ui"));
    }
    Ok(())
}

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
            // walker.skip_current_dir();
        }
    }
    Ok(repositories)
}

pub fn is_git_repository(path: &Path) -> bool {
    path.join(".git").exists()
}

pub fn is_hidden(path: &Path) -> bool {
    path.file_name().unwrap().to_str().unwrap().starts_with(".")
}

pub fn noneify<T>(v: &Vec<T>) -> Option<&Vec<T>> {
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}
