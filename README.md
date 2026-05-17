**Languages:** [English](README.md) | [简体中文](README.zh-CN.md)

# Cnblogs' command line tool
 
[![Build / Release](https://github.com/cnblogs/cli/actions/workflows/build-release.yml/badge.svg)](https://github.com/cnblogs/cli/actions/workflows/build-release.yml)
[![Build / Development](https://github.com/cnblogs/cli/actions/workflows/build-dev.yml/badge.svg)](https://github.com/cnblogs/cli/actions/workflows/build-dev.yml)

Access and manage your cnblogs content directly form the command line.

## Features

- 📝 Manage Content: Create, view, and interact with posts, moments, and comments
- 🔐 Secure Authentication: Personal Access Token (PAT) based authentication
- ⚡ Fast & Lightweight: Built with Rust for optimal performance
- 🔧 Cross-Platform: Available for Windows, macOS, and Linux
- 📚 Intuitive Interface: Familiar CLI patterns and comprehensive help system

## Installation

### Download Pre-built Binaries (Recommended)

Download the latest release for your platform from the [Releases page](https://github.com/cnblogs/cli/releases).

#### Quick Install (macOS/Linux)

```sh
# Download and install cnb
curl -fSL -O https://github.com/cnblogs/cli/releases/download/v$version/cnb-$version-$arch-$os.zip
unzip -d . cnb-$version-$arch-$os.zip
mv ./cnb ~/.local/bin/
```

### Build from Source

Versions prior to 0.2.1 (inclusive) require a Rust nightly toolchain(channel=2026-01-10+), and versions after 0.2.1 switch to the stable toolchain (1.92+)

If you have a Rust compilation environment locally, you can install or build it using Cargo.

Cargo Install

```sh
# from repo main
cargo install --git https://github.com/cnblogs/cli.git

# Or local install
# Clone repository
git clone --depth 1 https://github.com/cnblogs/cli.git

cargo install --path ./cli --bin cnb
```

Build from Source

```sh
# Clone repository
git clone --depth 1 https://github.com/cnblogs/cli.git
cd cli

# Build release version
cargo build --release --bin cnb

# The binary will be available at ./target/release/cnb
```

## Quick Start

### 1. Get Your Personal Access Token

1. Visit [https://account.cnblogs.com/settings/tokens](https://account.cnblogs.com/settings/tokens)
2. Click "Generate New Token"
3. Copy the generated token (you won't be able to see it again)

### 2. Login

```sh
# login (recommended)
cnb user login YOUR_PAT_TOKEN

# Verify login status
cnb user status
```

This will save your PAT to `~/.cnblogs/token`

If you want to log out, run `cnb user logout` or just remove `~/.cnblogs`.

## Command Reference

### Command Usage

```sh
cnb <command> <subcommand> [option] [arg]
```

### Available Commands

| Command | Description      | Available Subcommands                       |
|---------|------------------|---------------------------------------------|
| `user`  | User module      | `login`, `logout`, `status`                 |
| `ing`   | Moments module   | `create`, `delete`, `list`, `show`, `reply` |
| `post`  | posts module     | `list`, `show`, `reply`                     |
| `news`  | news module      | `list`                                      |
| `fav`   | bookmarks module | `list`                                      |

### Usage Examples

It's time to enjoy cnblogs.

Here are some simple examples:

```sh
# Check your post list
cnb post list
# Check your post
cnb post show 114514

# Show ing list
cnb ing list
cnb ing list my --page-index 1 --page-size 10

# Publish ing
cnb ing create 'Hello world!'
cnb ing create 'Hello world!' --tag lucky

# Comment to ing
cnb  ing replay 'Awesome!' --id 114514
```

For more information, try `cnb --help` or `cnb help`.

## Project Structure

```text
cli/
├── Cargo.lock
├── Cargo.toml
├── LICENSE
├── README.md
├── README.zh-CN.md
├── rust-fmt.toml
├── rust-toolchain.toml
├── shell.nix
└── src
    ├── api                       # CNBlogs API interaction module
    │   ├── fav.rs                # Favorites/bookmarks API endpoints
    │   ├── ing.rs                # Moments/statuses API endpoints
    │   ├── mod.rs                # Module exports for API layer
    │   ├── news.rs               # News API endpoints
    │   ├── post.rs               # Blog posts API endpoints
    │   ├── urls.rs               # URL generator for API request construction
    │   └── user.rs               # User API endpoints (login, profile, etc.)
    ├── bin                       # Binary entry point
    │   └── cnb.rs                # CLI main executable (argument parsing and routing)
    ├── commands                  # CLI command implementations
    │   ├── fav.rs
    │   ├── ing.rs
    │   ├── mod.rs
    │   ├── news.rs
    │   ├── post.rs
    │   └── user.rs
    ├── context                   # Context management (configuration, state, output)
    │   ├── config.rs             # Configuration file reading/writing and management
    │   ├── mod.rs                # Context module exports
    │   └── output.rs             # Output formatting control (JSON, table, text, etc.)
    ├── display                   # Data display and formatting module
    │   ├── ing.rs                # Moment data display formatting
    │   └── mod.rs                # Display module exports
    ├── lib.rs                    # Library crate root, exports public interfaces
    ├── logic                     # Business logic layer (orchestrates operations)
    │   ├── fav.rs
    │   ├── ing.rs
    │   ├── mod.rs
    │   ├── news.rs
    │   ├── post.rs
    │   └── user.rs
    ├── models                    # Data model definitions and formats
    │   ├── fav.rs
    │   ├── ing.rs
    │   ├── mod.rs
    │   ├── news.rs
    │   ├── post.rs
    │   └── user.rs
    └── tools                     # Utility functions and extensions
        ├── http.rs               # Extensions to reqwest.
        ├── mod.rs
        ├── strings.rs            # Extensions to String.
        └── timer.rs              # Extensions to chrono.
```

## License

[MIT](https://raw.githubusercontent.com/cnblogs/cli/main/LICENSE)

## Feedback

We’d love to hear your thoughts on this project. Feel free to drop us a note!

[Issues](https://github.com/cnblogs/cli/issues)
