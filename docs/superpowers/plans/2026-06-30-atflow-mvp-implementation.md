# Atflow MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first Linux/WSL Atflow MVP: an `at` Rust binary with `@` shell integration, command-palette TUI, config wizard, SQLite history, recent/flow/search flows, and installer script.

**Architecture:** Use one Rust crate named `at` with focused modules for CLI, config, history, shell integration, search, flow, open actions, and UI. Keep core behavior testable without a terminal by separating state/formatting logic from crossterm/ratatui runtime code.

**Tech Stack:** Rust 1.96, clap, anyhow, serde, toml, dirs, rusqlite with bundled SQLite, ignore, crossterm, ratatui, tempfile, assert_cmd.

---

## File Structure

- Create: `Cargo.toml` for package metadata, dependencies, and dev-dependencies.
- Create: `src/main.rs` for top-level error handling and CLI dispatch.
- Create: `src/lib.rs` for module exports used by tests.
- Create: `src/cli.rs` for clap command definitions.
- Create: `src/config.rs` for config defaults and TOML read/write.
- Create: `src/history.rs` for SQLite schema, path recording, and recent queries.
- Create: `src/path_display.rs` for middle-ellipsis path clipping and expansion display text.
- Create: `src/shell.rs` for shell quoting, shell functions, hooks, and shell command output.
- Create: `src/open.rs` for open action resolution.
- Create: `src/search.rs` for scoped filesystem search.
- Create: `src/flow.rs` for directory browsing state and Git root detection.
- Create: `src/init.rs` for first-run configuration wizard logic.
- Create: `src/ui/mod.rs` for UI module exports.
- Create: `src/ui/theme.rs` for Mist, Ink, and Paper theme definitions.
- Create: `src/ui/palette.rs` for command-palette state, selection, filtering, and expansion.
- Create: `src/ui/runtime.rs` for crossterm/ratatui event loop.
- Create: `tests/cli_smoke.rs` for CLI integration smoke tests.
- Create: `scripts/install.sh` for Linux/WSL installer entrypoint.
- Create: `README.md` for user-facing installation and MVP usage.
- Modify: `.gitignore` to keep build/test artifacts out of git if new generated paths appear.

## Task 1: Rust Crate Skeleton And CLI Routing

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Create: `src/cli.rs`
- Create: `tests/cli_smoke.rs`

- [ ] **Step 1: Initialize the Rust binary crate**

Run:

```bash
cargo init --bin --name at .
```

Expected: `Cargo.toml` and `src/main.rs` are created.

- [ ] **Step 2: Add dependencies**

Run:

```bash
cargo add anyhow
cargo add clap --features derive
cargo add serde --features derive
cargo add toml
cargo add dirs
cargo add rusqlite --features bundled
cargo add ignore
cargo add crossterm
cargo add ratatui
cargo add --dev assert_cmd
cargo add --dev tempfile
```

Expected: `Cargo.toml` contains runtime dependencies and dev-dependencies, and `Cargo.lock` is created.

- [ ] **Step 3: Create CLI definitions**

Write `src/cli.rs`:

```rust
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "at", version, about = "A lightweight @ command palette for Linux terminals")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    Menu {
        #[arg(long)]
        shell: bool,
    },
    Recent {
        #[arg(long)]
        shell: bool,
    },
    Flow {
        #[arg(long)]
        shell: bool,
    },
    Search {
        #[arg(long)]
        shell: bool,
        #[arg(value_name = "QUERY")]
        query: Vec<String>,
    },
    Setting,
    Init,
    Shell {
        #[command(subcommand)]
        command: ShellCommand,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ShellCommand {
    Print,
    Hook,
}

impl Command {
    pub fn search_query(query: &[String]) -> Option<String> {
        let joined = query.join(" ");
        let trimmed = joined.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    }
}
```

- [ ] **Step 4: Create library exports**

Write `src/lib.rs`:

```rust
pub mod cli;
```

- [ ] **Step 5: Wire a minimal main**

Write `src/main.rs`:

```rust
use anyhow::Result;
use at::cli::{Cli, Command, ShellCommand};
use clap::Parser;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Menu { shell: false }) {
        Command::Menu { shell } => println!("menu shell={shell}"),
        Command::Recent { shell } => println!("recent shell={shell}"),
        Command::Flow { shell } => println!("flow shell={shell}"),
        Command::Search { shell, query } => {
            let query = Command::search_query(&query).unwrap_or_default();
            println!("search shell={shell} query={query}");
        }
        Command::Setting => println!("setting"),
        Command::Init => println!("init"),
        Command::Shell { command } => match command {
            ShellCommand::Print => println!("shell print"),
            ShellCommand::Hook => println!("shell hook"),
        },
    }
    Ok(())
}
```

- [ ] **Step 6: Add CLI smoke tests**

Write `tests/cli_smoke.rs`:

```rust
use assert_cmd::Command;

#[test]
fn help_mentions_core_commands() {
    Command::cargo_bin("at")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("menu"))
        .stdout(predicates::str::contains("recent"))
        .stdout(predicates::str::contains("search"));
}

#[test]
fn search_accepts_optional_query() {
    Command::cargo_bin("at")
        .unwrap()
        .args(["search", "--shell", "nightlight"])
        .assert()
        .success()
        .stdout(predicates::str::contains("query=nightlight"));
}
```

- [ ] **Step 7: Add missing dev dependency for predicates**

Run:

```bash
cargo add --dev predicates
```

Expected: `predicates` is added because `tests/cli_smoke.rs` uses `predicates::str::contains`.

- [ ] **Step 8: Verify tests pass**

Run:

```bash
cargo test
```

Expected: all CLI smoke tests pass.

- [ ] **Step 9: Commit Task 1**

Run:

```bash
git add Cargo.toml Cargo.lock src/main.rs src/lib.rs src/cli.rs tests/cli_smoke.rs
git commit -m "feat: scaffold Rust CLI"
```

## Task 2: Path Display And Theme Names

**Files:**
- Create: `src/path_display.rs`
- Create: `src/ui/mod.rs`
- Create: `src/ui/theme.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write path clipping tests**

Write `src/path_display.rs` with tests first:

```rust
use std::path::Path;

pub fn clip_middle(input: &str, max_width: usize) -> String {
    crate::path_display::implementation::clip_middle(input, max_width)
}

pub fn display_path(path: &Path, expanded: bool, max_width: usize) -> String {
    crate::path_display::implementation::display_path(path, expanded, max_width)
}

mod implementation {
    use std::path::Path;

    pub fn clip_middle(input: &str, max_width: usize) -> String {
        if input.chars().count() <= max_width {
            return input.to_owned();
        }
        if max_width <= 1 {
            return ".".to_owned();
        }
        if max_width <= 3 {
            return ".".repeat(max_width);
        }
        let ellipsis = "...";
        let available = max_width - ellipsis.len();
        let left = available / 2;
        let right = available - left;
        let prefix: String = input.chars().take(left).collect();
        let suffix: String = input
            .chars()
            .rev()
            .take(right)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("{prefix}{ellipsis}{suffix}")
    }

