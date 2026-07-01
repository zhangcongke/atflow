use anyhow::{Context, Result, bail};
use at::cli::{Cli, Command, ShellCommand};
use at::config::{Config, SearchRootMode, default_config_path};
use at::history::{HistoryDb, HistoryEntry, HistorySource, PathKind, default_history_path};
use at::open::{OpenAction, OpenMode, resolve_editor_command, resolve_open_action};
use at::search::{SearchFilter, SearchRequest, search};
use at::ui::palette::{PaletteItem, PaletteItemKind, PaletteState};
use at::ui::runtime::{
    FlowDelegate, SettingsOutcome, UiOutcome, run_flow_palette, run_settings_palette,
};
use clap::Parser;
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::process::Stdio;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let command = cli.command.unwrap_or(Command::Flow {
        shell: false,
        query: Vec::new(),
    });
    match command {
        Command::Flow { shell, query } => run_flow(shell, Command::search_query(&query)),
        Command::Setting { path } => run_setting(false, path).map(|_| ()),
        Command::Init => at::init::run_init(),
        Command::RecentRecord { path } => record_cd_hook(Path::new(&path)),
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

fn run_flow(shell: bool, query: Option<String>) -> Result<()> {
    let config = load_config()?;
    let db = HistoryDb::open(&default_history_path())?;
    let current_dir = std::env::current_dir()?;
    let start = at::flow::flow_start(&current_dir, config.general.start_from_git_root);
    let search_roots = flow_search_roots_from(&config, current_dir);
    let theme = config.general.theme;
    let mut delegate = FlowData {
        config: &config,
        db: &db,
    };
    let response = run_flow_palette("@ Flow", start, search_roots, query, theme, &mut delegate)?;

    handle_open_outcome(&response.outcome, &response.state, shell, &config)
}

fn flow_search_roots_from(config: &Config, current_dir: PathBuf) -> Vec<PathBuf> {
    match config.search.root_mode {
        SearchRootMode::Invocation => vec![current_dir],
        SearchRootMode::Configured => {
            let roots: Vec<PathBuf> = config
                .search
                .roots
                .iter()
                .map(|root| expand_home(root))
                .collect();
            if roots.is_empty() {
                vec![current_dir]
            } else {
                roots
            }
        }
    }
}

fn search_items(
    roots: &[PathBuf],
    ignore_names: &[String],
    query_text: &str,
    filter: SearchFilter,
) -> Result<Vec<PaletteItem>> {
    let trimmed = query_text.trim();
    let results = search(&SearchRequest {
        roots: roots.to_vec(),
        query: (!trimmed.is_empty()).then(|| trimmed.to_owned()),
        filter,
        ignore_names: ignore_names.to_vec(),
        limit: 100,
    })?;
    let mut seen = HashSet::new();

    Ok(results
        .into_iter()
        .filter(|result| {
            let key = result
                .path
                .canonicalize()
                .unwrap_or_else(|_| result.path.clone());
            seen.insert(key)
        })
        .map(|result| {
            if result.is_dir {
                PaletteItem::dir(result.path, result.source)
            } else {
                PaletteItem::file(result.path, result.source)
            }
        })
        .collect())
}

struct FlowData<'a> {
    config: &'a Config,
    db: &'a HistoryDb,
}

impl FlowDelegate for FlowData<'_> {
    fn recent_items(&mut self) -> Result<Vec<PaletteItem>> {
        let limit = self.config.general.max_recent;
        let mut items = Vec::new();
        let mut seen = HashSet::new();

        for entry in self.db.pinned_paths(limit)? {
            let item = history_item(entry).pinned();
            if remember_item_path(&mut seen, &item) {
                items.push(item);
            }
            if items.len() >= limit {
                return Ok(items);
            }
        }

        for entry in self.db.recent_paths(limit)? {
            let item = history_item(entry);
            if remember_item_path(&mut seen, &item) {
                items.push(item);
            }
            if items.len() >= limit {
                break;
            }
        }

        Ok(items)
    }

    fn browse_items(&mut self, cwd: &Path) -> Result<Vec<PaletteItem>> {
        let pinned = self.pinned_path_keys()?;
        at::flow::list_entries(cwd)?
            .into_iter()
            .map(|entry| {
                let item = if entry.is_dir {
                    PaletteItem::dir(entry.path, "flow")
                } else {
                    PaletteItem::file(entry.path, "flow")
                };
                Ok(mark_if_pinned(item, &pinned))
            })
            .collect()
    }

    fn search_items(&mut self, query: &str, roots: &[PathBuf]) -> Result<Vec<PaletteItem>> {
        let pinned = self.pinned_path_keys()?;
        Ok(
            search_items(roots, &self.config.search.ignore, query, SearchFilter::All)?
                .into_iter()
                .map(|item| mark_if_pinned(item, &pinned))
                .collect(),
        )
    }

    fn toggle_pin(&mut self, item: &PaletteItem) -> Result<()> {
        let Some(path) = item.path.as_deref() else {
            return Ok(());
        };
        let kind = match item.kind {
            PaletteItemKind::Dir => PathKind::Dir,
            PaletteItemKind::File => PathKind::File,
            PaletteItemKind::Menu => return Ok(()),
        };
        self.db.toggle_pin_at(path, kind, unix_now()?)
    }

    fn pinned_dirs(&mut self) -> Result<Vec<PathBuf>> {
        self.db
            .pinned_dirs(self.config.general.max_recent)
            .map(|entries| entries.into_iter().map(|entry| entry.path).collect())
    }
}

