# Atflow

Atflow is a lightweight terminal command palette for moving through projects, recent directories, and files from a Linux shell. It installs a binary named `at` and shell shortcuts that make the palette feel like native commands.

Status: MVP development preview. Linux and WSL are the first supported environments. The current focus is the Rust binary, shell integration, and local development installs; packaged installers are a long-term goal.

## Commands

- `at`: opens the main menu.
- `at recent`: opens recent directories.
- `at flow`: opens the flow navigator.
- `at search [query]`: searches files and directories, optionally starting with `query`.
- `at setting`: opens the config file in your configured editor.
- `at setting --path`: prints the config file path.
- `at init`: runs the setup wizard.
- `at shell print`: prints the shell functions.
- `at shell hook`: prints the optional `cd` history hook.

After `at init`, restart your shell or source the generated shell integration file to enable:

- `@`: main menu.
- `@recent`: recent directories.
- `@flow`: flow navigator.
- `@search`: search palette.
- `@search query`: search palette with an initial query.
- `@setting`: open settings in your editor.

## Install

Development install from GitHub:

```bash
bash <(curl -fsSL https://raw.githubusercontent.com/zhangcongke/atflow/main/scripts/install.sh)
```

The installer clones the public repo, runs `cargo install --path ... --locked`, prints the installed `at` path, then runs `at init`.

Defaults:

- Repository: `https://github.com/zhangcongke/atflow.git`
- Install directory: `$HOME/.local/bin`

Overrides:

```bash
ATFLOW_REPO_URL=https://github.com/zhangcongke/atflow.git \
ATFLOW_INSTALL_DIR="$HOME/.local/bin" \
bash <(curl -fsSL https://raw.githubusercontent.com/zhangcongke/atflow/main/scripts/install.sh)
```

`ATFLOW_REPO_URL` can point to an accessible fork or local mirror. `ATFLOW_INSTALL_DIR` is the final directory that will contain the `at` binary, so `ATFLOW_INSTALL_DIR=/tmp/atflow-install-test` installs `/tmp/atflow-install-test/at`.

Use process substitution or download the script before running it. Avoid `curl ... | bash`: `at init` is interactive, and a pipeline leaves the installer without terminal stdin.

Make sure the install directory is on `PATH` before using the shell functions, because they call `command at`:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Add that line to your shell profile if `$HOME/.local/bin` is not already present. For a custom `ATFLOW_INSTALL_DIR`, add that directory instead.

`at init` writes shell shortcuts to `${XDG_CONFIG_HOME:-$HOME/.config}/at/shell.sh` and adds a source line to your shell profile (`.bashrc`, `.zshrc`, or `.profile`). The shortcuts affect the current shell only after you restart it or run the source command printed by `at init`.

## Manual Setup

From a local checkout:

```bash
cargo install --path . --locked
at init
```

If you want the binary in `$HOME/.local/bin`, run:

```bash
cargo install --path . --locked --root "$HOME/.local"
"$HOME/.local/bin/at" init
```

## Data Locations

Atflow follows the XDG directories used by the platform:

- Config: `${XDG_CONFIG_HOME:-$HOME/.config}/at/config.toml`
- Shell integration: `${XDG_CONFIG_HOME:-$HOME/.config}/at/shell.sh`
- History: `${XDG_DATA_HOME:-$HOME/.local/share}/at/history.sqlite`

`@setting` or `at setting` opens the active config file. Use `at setting --path` when you only need the path.

## MVP Behavior

The main `@` menu links to recent projects, flow navigation, search, and settings.

`@recent` shows recently opened directories from Atflow history. If the optional `cd` hook is enabled, ordinary shell `cd` usage is also recorded.

`@flow` starts from the current Git root by default when one is found, otherwise from the current directory. The init wizard can disable Git-root start. Use Up/Down to move, Left or `h` to go to the parent directory, Right or `l` to enter the selected directory, Enter to select, Ctrl+E to open with the editor, and Ctrl+O to open with the system opener.

`@search` searches the current directory, configured roots, and recent directories. `@search query` starts with `query` already typed; multiple words are joined with spaces. Tab cycles all, dirs, and files. Search respects git ignore files and the configured ignore names.

Long paths are clipped in the middle to fit the terminal row. Press Space to expand the selected path to its full text; moving the selection collapses it again.

## Init Choices

`at init` prompts for:

- Whether to install shell shortcuts for `@`, `@recent`, `@flow`, `@search`, and `@setting`.
- Whether to install and enable the `cd` hook for shell directory history.
- The terminal editor command.
- Search roots.
- Theme.
- Whether `@flow` starts from the current Git root.

## Packaging Goals

Long-term package goals:

- `.deb` for Linux.
- `.msi` for Windows.
- `.dmg` for macOS.