    pub fn display_path(path: &Path, expanded: bool, max_width: usize) -> String {
        let text = path.display().to_string();
        if expanded {
            text
        } else {
            clip_middle(&text, max_width)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn leaves_short_text_unchanged() {
        assert_eq!(clip_middle("src/main.rs", 30), "src/main.rs");
    }

    #[test]
    fn clips_long_text_in_the_middle() {
        assert_eq!(
            clip_middle("/home/congke/work/ntl-imputation/data/nightlight", 28),
            "/home/congke...ta/nightlight"
        );
    }

    #[test]
    fn expanded_display_returns_full_path() {
        let path = PathBuf::from("/home/congke/work/ntl-imputation/data/nightlight");
        assert_eq!(
            display_path(&path, true, 20),
            "/home/congke/work/ntl-imputation/data/nightlight"
        );
    }

    #[test]
    fn collapsed_display_uses_width_limit() {
        let path = PathBuf::from("/home/congke/work/ntl-imputation/data/nightlight");
        assert_eq!(display_path(&path, false, 20), "/home/co...ightlight");
    }
}
```

- [ ] **Step 2: Export modules**

Modify `src/lib.rs`:

```rust
pub mod cli;
pub mod path_display;
pub mod ui;
```

- [ ] **Step 3: Add UI module and theme definitions**

Write `src/ui/mod.rs`:

```rust
pub mod theme;
```

Write `src/ui/theme.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeName {
    Mist,
    Ink,
    Paper,
}

impl Default for ThemeName {
    fn default() -> Self {
        Self::Mist
    }
}

impl ThemeName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mist => "mist",
            Self::Ink => "ink",
            Self::Paper => "paper",
        }
    }
}
```

- [ ] **Step 4: Run focused tests**

Run:

```bash
cargo test path_display
```

Expected: all path display tests pass.

- [ ] **Step 5: Commit Task 2**

Run:

```bash
git add src/lib.rs src/path_display.rs src/ui/mod.rs src/ui/theme.rs
git commit -m "feat: add path display helpers"
```

## Task 3: Configuration Defaults And TOML Persistence

**Files:**
- Create: `src/config.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write config implementation and tests**

Write `src/config.rs`:

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ui::theme::ThemeName;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub open: OpenConfig,
    pub search: SearchConfig,
    pub history: HistoryConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub theme: ThemeName,
    pub max_recent: usize,
    pub start_from_git_root: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenConfig {
    pub editor: String,
    pub gui_editor: String,
    pub file_opener: String,
    pub prefer_terminal_editor: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchConfig {
    pub roots: Vec<String>,
    pub ignore: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryConfig {
    pub record_atflow_opens: bool,
    pub record_shell_cd: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                theme: ThemeName::Mist,
                max_recent: 100,
                start_from_git_root: true,
            },
            open: OpenConfig {
                editor: std::env::var("EDITOR").unwrap_or_else(|_| "nvim".to_owned()),
                gui_editor: "code".to_owned(),
                file_opener: "xdg-open".to_owned(),
                prefer_terminal_editor: true,
            },
            search: SearchConfig {
                roots: vec!["~/work".to_owned(), "~/code".to_owned(), "~/Documents".to_owned()],
                ignore: vec![
                    ".git".to_owned(),
                    "node_modules".to_owned(),
                    "__pycache__".to_owned(),
                    ".venv".to_owned(),
                    "target".to_owned(),
                    "dist".to_owned(),
                ],
            },
            history: HistoryConfig {
                record_atflow_opens: true,
                record_shell_cd: false,
            },
        }
    }
}

impl Config {
    pub fn load_or_default(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path)
            .with_context(|| format!("failed to read config {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("failed to parse config {}", path.display()))
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create config dir {}", parent.display()))?;
        }
        let text = toml::to_string_pretty(self).context("failed to serialize config")?;
        fs::write(path, text).with_context(|| format!("failed to write config {}", path.display()))
    }
}

pub fn default_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("at")
        .join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_matches_mvp_defaults() {
        let config = Config::default();
        assert_eq!(config.general.theme, ThemeName::Mist);
        assert_eq!(config.general.max_recent, 100);
        assert!(config.general.start_from_git_root);
        assert_eq!(config.open.file_opener, "xdg-open");
        assert!(config.search.ignore.contains(&".git".to_owned()));
        assert!(config.history.record_atflow_opens);
        assert!(!config.history.record_shell_cd);
    }

    #[test]
    fn load_missing_config_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        assert_eq!(Config::load_or_default(&path).unwrap(), Config::default());
    }

    #[test]
    fn saves_and_loads_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("config.toml");
        let mut config = Config::default();
        config.general.theme = ThemeName::Paper;
        config.history.record_shell_cd = true;

        config.save_to(&path).unwrap();
        let loaded = Config::load_or_default(&path).unwrap();

        assert_eq!(loaded.general.theme, ThemeName::Paper);
        assert!(loaded.history.record_shell_cd);
    }
}
```

- [ ] **Step 2: Export config module**

Modify `src/lib.rs`:

```rust
pub mod cli;
pub mod config;
pub mod path_display;
pub mod ui;
```

- [ ] **Step 3: Run focused tests**

Run:

```bash
cargo test config
```

Expected: all config tests pass.

- [ ] **Step 4: Commit Task 3**

Run:

```bash
git add src/lib.rs src/config.rs
git commit -m "feat: add config persistence"
```

## Task 4: Shell Integration

**Files:**
- Create: `src/shell.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`
- Modify: `tests/cli_smoke.rs`

- [ ] **Step 1: Write shell integration helpers and tests**

Write `src/shell.rs`:

```rust
use std::path::Path;

pub fn shell_quote(input: &str) -> String {
    if input.is_empty() {
        return "''".to_owned();
    }
    format!("'{}'", input.replace('\'', "'\\''"))
}

pub fn cd_command(path: &Path) -> String {
    format!("cd {}", shell_quote(&path.display().to_string()))
}

pub fn functions_block() -> &'static str {
    r#"@()        { eval "$(at menu --shell "$@")"; }
@recent()  { eval "$(at recent --shell "$@")"; }
@flow()    { eval "$(at flow --shell "$@")"; }
@search()  { eval "$(at search --shell "$@")"; }
@setting() { at setting "$@"; }"#
}

pub fn cd_hook_block() -> &'static str {
    r#"_atflow_record_cd() {
  command at recent-record "$PWD" >/dev/null 2>&1 || true
}

if [ -n "${ZSH_VERSION:-}" ]; then
  autoload -Uz add-zsh-hook
  add-zsh-hook chpwd _atflow_record_cd
elif [ -n "${BASH_VERSION:-}" ]; then
  _atflow_original_cd() {
    builtin cd "$@" && _atflow_record_cd
  }
  alias cd='_atflow_original_cd'
fi"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn quotes_paths_for_shell_eval() {
        assert_eq!(shell_quote("/home/a b/project"), "'/home/a b/project'");
        assert_eq!(shell_quote("/home/it's/project"), "'/home/it'\\''s/project'");
    }

    #[test]
    fn cd_command_wraps_quoted_path() {
        assert_eq!(
            cd_command(&PathBuf::from("/home/congke/work/at flow")),
            "cd '/home/congke/work/at flow'"
        );
    }

    #[test]
    fn functions_include_user_facing_entries() {
        let block = functions_block();
        assert!(block.contains("@()"));
        assert!(block.contains("@recent()"));
        assert!(block.contains("@flow()"));
        assert!(block.contains("@search()"));
    }
}
```

- [ ] **Step 2: Export shell module**

Modify `src/lib.rs`:

```rust
pub mod cli;
pub mod config;
pub mod path_display;
pub mod shell;
pub mod ui;
```

- [ ] **Step 3: Wire `at shell print` and `at shell hook`**

Modify `src/main.rs` shell branch:

```rust
use anyhow::Result;
use at::cli::{Cli, Command, ShellCommand};
use clap::Parser;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Menu { shell: false }) {
        Command::Menu { shell } => println!("menu shell={shell}"),
        Command::Recent { shell } => println!("recent shell={shell}"),
        Command::Flow { shell } => println!("flow shell={shell}"),
        Command::Search { shell, query } => {
            let query = Command::search_query(&query).unwrap_or_default();
            println!("search shell={shell} query={query}");
        }
        Command::Setting => println!("setting"),
        Command::Init => println!("init"),
        Command::Shell { command } => match command {
            ShellCommand::Print => println!("{}", at::shell::functions_block()),
            ShellCommand::Hook => println!("{}", at::shell::cd_hook_block()),
        },
    }
    Ok(())
}
```

- [ ] **Step 4: Add integration smoke test for shell print**

Append to `tests/cli_smoke.rs`:

```rust
#[test]
fn shell_print_outputs_functions() {
    Command::cargo_bin("at")
        .unwrap()
        .args(["shell", "print"])
        .assert()
        .success()
        .stdout(predicates::str::contains("@()"))
        .stdout(predicates::str::contains("@search()"));
}
```

- [ ] **Step 5: Run tests**

Run:

```bash
cargo test shell
cargo test --test cli_smoke
```

Expected: shell unit tests and CLI smoke tests pass.

- [ ] **Step 6: Commit Task 4**

Run:

```bash
git add src/lib.rs src/main.rs src/shell.rs tests/cli_smoke.rs
git commit -m "feat: add shell integration output"
```

## Task 5: SQLite History

**Files:**
- Create: `src/history.rs`
- Modify: `src/lib.rs`
- Modify: `src/cli.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Extend CLI for cd hook recording**

