# AtFlow

AtFlow is a lightweight terminal file flow for moving through projects, recent paths, and files from a Linux shell. It installs a binary named `at` and a shell shortcut named `@`.

Status: MVP development preview. Linux and WSL are the first supported environments. The current focus is the Rust binary, shell integration, and local development installs; packaged installers are a long-term goal.

## Commands

- `at`: opens Flow.
- `at flow [query]`: opens Flow, optionally starting with `query`.
- `at setting`: opens the interactive settings menu.
- `at setting --path`: prints the config file path.
- `at init`: runs the setup wizard.
- `at shell print`: prints the shell functions.
- `at shell hook`: prints the optional `cd` history hook.

After `at init`, restart your shell or source the generated shell integration file to enable:

- `@`: Flow.
- `@ query`: Flow with an initial search query.
- `@ setting`: interactive settings menu.
- `@setting`: interactive settings menu.

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

AtFlow follows the XDG directories used by the platform:

- Config: `${XDG_CONFIG_HOME:-$HOME/.config}/at/config.toml`
- Shell integration: `${XDG_CONFIG_HOME:-$HOME/.config}/at/shell.sh`
- History: `${XDG_DATA_HOME:-$HOME/.local/share}/at/history.sqlite`

`@setting` or `at setting` opens an interactive settings menu. Use Left/Right to change the selected option, Enter to save, and Esc to cancel. Use `at setting --path` when you only need the config file path.

## MVP Behavior

Long lists scroll to keep the selected row visible. Pressing Down on the last item wraps to the first item, and pressing Up on the first item wraps to the last item.

`@` opens Flow. The default list shows pinned paths first, then recently opened files and directories from AtFlow history. If the optional `cd` hook is enabled, ordinary shell `cd` usage is also recorded.

Typing text enters search mode in the same Flow page. By default the search root is the directory where `@` was invoked. Settings can switch search roots to the configured path list instead.

Use Up/Down to move, Enter to enter a selected directory or open a selected file, Right to enter a selected directory, Left to move to the parent directory, Tab to pin or unpin the selected path, and Shift+Tab to cycle through pinned directories as the active root. When you move to a parent directory, the cursor stays on the directory you just left.

Search respects git ignore files and the configured ignore names.

Long paths are clipped in the middle to fit the terminal row. Press Space to expand the selected path to its full text; moving the selection collapses it again.

Theme changes in settings are saved to config and used by the TUI palettes.

## Init Choices

`at init` prompts for:

- Whether to install shell shortcuts for `@` and `@setting`.
- Whether to install and enable the `cd` hook for shell directory history.
- The terminal editor command.
- Search root mode.
- Search roots.
- Theme.
- Whether Flow starts from the current Git root.

## Packaging Goals

Long-term package goals:

- `.deb` for Linux.
- `.msi` for Windows.
- `.dmg` for macOS.
