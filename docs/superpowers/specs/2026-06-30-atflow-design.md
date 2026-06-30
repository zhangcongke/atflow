# Atflow MVP Design

Date: 2026-06-30

## Summary

Atflow is a lightweight `@` command palette for Linux terminals. Its first release focuses on Linux/WSL and provides a complete workflow for opening a terminal menu, switching to recent projects, browsing directories, searching files or folders, and configuring shell integration.

The user-facing shell command is `@`. The installed Rust binary is `at`.

## Goals

- Provide a fast terminal command palette for developer workflows.
- Support `@`, `@recent`, `@flow`, `@search`, and `@setting` as shell-level entry points.
- Keep the MVP complete enough to feel like a real product, even if each feature starts shallow.
- Prioritize a polished terminal UI using a command-palette layout.
- Make first-time setup convenient through an installer-driven `at init` flow.
- Store recent/history data in SQLite from the first version.
- Target Linux/WSL first, with packaging plans for `.deb`, `.msi`, and `.dmg` later.

## Non-Goals For MVP

- No full-disk search.
- No background indexing daemon.
- No Windows PowerShell integration in the first implementation.
- No macOS packaging in the first implementation.
- No deep agent launcher beyond leaving room in the command model for a later `agent` feature.
- No SQLite server or external database dependency.

## Architecture

The first implementation uses a single Rust crate with binary name `at`. A workspace can be introduced later if the project grows, but the MVP should avoid premature crate splitting.

Planned modules:

- `cli`: command parsing and dispatch for `menu`, `recent`, `flow`, `search`, `setting`, and `init`.
- `ui`: ratatui/crossterm terminal UI components, including the command palette, search box, footer hints, selection state, and path clipping.
- `config`: read/write `~/.config/at/config.toml`.
- `history`: SQLite-backed history and recent storage at `~/.local/share/at/history.sqlite`.
- `shell`: shell integration output, `--shell` behavior, and installable shell functions/hooks.
- `search`: scoped search over current directory, recent paths, and configured roots.
- `flow`: directory browsing state and navigation.
- `open`: folder/file action resolution.

History uses SQLite through Rust, preferably `rusqlite` with bundled SQLite, so ordinary users do not need to install SQLite separately. Developers building from source need the normal native build toolchain required to compile bundled SQLite.

## Commands

User-facing shell entry points:

```bash
@
@recent
@flow
@search
@search nightlight
@setting
```

Underlying binary commands:

```bash
at menu --shell
at recent --shell
at flow --shell
at search --shell
at search --shell nightlight
at setting
at init
```

`@search` has two modes:

- Without a query, it opens the search UI with an empty search box.
- With a query, it opens the same UI with the query prefilled and matching results shown immediately.

## Shell Integration

`at init` installs or prints shell functions after user confirmation:

```bash
@()        { eval "$(at menu --shell "$@")"; }
@recent()  { eval "$(at recent --shell "$@")"; }
@flow()    { eval "$(at flow --shell "$@")"; }
@search()  { eval "$(at search --shell "$@")"; }
@setting() { at setting "$@"; }
```

The shell functions exist because a child process cannot directly change the parent shell directory. In `--shell` mode, `at` prints shell commands such as:

```bash
cd '/home/user/work/project'
```

The shell function evaluates that output in the active shell.

An optional shell `cd` hook can record normal `cd` usage into history. It is offered in `at init` but is not forced.

## Interaction Design

Common keys:

- `Up` / `Down`: move selection.
- `Enter`: open the selected item.
- `Esc`: cancel and exit without shell output.
- `Space`: expand or collapse the selected item so long paths can be inspected.

Search-specific keys:

- `Tab`: cycle result filter between `all`, `dirs`, and `files`.
- `Ctrl+e`: open selected file with the configured editor.
- `Ctrl+o`: open selected file with the system opener.

Open behavior:

- Directories output a `cd` command in shell mode.
- Code/text files open with `$EDITOR`, configured editor, or `nvim` fallback.
- Other files open with `xdg-open` on Linux/WSL.

`@flow` starts from the current directory by default. If `start_from_git_root` is enabled and the current directory is inside a Git repository, it starts from the repository root.