Modify `src/cli.rs`:

```rust
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "at", version, about = "A lightweight @ command palette for Linux terminals")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    Menu {
        #[arg(long)]
        shell: bool,
    },
    Recent {
        #[arg(long)]
        shell: bool,
    },
    Flow {
        #[arg(long)]
        shell: bool,
    },
    Search {
        #[arg(long)]
        shell: bool,
        #[arg(value_name = "QUERY")]
        query: Vec<String>,
    },
    Setting,
    Init,
    RecentRecord {
        path: String,
    },
    Shell {
        #[command(subcommand)]
        command: ShellCommand,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ShellCommand {
    Print,
    Hook,
}

impl Command {
    pub fn search_query(query: &[String]) -> Option<String> {
        let joined = query.join(" ");
        let trimmed = joined.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    }
}
```

- [ ] **Step 2: Write SQLite history implementation and tests**

Write `src/history.rs`:

```rust
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathKind {
    Dir,
    File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistorySource {
    Atflow,
    ShellCdHook,
    ManualRootScan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub path: PathBuf,
    pub kind: PathKind,
    pub source: HistorySource,
    pub last_opened_at: i64,
    pub open_count: i64,
}

pub struct HistoryDb {
    conn: Connection,
}

impl PathKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dir => "dir",
            Self::File => "file",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "file" => Self::File,
            _ => Self::Dir,
        }
    }
}

impl HistorySource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Atflow => "atflow",
            Self::ShellCdHook => "shell_cd_hook",
            Self::ManualRootScan => "manual_root_scan",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "shell_cd_hook" => Self::ShellCdHook,
            "manual_root_scan" => Self::ManualRootScan,
            _ => Self::Atflow,
        }
    }
}

impl HistoryDb {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create history dir {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("failed to open history database {}", path.display()))?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn open_memory() -> Result<Self> {
        let db = Self {
            conn: Connection::open_in_memory().context("failed to open in-memory database")?,
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS paths (
              id INTEGER PRIMARY KEY,
              path TEXT NOT NULL UNIQUE,
              kind TEXT NOT NULL,
              source TEXT NOT NULL,
              last_opened_at INTEGER NOT NULL,
              open_count INTEGER NOT NULL DEFAULT 1
            );

            CREATE INDEX IF NOT EXISTS idx_paths_last_opened ON paths(last_opened_at DESC);
            CREATE INDEX IF NOT EXISTS idx_paths_kind ON paths(kind);
            "#,
        )?;
        Ok(())
    }

    pub fn record_path_at(
        &self,
        path: &Path,
        kind: PathKind,
        source: HistorySource,
        timestamp: i64,
    ) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO paths (path, kind, source, last_opened_at, open_count)
            VALUES (?1, ?2, ?3, ?4, 1)
            ON CONFLICT(path) DO UPDATE SET
              kind = excluded.kind,
              source = excluded.source,
              last_opened_at = excluded.last_opened_at,
              open_count = paths.open_count + 1
            "#,
            params![path.display().to_string(), kind.as_str(), source.as_str(), timestamp],
        )?;
        Ok(())
    }

    pub fn recent_dirs(&self, limit: usize) -> Result<Vec<HistoryEntry>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT path, kind, source, last_opened_at, open_count
            FROM paths
            WHERE kind = 'dir'
            ORDER BY last_opened_at DESC, open_count DESC
            LIMIT ?1
            "#,
        )?;
        let rows = stmt.query_map([limit as i64], |row| {
            Ok(HistoryEntry {
                path: PathBuf::from(row.get::<_, String>(0)?),
                kind: PathKind::from_str(&row.get::<_, String>(1)?),
                source: HistorySource::from_str(&row.get::<_, String>(2)?),
                last_opened_at: row.get(3)?,
                open_count: row.get(4)?,
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>().context("failed to load recent dirs")
    }
}

pub fn default_history_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("at")
        .join("history.sqlite")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_sorts_recent_directories() {
        let db = HistoryDb::open_memory().unwrap();
        db.record_path_at(Path::new("/tmp/old"), PathKind::Dir, HistorySource::Atflow, 100).unwrap();
        db.record_path_at(Path::new("/tmp/new"), PathKind::Dir, HistorySource::ShellCdHook, 200).unwrap();
        db.record_path_at(Path::new("/tmp/file.rs"), PathKind::File, HistorySource::Atflow, 300).unwrap();

        let recent = db.recent_dirs(10).unwrap();

        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].path, PathBuf::from("/tmp/new"));
        assert_eq!(recent[0].source, HistorySource::ShellCdHook);
        assert_eq!(recent[1].path, PathBuf::from("/tmp/old"));
    }

    #[test]
    fn updates_existing_path_count_and_time() {
        let db = HistoryDb::open_memory().unwrap();
        db.record_path_at(Path::new("/tmp/project"), PathKind::Dir, HistorySource::Atflow, 100).unwrap();
        db.record_path_at(Path::new("/tmp/project"), PathKind::Dir, HistorySource::ShellCdHook, 300).unwrap();

        let recent = db.recent_dirs(10).unwrap();

        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].last_opened_at, 300);
        assert_eq!(recent[0].open_count, 2);
        assert_eq!(recent[0].source, HistorySource::ShellCdHook);
    }
}
```

- [ ] **Step 3: Export history module**

Modify `src/lib.rs`:

```rust
pub mod cli;
pub mod config;
pub mod history;
pub mod path_display;
pub mod shell;
pub mod ui;
```

- [ ] **Step 4: Wire `recent-record`**

Modify `src/main.rs` match arm:

```rust
        Command::RecentRecord { path } => {
            let db = at::history::HistoryDb::open(&at::history::default_history_path())?;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs() as i64;
            db.record_path_at(
                std::path::Path::new(&path),
                at::history::PathKind::Dir,
                at::history::HistorySource::ShellCdHook,
                now,
            )?;
        }
```

- [ ] **Step 5: Run focused tests**

Run:

```bash
cargo test history
```

Expected: SQLite history tests pass.

- [ ] **Step 6: Commit Task 5**

Run:

```bash
git add src/lib.rs src/cli.rs src/main.rs src/history.rs
git commit -m "feat: add SQLite history"
```

## Task 6: Open Action Resolution

**Files:**
- Create: `src/open.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write open action resolver and tests**

Write `src/open.rs`:

```rust
use std::path::{Path, PathBuf};

