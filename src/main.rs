use anyhow::{Context, Result};
use at::cli::{Cli, Command, ShellCommand};
use at::config::{Config, default_config_path};
use at::history::{HistoryDb, HistorySource, PathKind, default_history_path};
use at::open::{OpenAction, OpenMode, resolve_open_action};
use at::search::{SearchFilter, SearchRequest, search};
use at::ui::palette::{PaletteItem, PaletteItemKind, PaletteState};
use at::ui::runtime::{UiOutcome, run_palette, run_search_palette};
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

fn run_menu(shell: bool) -> Result<()> {
    let state = PaletteState::new(vec![
        PaletteItem::menu("Recent projects"),
        PaletteItem::menu("Flow navigator"),
        PaletteItem::menu("Search files"),
        PaletteItem::menu("Settings"),
    ]);

    match run_palette("@", state)? {
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
    let items = db
        .recent_dirs(config.general.max_recent)?
        .into_iter()
        .map(|entry| PaletteItem::dir(entry.path, entry.source.as_str()))
        .collect();

    handle_palette_result("@recent", PaletteState::new(items), shell, &config)
}

fn run_flow(shell: bool) -> Result<()> {
    let config = load_config()?;
    let start = at::flow::flow_start(
        &std::env::current_dir()?,
        config.general.start_from_git_root,
    );
    let items = at::flow::list_entries(&start)?
        .into_iter()
        .map(|entry| {
            if entry.is_dir {
                PaletteItem::dir(entry.path, "flow")
            } else {
                PaletteItem::file(entry.path, "flow")
            }
        })
        .collect();

    handle_palette_result("@flow", PaletteState::new(items), shell, &config)
}

fn run_search(shell: bool, query: Option<String>) -> Result<()> {
    let config = load_config()?;
    let roots = search_roots(&config)?;
    let refresh = |query_text: &str, filter: SearchFilter| {
        search_items(&roots, &config.search.ignore, query_text, filter)
    };
    let initial_query = query.unwrap_or_default();
    let mut state = PaletteState::new(refresh(&initial_query, SearchFilter::All)?);
    state.query = initial_query;

    let response = run_search_palette("@search", state, refresh)?;
    handle_open_outcome(&response.outcome, &response.state, shell, &config)
}

fn search_roots(config: &Config) -> Result<Vec<PathBuf>> {
    let mut roots = vec![std::env::current_dir()?];
    roots.extend(config.search.roots.iter().map(|root| expand_home(root)));
    Ok(roots)
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
}

fn handle_palette_result(
    title: &str,
    state: PaletteState,
    shell: bool,
    config: &Config,
) -> Result<()> {
    let outcome = run_palette(title, state.clone())?;
    handle_open_outcome(&outcome, &state, shell, config)
}

fn handle_open_outcome(
    outcome: &UiOutcome,
    state: &PaletteState,
    shell: bool,
    config: &Config,
) -> Result<()> {
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
        OpenAction::Editor { command, path } | OpenAction::System { command, path } => {
            std::process::Command::new(&command)
                .arg(&path)
                .status()
                .with_context(|| format!("failed to launch opener `{command}`"))?;
            record_atflow_open(&path, PathKind::File, config)?;
        }
    }
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

fn run_setting() -> Result<()> {
    println!("{}", default_config_path().display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use at::config::Config;
    use at::history::{HistoryDb, HistorySource, PathKind};
    use at::open::OpenMode;
    use at::ui::palette::{PaletteItem, PaletteState};
    use at::ui::runtime::UiOutcome;
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
}