impl FlowData<'_> {
    fn pinned_path_keys(&self) -> Result<HashSet<PathBuf>> {
        let pinned = self.db.pinned_paths(self.config.general.max_recent)?;
        Ok(pinned
            .into_iter()
            .map(|entry| path_key(&entry.path))
            .collect())
    }
}

fn history_item(entry: HistoryEntry) -> PaletteItem {
    match entry.kind {
        PathKind::Dir => PaletteItem::dir(entry.path, entry.source.as_str()),
        PathKind::File => PaletteItem::file(entry.path, entry.source.as_str()),
    }
}

fn remember_item_path(seen: &mut HashSet<PathBuf>, item: &PaletteItem) -> bool {
    item.path
        .as_deref()
        .map(|path| seen.insert(path_key(path)))
        .unwrap_or(true)
}

fn mark_if_pinned(item: PaletteItem, pinned: &HashSet<PathBuf>) -> PaletteItem {
    let Some(path) = item.path.as_deref() else {
        return item;
    };
    if pinned.contains(&path_key(path)) {
        item.pinned()
    } else {
        item
    }
}

fn path_key(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn handle_open_outcome(
    outcome: &UiOutcome,
    state: &PaletteState,
    shell: bool,
    config: &Config,
) -> Result<()> {
    if matches!(outcome, UiOutcome::Cancelled) {
        return Ok(());
    }
    if let Some(target) = selected_open_target(outcome, state) {
        run_open_action(&target.path, target.is_dir, target.mode, shell, config)?;
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SelectedOpenTarget {
    path: PathBuf,
    is_dir: bool,
    mode: OpenMode,
}

fn selected_open_target(outcome: &UiOutcome, state: &PaletteState) -> Option<SelectedOpenTarget> {
    let (index, mode) = match outcome {
        UiOutcome::Selected(index) => (*index, OpenMode::Default),
        UiOutcome::Editor(index) => (*index, OpenMode::Editor),
        UiOutcome::System(index) => (*index, OpenMode::System),
        UiOutcome::Cancelled => return None,
    };
    let item = state.items.get(index)?;
    let path = item.path.clone()?;
    Some(SelectedOpenTarget {
        path,
        is_dir: matches!(item.kind, PaletteItemKind::Dir),
        mode,
    })
}

fn run_open_action(
    path: &Path,
    is_dir: bool,
    mode: OpenMode,
    shell: bool,
    config: &Config,
) -> Result<()> {
    match resolve_open_action(path, is_dir, mode, config) {
        OpenAction::Cd(path) => {
            record_atflow_open(&path, PathKind::Dir, config)?;
            if shell {
                println!("{}", at::shell::cd_command(&path));
            } else {
                println!("{}", path.display());
            }
        }
        OpenAction::Editor { command, path } => {
            launch_editor(&command, &path, shell)?;
            record_atflow_open(&path, PathKind::File, config)?;
        }
        OpenAction::System { command, path } => {
            launch_opener(&command, &path, shell)?;
            record_atflow_open(&path, PathKind::File, config)?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenerStdio {
    Inherit,
    Tty,
}

fn opener_stdio(shell: bool) -> OpenerStdio {
    if shell {
        OpenerStdio::Tty
    } else {
        OpenerStdio::Inherit
    }
}

fn launch_opener(command: &str, path: &Path, shell: bool) -> Result<()> {
    let mut opener = std::process::Command::new(command);
    opener.arg(path);
    if opener_stdio(shell) == OpenerStdio::Tty {
        attach_tty_stdio(&mut opener)?;
    }

    let status = opener
        .status()
        .with_context(|| format!("failed to launch opener `{command}`"))?;
    if !status.success() {
        bail!("opener `{command}` exited with {status}");
    }
    Ok(())
}

fn launch_editor(command: &str, path: &Path, shell: bool) -> Result<()> {
    let editor = resolve_editor_command(command);
    if let Some(previous) = editor.fallback_from.as_deref()
        && previous != editor.command
    {
        eprintln!(
            "editor `{previous}` was not found; falling back to `{}`",
            editor.command
        );
    }
    launch_opener(&editor.command, path, shell)
}

fn attach_tty_stdio(command: &mut std::process::Command) -> Result<()> {
    let tty = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .context("failed to open /dev/tty for shell-mode opener")?;
    command.stdin(Stdio::from(
        tty.try_clone()
            .context("failed to clone /dev/tty for opener stdin")?,
    ));
    command.stdout(Stdio::from(
        tty.try_clone()
            .context("failed to clone /dev/tty for opener stdout")?,
    ));
    command.stderr(Stdio::from(tty));
    Ok(())
}

fn record_atflow_open(path: &Path, kind: PathKind, config: &Config) -> Result<()> {
    if !config.history.record_atflow_opens {
        return Ok(());
    }
    let db = HistoryDb::open(&default_history_path())?;
    record_atflow_open_at(&db, config, path, kind, unix_now()?)
}

fn record_atflow_open_at(
    db: &HistoryDb,
    config: &Config,
    path: &Path,
    kind: PathKind,
    timestamp: i64,
) -> Result<()> {
    if !config.history.record_atflow_opens {
        return Ok(());
    }
    db.record_path_at(path, kind, HistorySource::Atflow, timestamp)
}

fn record_cd_hook(path: &Path) -> Result<()> {
    let config = load_config()?;
    if !config.history.record_shell_cd {
        return Ok(());
    }
    let db = HistoryDb::open(&default_history_path())?;
    db.record_path_at(path, PathKind::Dir, HistorySource::ShellCdHook, unix_now()?)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InfoStream {
    Stdout,
    Stderr,
}

fn setting_stream(shell: bool) -> InfoStream {
    if shell {
        InfoStream::Stderr
    } else {
        InfoStream::Stdout
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingResult {
    Saved,
    Cancelled,
    PathPrinted,
}

fn run_setting(shell: bool, path_only: bool) -> Result<SettingResult> {
    let path = default_config_path();
    if path_only {
        match setting_stream(shell) {
            InfoStream::Stdout => println!("{}", path.display()),
            InfoStream::Stderr => eprintln!("{}", path.display()),
        }
        return Ok(SettingResult::PathPrinted);
    }

    let config = ensure_config_file(&path)?;
    match run_settings_palette(config)? {
        SettingsOutcome::Saved(config) => {
            config.save_to(&path)?;
            match setting_stream(shell) {
                InfoStream::Stdout => println!("Config saved to {}", path.display()),
                InfoStream::Stderr => eprintln!("Config saved to {}", path.display()),
            }
            Ok(SettingResult::Saved)
        }
        SettingsOutcome::Cancelled => Ok(SettingResult::Cancelled),
    }
}

fn ensure_config_file(path: &Path) -> Result<Config> {
    if path
        .try_exists()
        .with_context(|| format!("failed to inspect config {}", path.display()))?
    {
        return Config::load_or_default(path);
    }

    let config = Config::default();
    config.save_to(path)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use at::config::{Config, SearchRootMode};
    use at::history::{HistoryDb, HistorySource, PathKind};
    use at::open::OpenMode;
    use at::ui::palette::{PaletteItem, PaletteState};
    use at::ui::runtime::UiOutcome;
    use std::fs;
    use std::path::{Path, PathBuf};

    #[test]
    fn selected_open_target_maps_selected_directory_to_default_mode() {
        let path = PathBuf::from("/tmp/project");
        let state = PaletteState::new(vec![PaletteItem::dir(path.clone(), "recent")]);

        let selected = selected_open_target(&UiOutcome::Selected(0), &state).unwrap();

        assert_eq!(
            selected,
            SelectedOpenTarget {
                path,
                is_dir: true,
                mode: OpenMode::Default,
            }
        );
    }

    #[test]
    fn selected_open_target_maps_forced_file_open_modes() {
        let path = PathBuf::from("/tmp/project/src/main.rs");
        let state = PaletteState::new(vec![PaletteItem::file(path.clone(), "search")]);

        assert_eq!(
            selected_open_target(&UiOutcome::Editor(0), &state).unwrap(),
            SelectedOpenTarget {
                path: path.clone(),
                is_dir: false,
                mode: OpenMode::Editor,
            }
        );
        assert_eq!(
            selected_open_target(&UiOutcome::System(0), &state).unwrap(),
            SelectedOpenTarget {
                path,
                is_dir: false,
                mode: OpenMode::System,
            }
        );
    }

    #[test]
    fn selected_open_target_ignores_cancelled_menu_and_stale_indexes() {
        let state = PaletteState::new(vec![PaletteItem::menu("Settings")]);

        assert_eq!(selected_open_target(&UiOutcome::Cancelled, &state), None);
        assert_eq!(selected_open_target(&UiOutcome::Selected(0), &state), None);
        assert_eq!(selected_open_target(&UiOutcome::Selected(99), &state), None);
    }

    #[test]
    fn record_atflow_open_at_respects_config_flag() {
        let db = HistoryDb::open_memory().unwrap();
        let mut config = Config::default();
        let path = Path::new("/tmp/project");

        config.history.record_atflow_opens = false;
        record_atflow_open_at(&db, &config, path, PathKind::Dir, 100).unwrap();
        assert!(db.recent_dirs(10).unwrap().is_empty());

        config.history.record_atflow_opens = true;
        record_atflow_open_at(&db, &config, path, PathKind::Dir, 200).unwrap();
        let recent = db.recent_dirs(10).unwrap();

        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].path, path);
        assert_eq!(recent[0].kind, PathKind::Dir);
        assert_eq!(recent[0].source, HistorySource::Atflow);
    }

    #[test]
    fn expand_home_expands_only_home_prefixes() {
        if let Some(home) = dirs::home_dir() {
            assert_eq!(expand_home("~"), home);
            assert_eq!(expand_home("~/work"), home.join("work"));
        }

        assert_eq!(expand_home("/tmp/project"), PathBuf::from("/tmp/project"));
        assert_eq!(
            expand_home("relative/project"),
            PathBuf::from("relative/project")
        );
    }

    #[test]
    fn setting_stream_uses_stderr_for_shell_mode() {
        assert_eq!(setting_stream(false), InfoStream::Stdout);
        assert_eq!(setting_stream(true), InfoStream::Stderr);
    }

    #[test]
    fn ensure_config_file_creates_default_config_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("at").join("config.toml");

        let config = ensure_config_file(&path).unwrap();

        assert_eq!(config, Config::default());
        assert_eq!(Config::load_or_default(&path).unwrap(), Config::default());
    }

    #[test]
    fn launch_opener_reports_nonzero_exit() {
        let error =
            launch_opener("false", Path::new("/tmp/atflow-opener-test"), false).unwrap_err();

        assert!(error.to_string().contains("opener `false` exited"));
    }

    #[test]
    fn shell_mode_openers_use_tty_stdio() {
        assert_eq!(opener_stdio(false), OpenerStdio::Inherit);
        assert_eq!(opener_stdio(true), OpenerStdio::Tty);
    }

    #[test]
    fn invocation_search_root_uses_current_directory_only() {
        let mut config = Config::default();
        config.search.root_mode = SearchRootMode::Invocation;
        config.search.roots = vec!["/configured".to_owned()];

        let roots = flow_search_roots_from(&config, PathBuf::from("/current"));

        assert_eq!(roots, vec![PathBuf::from("/current")]);
    }

    #[test]
    fn configured_search_roots_use_configured_paths_and_fall_back_to_current() {
        let mut config = Config::default();
        config.search.root_mode = SearchRootMode::Configured;
        config.search.roots = vec!["/configured".to_owned()];

        let roots = flow_search_roots_from(&config, PathBuf::from("/current"));

        assert_eq!(roots, vec![PathBuf::from("/configured")]);

        config.search.roots.clear();
        let roots = flow_search_roots_from(&config, PathBuf::from("/current"));

        assert_eq!(roots, vec![PathBuf::from("/current")]);
    }

    #[test]
    fn flow_data_recent_items_put_pinned_paths_first_and_deduplicate() {
        let db = HistoryDb::open_memory().unwrap();
        let config = Config::default();
        db.record_path_at(
            Path::new("/tmp/recent"),
            PathKind::Dir,
            HistorySource::Atflow,
            100,
        )
        .unwrap();
        db.record_path_at(
            Path::new("/tmp/pinned"),
            PathKind::Dir,
            HistorySource::Atflow,
            200,
        )
        .unwrap();
        db.toggle_pin_at(Path::new("/tmp/pinned"), PathKind::Dir, 300)
            .unwrap();
        let mut data = FlowData {
            config: &config,
            db: &db,
        };

        let items = data.recent_items().unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].path.as_deref(), Some(Path::new("/tmp/pinned")));
        assert!(items[0].pinned);
        assert_eq!(items[1].path.as_deref(), Some(Path::new("/tmp/recent")));
        assert!(!items[1].pinned);
    }

    #[test]
    fn search_items_deduplicates_results_from_overlapping_roots() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("project");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/needle.rs"), "").unwrap();

        let items = search_items(&[root.clone(), root], &[], "needle", SearchFilter::All).unwrap();

        assert_eq!(items.len(), 1);
        assert!(items[0].path.as_ref().unwrap().ends_with("needle.rs"));
    }
}