use crate::config::Config;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenAction {
    Cd(PathBuf),
    Editor { command: String, path: PathBuf },
    System { command: String, path: PathBuf },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenMode {
    Default,
    Editor,
    System,
}

pub fn resolve_open_action(path: &Path, is_dir: bool, mode: OpenMode, config: &Config) -> OpenAction {
    if is_dir {
        return OpenAction::Cd(path.to_path_buf());
    }

    match mode {
        OpenMode::Editor => OpenAction::Editor {
            command: config.open.editor.clone(),
            path: path.to_path_buf(),
        },
        OpenMode::System => OpenAction::System {
            command: config.open.file_opener.clone(),
            path: path.to_path_buf(),
        },
        OpenMode::Default => {
            if is_text_or_code(path) {
                OpenAction::Editor {
                    command: config.open.editor.clone(),
                    path: path.to_path_buf(),
                }
            } else {
                OpenAction::System {
                    command: config.open.file_opener.clone(),
                    path: path.to_path_buf(),
                }
            }
        }
    }
}

pub fn is_text_or_code(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()).unwrap_or_default(),
        "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "md" | "txt" | "toml" | "json" | "yaml" | "yml" | "sh" | "zsh" | "bash" | "html" | "css"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directories_resolve_to_cd() {
        let config = Config::default();
        assert_eq!(
            resolve_open_action(Path::new("/tmp/project"), true, OpenMode::Default, &config),
            OpenAction::Cd(PathBuf::from("/tmp/project"))
        );
    }

    #[test]
    fn rust_files_use_editor_by_default() {
        let config = Config::default();
        assert_eq!(
            resolve_open_action(Path::new("/tmp/main.rs"), false, OpenMode::Default, &config),
            OpenAction::Editor {
                command: config.open.editor.clone(),
                path: PathBuf::from("/tmp/main.rs")
            }
        );
    }

    #[test]
    fn images_use_system_opener_by_default() {
        let config = Config::default();
        assert_eq!(
            resolve_open_action(Path::new("/tmp/map.png"), false, OpenMode::Default, &config),
            OpenAction::System {
                command: "xdg-open".to_owned(),
                path: PathBuf::from("/tmp/map.png")
            }
        );
    }
}
```

- [ ] **Step 2: Export open module**

Modify `src/lib.rs`:

```rust
pub mod cli;
pub mod config;
pub mod history;
pub mod open;
pub mod path_display;
pub mod shell;
pub mod ui;
```

- [ ] **Step 3: Run focused tests**

Run:

```bash
cargo test open
```

Expected: open action tests pass.

- [ ] **Step 4: Commit Task 6**

Run:

```bash
git add src/lib.rs src/open.rs
git commit -m "feat: add open action resolver"
```

## Task 7: Scoped Search

**Files:**
- Create: `src/search.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write search implementation and tests**

Write `src/search.rs`:

```rust
use anyhow::Result;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchFilter {
    All,
    Dirs,
    Files,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub path: PathBuf,
    pub is_dir: bool,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct SearchRequest {
    pub roots: Vec<PathBuf>,
    pub query: Option<String>,
    pub filter: SearchFilter,
    pub ignore_names: Vec<String>,
    pub limit: usize,
}

pub fn search(request: &SearchRequest) -> Result<Vec<SearchResult>> {
    let query = request.query.as_deref().unwrap_or_default().to_lowercase();
    let mut results = Vec::new();

    for root in &request.roots {
        if !root.exists() {
            continue;
        }
        let mut builder = WalkBuilder::new(root);
        builder.hidden(false).git_ignore(true).git_exclude(true);

        for entry in builder.build().filter_map(Result::ok) {
            let path = entry.path();
            if path == root {
                continue;
            }
            if ignored_by_name(path, &request.ignore_names) {
                continue;
            }

            let file_type = match entry.file_type() {
                Some(file_type) => file_type,
                None => continue,
            };
            let is_dir = file_type.is_dir();
            if request.filter == SearchFilter::Dirs && !is_dir {
                continue;
            }
            if request.filter == SearchFilter::Files && is_dir {
                continue;
            }

            let text = path.display().to_string().to_lowercase();
            if !query.is_empty() && !text.contains(&query) {
                continue;
            }

            results.push(SearchResult {
                path: path.to_path_buf(),
                is_dir,
                source: root.display().to_string(),
            });

            if results.len() >= request.limit {
                return Ok(results);
            }
        }
    }

    Ok(results)
}

fn ignored_by_name(path: &Path, ignore_names: &[String]) -> bool {
    path.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        ignore_names.iter().any(|ignored| ignored == &name)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn searches_files_and_dirs_with_query() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("data/nightlight")).unwrap();
        fs::write(dir.path().join("data/nightlight_loader.rs"), "").unwrap();
        fs::write(dir.path().join("notes.txt"), "").unwrap();

        let results = search(&SearchRequest {
            roots: vec![dir.path().to_path_buf()],
            query: Some("nightlight".to_owned()),
            filter: SearchFilter::All,
            ignore_names: vec![],
            limit: 10,
        })
        .unwrap();

        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|result| result.path.ends_with("nightlight")));
        assert!(results.iter().any(|result| result.path.ends_with("nightlight_loader.rs")));
    }

    #[test]
    fn filters_dirs_only() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/main.rs"), "").unwrap();

        let results = search(&SearchRequest {
            roots: vec![dir.path().to_path_buf()],
            query: Some("src".to_owned()),
            filter: SearchFilter::Dirs,
            ignore_names: vec![],
            limit: 10,
        })
        .unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].is_dir);
    }

    #[test]
    fn skips_ignored_names() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("target/debug")).unwrap();
        fs::write(dir.path().join("target/debug/app"), "").unwrap();

        let results = search(&SearchRequest {
            roots: vec![dir.path().to_path_buf()],
            query: Some("app".to_owned()),
            filter: SearchFilter::All,
            ignore_names: vec!["target".to_owned()],
            limit: 10,
        })
        .unwrap();

        assert!(results.is_empty());
    }
}
```

- [ ] **Step 2: Export search module**

Modify `src/lib.rs`:

```rust
pub mod cli;
pub mod config;
pub mod history;
pub mod open;
pub mod path_display;
pub mod search;
pub mod shell;
pub mod ui;
```

- [ ] **Step 3: Run focused tests**

Run:

```bash
cargo test search
```

Expected: search tests pass.

- [ ] **Step 4: Commit Task 7**

Run:

```bash
git add src/lib.rs src/search.rs
git commit -m "feat: add scoped search"
```

## Task 8: Flow Navigation State

**Files:**
- Create: `src/flow.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write flow implementation and tests**

Write `src/flow.rs`:

```rust
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowState {
    pub cwd: PathBuf,
    pub selected: usize,
}

impl FlowState {
    pub fn new(start: PathBuf) -> Self {
        Self { cwd: start, selected: 0 }
    }

    pub fn entries(&self) -> Result<Vec<FlowEntry>> {
        list_entries(&self.cwd)
    }

    pub fn parent(&mut self) {
        if let Some(parent) = self.cwd.parent() {
            self.cwd = parent.to_path_buf();
            self.selected = 0;
        }
    }

    pub fn enter(&mut self, entry: &FlowEntry) {
        if entry.is_dir {
            self.cwd = entry.path.clone();
            self.selected = 0;
        }
    }
}

pub fn list_entries(path: &Path) -> Result<Vec<FlowEntry>> {
    let mut entries = Vec::new();
    if let Some(parent) = path.parent() {
        entries.push(FlowEntry {
            path: parent.to_path_buf(),
            name: "..".to_owned(),
            is_dir: true,
        });
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let name = entry.file_name().to_string_lossy().to_string();
        entries.push(FlowEntry {
            path: entry.path(),
            name,
            is_dir: file_type.is_dir(),
        });
    }

    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
    Ok(entries)
}

pub fn git_root_from(start: &Path) -> Option<PathBuf> {
    let mut current = start;
    loop {
        if current.join(".git").exists() {
            return Some(current.to_path_buf());
        }
        current = current.parent()?;
    }
}

