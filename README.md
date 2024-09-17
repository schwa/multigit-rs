# Multigit

Multigit is a powerful command-line interface (CLI) tool designed to manage multiple Git repositories simultaneously. It streamlines common Git operations across multiple projects, saving time and effort for developers working with multiple repositories.

**Note: This project is currently in progress and may not work properly. Use at your own risk.**

## Features

- Perform Git operations (status, add, commit, push, pull) on multiple repositories at once
- Execute custom commands across selected repositories
- List and manage registered repositories
- Filter repositories for specific operations
- Open the configured Git UI program for selected repositories
- Easy registration and unregistration of repositories

## Installation

Multigit is a Rust project that can be installed using Cargo, the Rust package manager. Follow these steps to install Multigit:

1. If you don't have Rust and Cargo installed, first install them by following the instructions at [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install).

2. Once Rust and Cargo are installed, you can install Multigit directly from the GitHub repository:

   ```sh
   cargo install --git https://github.com/schwa/multigit-rs
   ```

   This command will download the source code, compile it, and install the `multigit` binary in your Cargo bin directory (usually `~/.cargo/bin/`).

3. Ensure that your Cargo bin directory is in your system's PATH.

Alternatively, if you want to contribute or modify the code:

1. Clone the repository:

   ```sh
   git clone https://github.com/schwa/multigit-rs.git
   cd multigit-rs
   ```

2. Build and install from the local source:

   ```sh
   cargo install --path .
   ```

After installation, you can run `multigit --version` to verify that it's installed correctly.

## Repository Management

### Registering Repositories

To start using Multigit, you first need to register your Git repositories:

```sh
multigit register <PATH>
```

The `<PATH>` can be:

- A direct path to a Git repository
- A path to a directory containing one or more Git repositories

If you provide a directory path, Multigit will recursively search for all Git repositories within that directory and register them.

Example:
```sh
multigit register ~/projects/repo1 ~/projects/repo2 ~/all-projects
```

### Unregistering Repositories

To remove repositories from Multigit's management:

```sh
multigit unregister <PATH>
```

Similar to the register command, `<PATH>` can be a direct path to a Git repository or a directory. If a directory is provided, all registered repositories within that directory will be unregistered.

To unregister all repositories at once:

```sh
multigit unregister --all
```

### Listing Repositories

To confirm whether repositories are registered or to see all managed repositories:

```sh
multigit list
```

This command is useful to verify if a repository was successfully registered or unregistered.

## Common Git Operations

Multigit provides the following commands for managing your repositories:

```sh
multigit [COMMAND] [OPTIONS]
```

### Commands:

- `status`: Show the status of repositories
- `add`: Add files to the staging area in selected repositories
- `commit`: Commit changes in selected repositories
- `push`: Push changes to remote repositories
- `pull`: Pull changes from remote repositories
- `exec`: Execute a custom command in selected repositories
- `ui`: Open the configured Git UI program for selected repositories

### Options:

Most commands support the following option:

- `--filter <FILTER>`: Apply filters to select specific repositories

### Examples:

1. Check status of all repositories:
   ```sh
   multigit status
   ```

2. Check status of dirty repositories:
   ```sh
   multigit status --filter dirty
   ```

3. Commit changes in dirty repositories:
   ```sh
   multigit commit --filter dirty -m "Update documentation"
   ```

4. Pull changes in dirty repositories:
   ```sh
   multigit pull --filter dirty
   ```

5. Execute a custom command in dirty repositories:
   ```sh
   multigit exec --filter dirty -- git log --oneline -n 5
   ```

## Configuration

Multigit can be configured by editing the TOML file located at `~/.config/multigit/config.toml`.

[Add more details about configuration options and their effects]

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License.

## Author

Jonathan Wight

## Version

0.1

For more detailed information on each command and its options, use the `--help` flag:

```sh
multigit --help
multigit [COMMAND] --help
```