## UI Design

The MVP uses a command-palette style terminal UI.

Default theme: Mist Console.

Additional theme options:

- Ink Terminal
- Paper Shell

UI rules:

- Keep the layout compact and fast.
- Avoid heavy borders and noisy colors.
- Show a search/input line when relevant.
- Show a fixed-height result list.
- Show a compact footer with current key hints.
- Mark selected item with `>`.
- Label entries with simple types such as `[dir]` and `[file]`.
- Clip long paths with a middle ellipsis, not by truncating only the end.
- Use `Space` to show the full selected path.

Example clipped result:

```text
> [dir]  ~/work/ntl-imput.../data/nightlight     recent
```

Example expanded result:

```text
Space: /home/congke/work/ntl-imputation/data/nightlight
```

## Configuration

Configuration file:

```text
~/.config/at/config.toml
```

Initial fields:

```toml
[general]
theme = "mist"
max_recent = 100
start_from_git_root = true

[open]
editor = "nvim"
gui_editor = "code"
file_opener = "xdg-open"
prefer_terminal_editor = true

[search]
roots = ["~/work", "~/code", "~/Documents"]
ignore = [".git", "node_modules", "__pycache__", ".venv", "target", "dist"]

[history]
record_atflow_opens = true
record_shell_cd = false
```

## SQLite History

History database:

```text
~/.local/share/at/history.sqlite
```

Initial table shape:

```sql
CREATE TABLE paths (
  id INTEGER PRIMARY KEY,
  path TEXT NOT NULL UNIQUE,
  kind TEXT NOT NULL,
  source TEXT NOT NULL,
  last_opened_at INTEGER NOT NULL,
  open_count INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_paths_last_opened ON paths(last_opened_at DESC);
CREATE INDEX idx_paths_kind ON paths(kind);
```

Initial `source` values:

- `atflow`
- `shell_cd_hook`
- `manual_root_scan`

## Installer And Packaging

The first install path is Linux/WSL-focused:

```bash
curl -fsSL https://example.com/atflow/install.sh | sh
```

The installer downloads or builds the `at` binary, places it on `PATH`, and runs:

```bash
at init
```

`at init` asks:

- Whether to install shell functions.
- Whether to enable the optional `cd` hook.
- Which editor to use.
- Which search roots to configure.
- Which theme to use.
- Whether `@flow` should start from Git root.

Long-term packaging goals:

- Linux: `.deb`
- Windows: `.msi`
- macOS: `.dmg`

Package installs should avoid fragile package-manager interactivity. They should install the binary, then let first run or an explicit `at init` handle user configuration.

## Development Milestones

1. Project skeleton and CLI routing.
2. Config system and `at init` wizard.
3. SQLite history.
4. UI base components.
5. Recent, Flow, and Search MVP flows.
6. Shell integration and `--shell` behavior.
7. Linux/WSL install script and documentation.

## Testing Strategy

Unit tests:

- Config defaults and config read/write.
- Middle-ellipsis path clipping.
- SQLite insert, update, sorting, and source handling.
- Search filters for `all`, `dirs`, and `files`.
- Open strategy resolution for directories, text/code files, and other files.

Integration tests:

- `at init` can generate config.
- Shell integration output contains expected functions.
- `--shell` mode emits expected shell commands for selected directories.

Manual UI verification:

- Narrow and wide terminal windows.
- Long path clipping and `Space` expansion.
- `Enter`, `Tab`, `Esc`, `Ctrl+e`, and `Ctrl+o`.
- WSL behavior for editor and `xdg-open`.

## Approved Decisions

- MVP scope: complete `@` menu with Recent, Flow, and Search connected.
- Binary name: `at`.
- Shell entry: `@`.
- File open behavior: folders `cd`; text/code files use editor; other files use system opener.
- Recent source: Atflow opens plus optional shell `cd` hook.
- History storage: SQLite.
- Search behavior: one search UI, optional initial query.
- UI style: Command Palette.
- Long paths: middle ellipsis with `Space` expansion.
- Platform priority: Linux/WSL first.
- Default theme: Mist Console.
- Additional themes: Ink Terminal and Paper Shell.