pub fn flow_start(current: &Path, start_from_git_root: bool) -> PathBuf {
    if start_from_git_root {
        git_root_from(current).unwrap_or_else(|| current.to_path_buf())
    } else {
        current.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_parent_dirs_then_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("Cargo.toml"), "").unwrap();

        let entries = list_entries(dir.path()).unwrap();

        assert_eq!(entries[0].name, "..");
        assert_eq!(entries[1].name, "src");
        assert!(entries[1].is_dir);
        assert_eq!(entries[2].name, "Cargo.toml");
        assert!(!entries[2].is_dir);
    }

    #[test]
    fn detects_git_root() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::create_dir_all(dir.path().join("src/nested")).unwrap();

        assert_eq!(
            git_root_from(&dir.path().join("src/nested")),
            Some(dir.path().to_path_buf())
        );
    }

    #[test]
    fn starts_from_git_root_when_enabled() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::create_dir_all(dir.path().join("src/nested")).unwrap();

        assert_eq!(
            flow_start(&dir.path().join("src/nested"), true),
            dir.path().to_path_buf()
        );
    }
}
```

- [ ] **Step 2: Export flow module**

Modify `src/lib.rs`:

```rust
pub mod cli;
pub mod config;
pub mod flow;
pub mod history;
pub mod open;
pub mod path_display;
pub mod search;
pub mod shell;
pub mod ui;
```

- [ ] **Step 3: Run focused tests**

Run:

```bash
cargo test flow
```

Expected: flow tests pass.

- [ ] **Step 4: Commit Task 8**

Run:

```bash
git add src/lib.rs src/flow.rs
git commit -m "feat: add flow navigation state"
```

## Task 9: Command Palette State

**Files:**
- Create: `src/ui/palette.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Write palette state and tests**

Write `src/ui/palette.rs`:

```rust
use std::path::PathBuf;

use crate::path_display::display_path;
use crate::search::SearchFilter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteItemKind {
    Menu,
    Dir,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteItem {
    pub label: String,
    pub path: Option<PathBuf>,
    pub kind: PaletteItemKind,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteState {
    pub query: String,
    pub items: Vec<PaletteItem>,
    pub selected: usize,
    pub expanded: bool,
    pub filter: SearchFilter,
}

impl PaletteState {
    pub fn new(items: Vec<PaletteItem>) -> Self {
        Self {
            query: String::new(),
            items,
            selected: 0,
            expanded: false,
            filter: SearchFilter::All,
        }
    }

    pub fn selected_item(&self) -> Option<&PaletteItem> {
        self.items.get(self.selected)
    }

    pub fn move_down(&mut self) {
        if !self.items.is_empty() {
            self.selected = (self.selected + 1).min(self.items.len() - 1);
            self.expanded = false;
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
        self.expanded = false;
    }

    pub fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
    }

    pub fn cycle_filter(&mut self) {
        self.filter = match self.filter {
            SearchFilter::All => SearchFilter::Dirs,
            SearchFilter::Dirs => SearchFilter::Files,
            SearchFilter::Files => SearchFilter::All,
        };
    }

    pub fn replace_items(&mut self, items: Vec<PaletteItem>) {
        self.items = items;
        self.selected = 0;
        self.expanded = false;
    }

    pub fn display_label(&self, item: &PaletteItem, width: usize) -> String {
        if let Some(path) = &item.path {
            display_path(path, self.expanded && self.selected_item() == Some(item), width)
        } else {
            item.label.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> PaletteState {
        PaletteState::new(vec![
            PaletteItem {
                label: "Recent projects".to_owned(),
                path: None,
                kind: PaletteItemKind::Menu,
                source: "menu".to_owned(),
            },
            PaletteItem {
                label: "project".to_owned(),
                path: Some(PathBuf::from("/home/congke/work/ntl-imputation/data/nightlight")),
                kind: PaletteItemKind::Dir,
                source: "recent".to_owned(),
            },
        ])
    }

    #[test]
    fn movement_resets_expansion() {
        let mut state = state();
        state.toggle_expanded();
        state.move_down();
        assert_eq!(state.selected, 1);
        assert!(!state.expanded);
    }

    #[test]
    fn cycles_search_filter() {
        let mut state = state();
        state.cycle_filter();
        assert_eq!(state.filter, SearchFilter::Dirs);
        state.cycle_filter();
        assert_eq!(state.filter, SearchFilter::Files);
        state.cycle_filter();
        assert_eq!(state.filter, SearchFilter::All);
    }

    #[test]
    fn expanded_item_shows_full_path() {
        let mut state = state();
        state.move_down();
        state.toggle_expanded();
        let item = state.selected_item().unwrap();
        assert_eq!(
            state.display_label(item, 20),
            "/home/congke/work/ntl-imputation/data/nightlight"
        );
    }
}
```

- [ ] **Step 2: Export palette module**

Modify `src/ui/mod.rs`:

```rust
pub mod palette;
pub mod theme;
```

- [ ] **Step 3: Run focused tests**

Run:

```bash
cargo test palette
```

Expected: palette state tests pass.

- [ ] **Step 4: Commit Task 9**

Run:

```bash
git add src/ui/mod.rs src/ui/palette.rs
git commit -m "feat: add command palette state"
```

## Task 10: Terminal UI Runtime

**Files:**
- Create: `src/ui/runtime.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add UI runtime result type and event loop**

Write `src/ui/runtime.rs`:

```rust
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, terminal};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Terminal;
use std::io;

use crate::search::SearchFilter;
use crate::ui::palette::{PaletteItem, PaletteState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiOutcome {
    Selected(usize),
    Cancelled,
    Editor(usize),
    System(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiResponse {
    pub outcome: UiOutcome,
    pub state: PaletteState,
}

pub fn run_palette(title: &str, mut state: PaletteState) -> Result<UiOutcome> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = run_palette_loop(&mut terminal, title, &mut state);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run_palette_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    title: &str,
    state: &mut PaletteState,
) -> Result<UiOutcome> {
    loop {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(3), Constraint::Length(1)])
                .split(frame.size());

            let input = Paragraph::new(state.query.as_str())
                .block(Block::default().title(title).borders(Borders::ALL));
            frame.render_widget(input, chunks[0]);

            let width = chunks[1].width.saturating_sub(20) as usize;
            let items: Vec<ListItem> = state
                .items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    let marker = if index == state.selected { "> " } else { "  " };
                    let label = state.display_label(item, width.max(10));
                    let line = format!("{marker}{label}  {}", item.source);
                    let style = if index == state.selected {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default()
                    };
                    ListItem::new(Line::from(line)).style(style)
                })
                .collect();
            frame.render_widget(List::new(items), chunks[1]);

            let footer = Paragraph::new("Up Down select  Space expand  Tab filter  Enter open  Esc cancel");
            frame.render_widget(footer, chunks[2]);
        })?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key {
                    KeyEvent { code: KeyCode::Esc, .. } => return Ok(UiOutcome::Cancelled),
                    KeyEvent { code: KeyCode::Enter, .. } => return Ok(UiOutcome::Selected(state.selected)),
                    KeyEvent { code: KeyCode::Down, .. } => state.move_down(),
                    KeyEvent { code: KeyCode::Up, .. } => state.move_up(),
                    KeyEvent { code: KeyCode::Tab, .. } => state.cycle_filter(),
                    KeyEvent { code: KeyCode::Char(' '), .. } => state.toggle_expanded(),
                    KeyEvent { code: KeyCode::Char('e'), modifiers: KeyModifiers::CONTROL, .. } => {
                        return Ok(UiOutcome::Editor(state.selected));
                    }
                    KeyEvent { code: KeyCode::Char('o'), modifiers: KeyModifiers::CONTROL, .. } => {
                        return Ok(UiOutcome::System(state.selected));
                    }
                    KeyEvent { code: KeyCode::Char(ch), .. } => {
                        state.query.push(ch);
                    }
                    KeyEvent { code: KeyCode::Backspace, .. } => {
                        state.query.pop();
                    }
                    _ => {}
                }
            }
        }

        if terminal::size().is_err() {
            return Ok(UiOutcome::Cancelled);
        }
    }
}

pub fn run_search_palette<F>(title: &str, mut state: PaletteState, mut refresh: F) -> Result<UiResponse>
where
    F: FnMut(&str, SearchFilter) -> Result<Vec<PaletteItem>>,
{
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let outcome = run_search_loop(&mut terminal, title, &mut state, &mut refresh);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(UiResponse {
        outcome: outcome?,
        state,
    })
}

fn run_search_loop<F>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    title: &str,
    state: &mut PaletteState,
    refresh: &mut F,
) -> Result<UiOutcome>
where
    F: FnMut(&str, SearchFilter) -> Result<Vec<PaletteItem>>,
{
    loop {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(3), Constraint::Length(1)])
                .split(frame.size());

            let input = Paragraph::new(state.query.as_str())
                .block(Block::default().title(title).borders(Borders::ALL));
            frame.render_widget(input, chunks[0]);

            let width = chunks[1].width.saturating_sub(20) as usize;
            let items: Vec<ListItem> = state
                .items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    let marker = if index == state.selected { "> " } else { "  " };
                    let label = state.display_label(item, width.max(10));
                    let line = format!("{marker}{label}  {}", item.source);
                    let style = if index == state.selected {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default()
                    };
                    ListItem::new(Line::from(line)).style(style)
                })
                .collect();
            frame.render_widget(List::new(items), chunks[1]);

            let footer = Paragraph::new("Tab filter  Space expand  Ctrl+e editor  Ctrl+o open  Enter select");
            frame.render_widget(footer, chunks[2]);
        })?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key {
                    KeyEvent { code: KeyCode::Esc, .. } => return Ok(UiOutcome::Cancelled),
                    KeyEvent { code: KeyCode::Enter, .. } => return Ok(UiOutcome::Selected(state.selected)),
                    KeyEvent { code: KeyCode::Down, .. } => state.move_down(),
                    KeyEvent { code: KeyCode::Up, .. } => state.move_up(),
                    KeyEvent { code: KeyCode::Char(' '), .. } => state.toggle_expanded(),
                    KeyEvent { code: KeyCode::Tab, .. } => {
                        state.cycle_filter();
                        let items = refresh(&state.query, state.filter)?;
                        state.replace_items(items);
                    }
                    KeyEvent { code: KeyCode::Char('e'), modifiers: KeyModifiers::CONTROL, .. } => {
                        return Ok(UiOutcome::Editor(state.selected));
                    }
                    KeyEvent { code: KeyCode::Char('o'), modifiers: KeyModifiers::CONTROL, .. } => {
                        return Ok(UiOutcome::System(state.selected));
                    }
                    KeyEvent { code: KeyCode::Char(ch), .. } => {
                        state.query.push(ch);
                        let items = refresh(&state.query, state.filter)?;
                        state.replace_items(items);
                    }
                    KeyEvent { code: KeyCode::Backspace, .. } => {
                        state.query.pop();
                        let items = refresh(&state.query, state.filter)?;
                        state.replace_items(items);
                    }
                    _ => {}
                }
            }
        }
    }
}
```

- [ ] **Step 2: Export runtime module**

Modify `src/ui/mod.rs`:

```rust
pub mod palette;
pub mod runtime;
pub mod theme;
```

- [ ] **Step 3: Run build**

Run:

```bash
cargo test
cargo build
```

Expected: tests pass and the binary builds.

- [ ] **Step 4: Commit Task 10**

Run:

```bash
git add src/main.rs src/ui/mod.rs src/ui/runtime.rs
git commit -m "feat: add terminal palette runtime"
```

## Task 11: Recent, Flow, And Search Command Flows

**Files:**
- Modify: `src/main.rs`
- Modify: `src/search.rs`
- Modify: `src/ui/palette.rs`
- Modify: `tests/cli_smoke.rs`

- [ ] **Step 1: Add conversion helpers for palette items**

Append to `src/ui/palette.rs`:

```rust
impl PaletteItem {
    pub fn dir(path: PathBuf, source: impl Into<String>) -> Self {
        let label = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_else(|| path.to_str().unwrap_or(""))
            .to_owned();
        Self {
            label,
            path: Some(path),
            kind: PaletteItemKind::Dir,
            source: source.into(),
        }
    }

    pub fn file(path: PathBuf, source: impl Into<String>) -> Self {
        let label = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_else(|| path.to_str().unwrap_or(""))
            .to_owned();
        Self {
            label,
            path: Some(path),
            kind: PaletteItemKind::File,
            source: source.into(),
        }
    }

    pub fn menu(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            path: None,
            kind: PaletteItemKind::Menu,
            source: "menu".to_owned(),
        }
    }
}
```

- [ ] **Step 2: Replace minimal main routing with command runners**

Write `src/main.rs`:

```rust
use anyhow::{Context, Result};
use at::cli::{Cli, Command, ShellCommand};
use at::config::{default_config_path, Config};
use at::history::{default_history_path, HistoryDb, HistorySource, PathKind};
use at::open::{resolve_open_action, OpenAction, OpenMode};
use at::search::{search, SearchFilter, SearchRequest};
use at::ui::palette::{PaletteItem, PaletteState};
use at::ui::runtime::{run_palette, run_search_palette, UiOutcome};
use clap::Parser;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let command = cli.command.unwrap_or(Command::Menu { shell: false });
    match command {
        Command::Menu { shell } => run_menu(shell),
        Command::Recent { shell } => run_recent(shell),
        Command::Flow { shell } => run_flow(shell),
        Command::Search { shell, query } => run_search(shell, Command::search_query(&query)),
        Command::Setting => run_setting(),
        Command::Init => at::init::run_init(),
        Command::RecentRecord { path } => record_cd_hook(&path),
        Command::Shell { command } => match command {
            ShellCommand::Print => {
                println!("{}", at::shell::functions_block());
                Ok(())
            }
            ShellCommand::Hook => {
                println!("{}", at::shell::cd_hook_block());
                Ok(())
            }
        },
    }
}

fn load_config() -> Result<Config> {
    Config::load_or_default(&default_config_path())
}

fn run_menu(shell: bool) -> Result<()> {
    let items = vec![
        PaletteItem::menu("Recent projects"),
        PaletteItem::menu("Flow navigator"),
        PaletteItem::menu("Search files"),
        PaletteItem::menu("Settings"),
    ];
    let outcome = run_palette("@", PaletteState::new(items))?;
    match outcome {
        UiOutcome::Selected(0) => run_recent(shell),
        UiOutcome::Selected(1) => run_flow(shell),
        UiOutcome::Selected(2) => run_search(shell, None),
        UiOutcome::Selected(3) => run_setting(),
        _ => Ok(()),
    }
}

fn run_recent(shell: bool) -> Result<()> {
    let config = load_config()?;
    let db = HistoryDb::open(&default_history_path())?;
    let entries = db.recent_dirs(config.general.max_recent)?;
    let items = entries
        .into_iter()
        .map(|entry| PaletteItem::dir(entry.path, entry.source.as_str()))
        .collect();
    let state = PaletteState::new(items);
    handle_palette_result("@recent", state, shell)
}

fn run_flow(shell: bool) -> Result<()> {
    let config = load_config()?;
    let start = at::flow::flow_start(&std::env::current_dir()?, config.general.start_from_git_root);
    let entries = at::flow::list_entries(&start)?;
    let items = entries
        .into_iter()
        .map(|entry| {
            if entry.is_dir {
                PaletteItem::dir(entry.path, "flow")
            } else {
                PaletteItem::file(entry.path, "flow")
            }
        })
        .collect();
    handle_palette_result("@flow", PaletteState::new(items), shell)
}

fn run_search(shell: bool, query: Option<String>) -> Result<()> {
    let config = load_config()?;
    let mut roots = vec![std::env::current_dir()?];
    roots.extend(config.search.roots.iter().map(expand_home));
    let make_items = |query_text: &str, filter: SearchFilter| -> Result<Vec<PaletteItem>> {
        let results = search(&SearchRequest {
            roots: roots.clone(),
            query: (!query_text.trim().is_empty()).then(|| query_text.to_owned()),
            filter,
            ignore_names: config.search.ignore.clone(),
            limit: 100,
        })?;
        Ok(results
            .into_iter()
            .map(|result| {
                if result.is_dir {
                    PaletteItem::dir(result.path, result.source)
                } else {
                    PaletteItem::file(result.path, result.source)
                }
            })
            .collect())
    };
    let initial_query = query.unwrap_or_default();
    let mut state = PaletteState::new(make_items(&initial_query, SearchFilter::All)?);
    state.query = initial_query;
    handle_search_result("@search", state, shell, make_items)
}

fn handle_search_result<F>(title: &str, state: PaletteState, shell: bool, refresh: F) -> Result<()>
where
    F: FnMut(&str, SearchFilter) -> Result<Vec<PaletteItem>>,
{
    let config = load_config()?;
    let response = run_search_palette(title, state, refresh)?;
    let index = match response.outcome {
        UiOutcome::Selected(index) | UiOutcome::Editor(index) | UiOutcome::System(index) => index,
        UiOutcome::Cancelled => return Ok(()),
    };
    let Some(item) = response.state.items.get(index) else {
        return Ok(());
    };
    let Some(path) = &item.path else {
        return Ok(());
    };
    let is_dir = matches!(item.kind, at::ui::palette::PaletteItemKind::Dir);
    let mode = match response.outcome {
        UiOutcome::Editor(_) => OpenMode::Editor,
        UiOutcome::System(_) => OpenMode::System,
        _ => OpenMode::Default,
    };
    run_open_action(path, is_dir, mode, shell, &config)
}

fn handle_palette_result(title: &str, state: PaletteState, shell: bool) -> Result<()> {
    let config = load_config()?;
    let outcome = run_palette(title, state.clone())?;
    let index = match outcome {
        UiOutcome::Selected(index) | UiOutcome::Editor(index) | UiOutcome::System(index) => index,
        UiOutcome::Cancelled => return Ok(()),
    };
    let Some(item) = state.items.get(index) else {
        return Ok(());
    };
    let Some(path) = &item.path else {
        return Ok(());
    };
    let is_dir = matches!(item.kind, at::ui::palette::PaletteItemKind::Dir);
    let mode = match outcome {
        UiOutcome::Editor(_) => OpenMode::Editor,
        UiOutcome::System(_) => OpenMode::System,
        _ => OpenMode::Default,
    };
    run_open_action(path, is_dir, mode, shell, &config)
}

fn run_open_action(path: &Path, is_dir: bool, mode: OpenMode, shell: bool, config: &Config) -> Result<()> {
    match resolve_open_action(path, is_dir, mode, config) {
        OpenAction::Cd(path) => {
            record_atflow_open(&path, PathKind::Dir)?;
            if shell {
                println!("{}", at::shell::cd_command(&path));
            } else {
                println!("{}", path.display());
            }
        }
        OpenAction::Editor { command, path } | OpenAction::System { command, path } => {
            record_atflow_open(&path, PathKind::File)?;
            std::process::Command::new(command)
                .arg(path)
                .status()
                .context("failed to launch opener")?;
        }
    }
    Ok(())
}

fn record_atflow_open(path: &Path, kind: PathKind) -> Result<()> {
    let db = HistoryDb::open(&default_history_path())?;
    let now = unix_now()?;
    db.record_path_at(path, kind, HistorySource::Atflow, now)
}

fn record_cd_hook(path: &str) -> Result<()> {
    let db = HistoryDb::open(&default_history_path())?;
    db.record_path_at(Path::new(path), PathKind::Dir, HistorySource::ShellCdHook, unix_now()?)
}

fn unix_now() -> Result<i64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64)
}

fn expand_home(value: &str) -> PathBuf {
    if value == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from(value));
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(rest);
    }
    PathBuf::from(value)
}

fn run_setting() -> Result<()> {
    println!("{}", default_config_path().display());
    Ok(())
}
```

- [ ] **Step 3: Add init module stub required by main**

Create `src/init.rs`:

```rust
use anyhow::Result;

pub fn run_init() -> Result<()> {
    println!("Run `at init` configuration wizard");
    Ok(())
}
```

Modify `src/lib.rs` to export init:

```rust
pub mod cli;
pub mod config;
pub mod flow;
pub mod history;
pub mod init;
pub mod open;
pub mod path_display;
pub mod search;
pub mod shell;
pub mod ui;
```

- [ ] **Step 4: Run build and tests**

Run:

```bash
cargo test
cargo build
```

Expected: all tests pass and the binary builds.

- [ ] **Step 5: Commit Task 11**

Run:

```bash
git add src/main.rs src/lib.rs src/init.rs src/search.rs src/ui/palette.rs tests/cli_smoke.rs
git commit -m "feat: wire MVP command flows"
```

## Task 12: Init Wizard

**Files:**
- Modify: `src/init.rs`
- Modify: `src/config.rs`
- Modify: `tests/cli_smoke.rs`

- [ ] **Step 1: Add non-interactive config builder for tests**

Write `src/init.rs`:

```rust
use anyhow::Result;
use std::io::{self, Write};

use crate::config::{default_config_path, Config};
use crate::ui::theme::ThemeName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitAnswers {
    pub install_shell_functions: bool,
    pub enable_cd_hook: bool,
    pub editor: String,
    pub search_roots: Vec<String>,
    pub theme: ThemeName,
    pub start_from_git_root: bool,
}

impl Default for InitAnswers {
    fn default() -> Self {
        Self {
            install_shell_functions: true,
            enable_cd_hook: false,
            editor: std::env::var("EDITOR").unwrap_or_else(|_| "nvim".to_owned()),
            search_roots: vec!["~/work".to_owned(), "~/code".to_owned(), "~/Documents".to_owned()],
            theme: ThemeName::Mist,
            start_from_git_root: true,
        }
    }
}

pub fn config_from_answers(answers: &InitAnswers) -> Config {
    let mut config = Config::default();
    config.open.editor = answers.editor.clone();
    config.search.roots = answers.search_roots.clone();
    config.general.theme = answers.theme;
    config.general.start_from_git_root = answers.start_from_git_root;
    config.history.record_shell_cd = answers.enable_cd_hook;
    config
}

pub fn run_init() -> Result<()> {
    println!("Atflow setup");
    let answers = prompt_answers()?;
    let config = config_from_answers(&answers);
    config.save_to(&default_config_path())?;

    if answers.install_shell_functions {
        println!("\nAdd this to your shell profile:\n");
        println!("{}", crate::shell::functions_block());
    }
    if answers.enable_cd_hook {
        println!("\nAdd this cd hook after the Atflow functions:\n");
        println!("{}", crate::shell::cd_hook_block());
    }
    Ok(())
}

fn prompt_answers() -> Result<InitAnswers> {
    let mut answers = InitAnswers::default();
    answers.install_shell_functions = yes_no("Install @ shell functions?", true)?;
    answers.enable_cd_hook = yes_no("Record normal cd history with a shell hook?", false)?;
    answers.editor = prompt_text("Default editor", &answers.editor)?;
    answers.search_roots = prompt_roots("Search roots", &answers.search_roots)?;
    answers.theme = prompt_theme()?;
    answers.start_from_git_root = yes_no("Start @flow from Git root when possible?", true)?;
    Ok(answers)
}

fn yes_no(prompt: &str, default: bool) -> Result<bool> {
    let suffix = if default { "[Y/n]" } else { "[y/N]" };
    print!("{prompt} {suffix} ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim().to_lowercase();
    if trimmed.is_empty() {
        Ok(default)
    } else {
        Ok(matches!(trimmed.as_str(), "y" | "yes"))
    }
}

fn prompt_text(prompt: &str, default: &str) -> Result<String> {
    print!("{prompt} [{default}] ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim();
    Ok(if trimmed.is_empty() { default.to_owned() } else { trimmed.to_owned() })
}

fn prompt_roots(prompt: &str, default: &[String]) -> Result<Vec<String>> {
    let joined = default.join(",");
    let input = prompt_text(prompt, &joined)?;
    Ok(input
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn prompt_theme() -> Result<ThemeName> {
    print!("Theme: 1) Mist 2) Ink 3) Paper [1] ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(match input.trim() {
        "2" => ThemeName::Ink,
        "3" => ThemeName::Paper,
        _ => ThemeName::Mist,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_config_from_answers() {
        let answers = InitAnswers {
            install_shell_functions: true,
            enable_cd_hook: true,
            editor: "code".to_owned(),
            search_roots: vec!["~/work".to_owned()],
            theme: ThemeName::Ink,
            start_from_git_root: false,
        };

        let config = config_from_answers(&answers);

        assert_eq!(config.open.editor, "code");
        assert_eq!(config.search.roots, vec!["~/work"]);
        assert_eq!(config.general.theme, ThemeName::Ink);
        assert!(!config.general.start_from_git_root);
        assert!(config.history.record_shell_cd);
    }
}
```

- [ ] **Step 2: Add init CLI smoke test with newline defaults**

Append to `tests/cli_smoke.rs`:

```rust
#[test]
fn init_accepts_default_answers() {
    Command::cargo_bin("at")
        .unwrap()
        .arg("init")
        .write_stdin("\n\n\n\n\n\n")
        .assert()
        .success()
        .stdout(predicates::str::contains("Atflow setup"));
}
```

- [ ] **Step 3: Run tests**

Run:

```bash
cargo test init
cargo test --test cli_smoke
```

Expected: init unit tests and CLI smoke tests pass.

- [ ] **Step 4: Commit Task 12**

Run:

```bash
git add src/init.rs tests/cli_smoke.rs
git commit -m "feat: add init wizard"
```

## Task 13: Installer And Documentation

**Files:**
- Create: `scripts/install.sh`
- Create: `README.md`
- Modify: `.gitignore`

- [ ] **Step 1: Write Linux/WSL installer script**

Write `scripts/install.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

repo_url="${ATFLOW_REPO_URL:-https://github.com/zhangcongke/atflow.git}"
install_dir="${ATFLOW_INSTALL_DIR:-$HOME/.local/bin}"
work_dir="$(mktemp -d)"

cleanup() {
  rm -rf "$work_dir"
}
trap cleanup EXIT

mkdir -p "$install_dir"

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required for this development installer."
  echo "Install Rust from https://rustup.rs, then rerun this script."
  exit 1
fi

git clone --depth 1 "$repo_url" "$work_dir/atflow"
cargo install --path "$work_dir/atflow" --root "$HOME/.local"

echo "Installed at to $install_dir/at"
echo
echo "Starting first-run configuration..."
"$install_dir/at" init
```

- [ ] **Step 2: Make installer executable**

Run:

```bash
chmod +x scripts/install.sh
```

Expected: `scripts/install.sh` is executable.

- [ ] **Step 3: Write README**

Write `README.md`:

```markdown
# Atflow

Atflow is a lightweight `@` command palette for Linux terminals.

The installed binary is `at`. Daily shell entry points are installed as shell functions:

```bash
@
@recent
@flow
@search
@search nightlight
@setting
```

## Status

Atflow is in early MVP development. Linux/WSL is the first supported target.

## Development Install

```bash
curl -fsSL https://raw.githubusercontent.com/zhangcongke/atflow/main/scripts/install.sh | sh
```

The installer installs `at` and starts:

```bash
at init
```

## Manual Setup

```bash
cargo install --path .
at init
```

Add the printed shell functions to your shell profile.

## Data Locations

- Config: `~/.config/at/config.toml`
- History: `~/.local/share/at/history.sqlite`

## MVP Behavior

- `@` opens the main command palette.
- `@recent` opens recently used directories.
- `@flow` browses directories from the current directory or Git root.
- `@search` opens search with an empty query.
- `@search nightlight` opens search with `nightlight` prefilled.
- Long paths are clipped in the middle.
- `Space` expands the selected path.
```

- [ ] **Step 4: Run format, tests, and build**

Run:

```bash
cargo fmt
cargo test
cargo build
```

Expected: format completes, all tests pass, and the binary builds.

- [ ] **Step 5: Commit Task 13**

Run:

```bash
git add README.md scripts/install.sh .gitignore
git commit -m "docs: add installer and README"
```

## Task 14: Manual Linux/WSL Verification

**Files:**
- Modify: `docs/superpowers/specs/2026-06-30-atflow-design.md` only if verification reveals a spec correction.

- [ ] **Step 1: Build release binary**

Run:

```bash
cargo build --release
```

Expected: `target/release/at` exists.

- [ ] **Step 2: Verify shell functions output**

Run:

```bash
target/release/at shell print
```

Expected: output contains `@()`, `@recent()`, `@flow()`, `@search()`, and `@setting()`.

- [ ] **Step 3: Verify config wizard can create config in a temporary HOME**

Run:

```bash
tmp_home="$(mktemp -d)"
HOME="$tmp_home" target/release/at init <<'EOF'






EOF
test -f "$tmp_home/.config/at/config.toml"
```

Expected: command exits 0 and the config file exists.

- [ ] **Step 4: Verify cd hook recording in a temporary HOME**

Run:

```bash
tmp_home="$(mktemp -d)"
HOME="$tmp_home" target/release/at recent-record /tmp
test -f "$tmp_home/.local/share/at/history.sqlite"
```

Expected: command exits 0 and the SQLite database file exists.

- [ ] **Step 5: Run interactive UI smoke checks**

Run:

```bash
target/release/at menu
target/release/at search atflow
target/release/at flow
```

Expected:

- `Esc` exits each view without printing a shell command.
- `Up` and `Down` move selection.
- `Space` expands and collapses the selected path.
- `Tab` cycles search filter in search mode.
- `Enter` on a directory in `--shell` mode prints a `cd` command.

- [ ] **Step 6: Push implementation branch**

Run:

```bash
git status --short
git log --oneline --decorate -5
git push
```

Expected: branch pushes to `origin/main`.

## Plan Self-Review

Spec coverage:

- `at` binary and `@` shell functions are covered by Tasks 1, 4, 11, and 13.
- `@recent`, `@flow`, `@search`, and `@setting` are covered by Tasks 5, 8, 11, and 12.
- SQLite history is covered by Task 5.
- Command Palette UI, themes, path clipping, and `Space` expansion are covered by Tasks 2, 9, and 10.
- Linux/WSL installer and first-run init flow are covered by Tasks 12 and 13.
- Manual verification for WSL/Linux behavior is covered by Task 14.

Placeholder scan:

- The plan avoids forbidden placeholder tokens and open-ended implementation instructions.
- The plan includes exact file paths, test commands, and commit points.

Type consistency:

- `ThemeName`, `SearchFilter`, `PaletteState`, `HistoryDb`, `PathKind`, `HistorySource`, `OpenAction`, and `UiOutcome` are introduced before later tasks use them.
- CLI commands in `src/cli.rs` match command runners in `src/main.rs`.
