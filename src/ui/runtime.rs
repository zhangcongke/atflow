use anyhow::Result;
use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::ops::Range;
#[cfg(unix)]
use std::os::fd::{AsRawFd, RawFd};
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::search::SearchFilter;
use crate::settings::SettingsState;
use crate::ui::palette::{PaletteItem, PaletteItemKind, PaletteState};
use crate::ui::theme::{PaletteTheme, ThemeName};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsOutcome {
    Saved(Config),
    Cancelled,
}

pub trait FlowDelegate {
    fn recent_items(&mut self) -> Result<Vec<PaletteItem>>;
    fn browse_items(&mut self, cwd: &Path) -> Result<Vec<PaletteItem>>;
    fn search_items(&mut self, query: &str, roots: &[PathBuf]) -> Result<Vec<PaletteItem>>;
    fn toggle_pin(&mut self, item: &PaletteItem) -> Result<()>;
    fn pinned_dirs(&mut self) -> Result<Vec<PathBuf>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnifiedFlowMode {
    Recent,
    Browse,
    Search,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UnifiedFlowState {
    cwd: PathBuf,
    search_roots: Vec<PathBuf>,
    pinned_root_index: Option<usize>,
    mode: UnifiedFlowMode,
    palette: PaletteState,
}

pub fn run_menu_palette(
    title: &str,
    mut state: PaletteState,
    theme_name: ThemeName,
) -> Result<UiOutcome> {
    let session = TerminalSession::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(session.output()?))?;
    terminal.clear()?;
    let theme = PaletteTheme::from(theme_name);

    loop {
        terminal.draw(|frame| render_palette(frame, title, &state, PaletteChrome::Menu, theme))?;
        if let Event::Key(key) = event::read()? {
            if !is_key_press(key) {
                continue;
            }
            if let Some(outcome) = handle_menu_key(&mut state, key) {
                return Ok(outcome);
            }
        }
    }
}

pub fn run_palette(
    title: &str,
    mut state: PaletteState,
    theme_name: ThemeName,
) -> Result<UiResponse> {
    let session = TerminalSession::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(session.output()?))?;
    terminal.clear()?;
    let theme = PaletteTheme::from(theme_name);

    loop {
        terminal.draw(|frame| render_palette(frame, title, &state, PaletteChrome::List, theme))?;
        if let Event::Key(key) = event::read()? {
            if !is_key_press(key) {
                continue;
            }
            if let Some(outcome) = handle_palette_key(&mut state, key) {
                return Ok(UiResponse { outcome, state });
            }
        }
    }
}

pub fn run_search_palette<F>(
    title: &str,
    mut state: PaletteState,
    mut refresh: F,
    theme_name: ThemeName,
) -> Result<UiResponse>
where
    F: FnMut(&str, SearchFilter) -> Result<Vec<PaletteItem>>,
{
    let _session = TerminalSession::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(_session.output()?))?;
    terminal.clear()?;
    let theme = PaletteTheme::from(theme_name);

    loop {
        terminal
            .draw(|frame| render_palette(frame, title, &state, PaletteChrome::Search, theme))?;
        if let Event::Key(key) = event::read()? {
            if !is_key_press(key) {
                continue;
            }
            if let Some(outcome) = handle_search_key(&mut state, key, &mut refresh)? {
                return Ok(UiResponse { outcome, state });
            }
        }
    }
}

pub fn run_flow_palette<D: FlowDelegate>(
    title: &str,
    start_dir: PathBuf,
    search_roots: Vec<PathBuf>,
    initial_query: Option<String>,
    theme_name: ThemeName,
    delegate: &mut D,
) -> Result<UiResponse> {
    let _session = TerminalSession::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(_session.output()?))?;
    terminal.clear()?;
    let mut state = init_unified_flow_state(start_dir, search_roots, initial_query, delegate)?;
    let theme = PaletteTheme::from(theme_name);

    loop {
        terminal.draw(|frame| {
            let chrome = if state.mode == UnifiedFlowMode::Search {
                PaletteChrome::FlowSearch
            } else {
                PaletteChrome::Flow
            };
            render_palette(frame, title, &state.palette, chrome, theme)
        })?;
        if let Event::Key(key) = event::read()? {
            if !is_key_press(key) {
                continue;
            }
            if let Some(outcome) = handle_unified_flow_key(&mut state, key, delegate)? {
                return Ok(UiResponse {
                    outcome,
                    state: state.palette,
                });
            }
        }
    }
}

pub fn run_settings_palette(config: Config) -> Result<SettingsOutcome> {
    let _session = TerminalSession::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(_session.output()?))?;
    terminal.clear()?;
    let mut settings = SettingsState::new(config);

    loop {
        let state = settings_palette_state(&settings);
        let theme = PaletteTheme::from(settings.config().general.theme);
        terminal.draw(|frame| {
            render_palette(frame, "@ Setting", &state, PaletteChrome::Settings, theme)
        })?;
        if let Event::Key(key) = event::read()? {
            if !is_key_press(key) {
                continue;
            }
            if let Some(outcome) = handle_settings_key(&mut settings, key) {
                return Ok(outcome);
            }
        }
    }
}

fn handle_menu_key(state: &mut PaletteState, key: KeyEvent) -> Option<UiOutcome> {
    match key {
        KeyEvent {
            code: KeyCode::Esc, ..
        } => Some(UiOutcome::Cancelled),
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => state.selected_index().map(UiOutcome::Selected),
        KeyEvent {
            code: KeyCode::Up, ..
        } => {
            state.move_up();
            None
        }
        KeyEvent {
            code: KeyCode::Down,
            ..
        } => {
            state.move_down();
            None
        }
        _ => None,
    }
}

fn handle_palette_key(state: &mut PaletteState, key: KeyEvent) -> Option<UiOutcome> {
    match key {
        KeyEvent {
            code: KeyCode::Esc, ..
        } => Some(UiOutcome::Cancelled),
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => state.selected_index().map(UiOutcome::Selected),
        KeyEvent {
            code: KeyCode::Up, ..
        } => {
            state.move_up();
            None
        }
        KeyEvent {
            code: KeyCode::Down,
            ..
        } => {
            state.move_down();
            None
        }
        KeyEvent {
            code: KeyCode::Char(' '),
            modifiers,
            ..
        } if is_plain_text_modifier(modifiers) => {
            state.toggle_expanded();
            None
        }
        _ => None,
    }
}

fn handle_search_key<F>(
    state: &mut PaletteState,
    key: KeyEvent,
    mut refresh: F,
) -> Result<Option<UiOutcome>>
where
    F: FnMut(&str, SearchFilter) -> Result<Vec<PaletteItem>>,
{
    let outcome = match key {
        KeyEvent {
            code: KeyCode::Tab, ..
        } => {
            state.cycle_filter();
            refresh_items(state, &mut refresh)?;
            None
        }
        KeyEvent {
            code: KeyCode::Backspace,
            ..
        } => {
            state.query.pop();
            refresh_items(state, &mut refresh)?;
            None
        }
        KeyEvent {
            code: KeyCode::Char(' '),
            modifiers,
            ..
        } if is_plain_text_modifier(modifiers) => {
            state.toggle_expanded();
            None
        }
        KeyEvent {
            code: KeyCode::Char(ch),
            modifiers,
            ..
        } if is_plain_text_modifier(modifiers) => {
            state.query.push(ch);
            refresh_items(state, &mut refresh)?;
            None
        }
        _ => handle_palette_key(state, key),
    };
    Ok(outcome)
}

fn init_unified_flow_state<D: FlowDelegate>(
    start_dir: PathBuf,
    mut search_roots: Vec<PathBuf>,
    initial_query: Option<String>,
    delegate: &mut D,
) -> Result<UnifiedFlowState> {
    if search_roots.is_empty() {
        search_roots.push(start_dir.clone());
    }
    let mut state = UnifiedFlowState {
        cwd: start_dir,
        search_roots,
        pinned_root_index: None,
        mode: UnifiedFlowMode::Recent,
        palette: PaletteState::new(Vec::new()),
    };
    if let Some(query) = initial_query.filter(|query| !query.trim().is_empty()) {
        state.mode = UnifiedFlowMode::Search;
        state.palette.query = query;
        refresh_unified_flow(&mut state, delegate, None)?;
    } else {
        refresh_unified_flow(&mut state, delegate, None)?;
    }
    Ok(state)
}

fn handle_unified_flow_key<D: FlowDelegate>(
    state: &mut UnifiedFlowState,
    key: KeyEvent,
    delegate: &mut D,
) -> Result<Option<UiOutcome>> {
    let outcome = match key {
        KeyEvent {
            code: KeyCode::Esc, ..
        } => Some(UiOutcome::Cancelled),
        KeyEvent {
            code: KeyCode::Up, ..
        } => {
            state.palette.move_up();
            None
        }
        KeyEvent {
            code: KeyCode::Down,
            ..
        } => {
            state.palette.move_down();
            None
        }
        KeyEvent {
            code: KeyCode::BackTab,
            ..
        } => {
            cycle_to_next_pinned_dir(state, delegate)?;
            None
        }
        KeyEvent {
            code: KeyCode::Tab,
            modifiers,
            ..
        } if modifiers.contains(KeyModifiers::SHIFT) => {
            cycle_to_next_pinned_dir(state, delegate)?;
            None
        }
        KeyEvent {
            code: KeyCode::Tab, ..
        } => {
            if let Some(item) = state.palette.selected_item().cloned() {
                delegate.toggle_pin(&item)?;
                refresh_unified_flow(state, delegate, item.path.as_deref())?;
            }
            None
        }
        KeyEvent {
            code: KeyCode::Left,
            ..
        } => {
            browse_parent(state, delegate)?;
            None
        }
        KeyEvent {
            code: KeyCode::Right,
            ..
        } => {
            enter_selected_dir(state, delegate)?;
            None
        }
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => {
            if selected_item_is_dir(state) {
                enter_selected_dir(state, delegate)?;
                None
            } else {
                state.palette.selected_index().map(UiOutcome::Selected)
            }
        }
        KeyEvent {
            code: KeyCode::Backspace,
            ..
        } => {
            state.palette.query.pop();
            if state.palette.query.is_empty() {
                state.mode = UnifiedFlowMode::Recent;
            } else {
                state.mode = UnifiedFlowMode::Search;
            }
            refresh_unified_flow(state, delegate, None)?;
            None
        }
        KeyEvent {
            code: KeyCode::Char(' '),
            modifiers,
            ..
        } if is_plain_text_modifier(modifiers) => {
            state.palette.toggle_expanded();
            None
        }
        KeyEvent {
            code: KeyCode::Char(ch),
            modifiers,
            ..
        } if is_plain_text_modifier(modifiers) => {
            state.mode = UnifiedFlowMode::Search;
            state.palette.query.push(ch);
            refresh_unified_flow(state, delegate, None)?;
            None
        }
        _ => None,
    };
    Ok(outcome)
}

fn refresh_unified_flow<D: FlowDelegate>(
    state: &mut UnifiedFlowState,
    delegate: &mut D,
    preferred_path: Option<&Path>,
) -> Result<()> {
    let items = match state.mode {
        UnifiedFlowMode::Recent => delegate.recent_items()?,
        UnifiedFlowMode::Browse => delegate.browse_items(&state.cwd)?,
        UnifiedFlowMode::Search => {
            delegate.search_items(&state.palette.query, &state.search_roots)?
        }
    };
    let query = state.palette.query.clone();
    state.palette.replace_items(items);
    state.palette.query = query;
    if let Some(path) = preferred_path
        && let Some(index) = state
            .palette
            .items
            .iter()
            .position(|item| item.path.as_deref() == Some(path))
    {
        state.palette.selected = index;
    }
    Ok(())
}

fn selected_item_is_dir(state: &UnifiedFlowState) -> bool {
    state
        .palette
        .selected_item()
        .is_some_and(|item| matches!(item.kind, PaletteItemKind::Dir))
}

fn enter_selected_dir<D: FlowDelegate>(
    state: &mut UnifiedFlowState,
    delegate: &mut D,
) -> Result<()> {
    let Some(item) = state.palette.selected_item().cloned() else {
        return Ok(());
    };
    if !matches!(item.kind, PaletteItemKind::Dir) {
        return Ok(());
    }
    let Some(path) = item.path else {
        return Ok(());
    };
    state.cwd = path;
    state.mode = UnifiedFlowMode::Browse;
    state.palette.query.clear();
    refresh_unified_flow(state, delegate, None)
}

fn browse_parent<D: FlowDelegate>(state: &mut UnifiedFlowState, delegate: &mut D) -> Result<()> {
    let previous = state.cwd.clone();
    let Some(parent) = state.cwd.parent() else {
        return Ok(());
    };
    state.cwd = parent.to_path_buf();
    state.mode = UnifiedFlowMode::Browse;
    state.palette.query.clear();
    refresh_unified_flow(state, delegate, Some(&previous))
}

fn cycle_to_next_pinned_dir<D: FlowDelegate>(
    state: &mut UnifiedFlowState,
    delegate: &mut D,
) -> Result<()> {
    let dirs = delegate.pinned_dirs()?;
    if dirs.is_empty() {
        return Ok(());
    }
    let index = state
        .pinned_root_index
        .map_or(0, |index| (index + 1) % dirs.len());
    state.pinned_root_index = Some(index);
    let root = dirs[index].clone();
    state.cwd = root.clone();
    state.search_roots = vec![root];
    state.mode = UnifiedFlowMode::Browse;
    state.palette.query.clear();
    refresh_unified_flow(state, delegate, None)
}

fn handle_settings_key(settings: &mut SettingsState, key: KeyEvent) -> Option<SettingsOutcome> {
    match key {
        KeyEvent {
            code: KeyCode::Esc, ..
        } => Some(SettingsOutcome::Cancelled),
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => Some(SettingsOutcome::Saved(settings.clone().into_config())),
        KeyEvent {
            code: KeyCode::Up, ..
        } => {
            settings.move_up();
            None
        }
        KeyEvent {
            code: KeyCode::Down,
            ..
        } => {
            settings.move_down();
            None
        }
        KeyEvent {
            code: KeyCode::Left,
            ..
        } => {
            settings.change_left();
            None
        }
        KeyEvent {
            code: KeyCode::Right,
            ..
        } => {
            settings.change_right();
            None
        }
        _ => None,
    }
}

fn settings_palette_state(settings: &SettingsState) -> PaletteState {
    let mut state = PaletteState::new(settings.palette_items());
    state.selected = settings.selected();
    state
}

fn refresh_items<F>(state: &mut PaletteState, refresh: &mut F) -> Result<()>
where
    F: FnMut(&str, SearchFilter) -> Result<Vec<PaletteItem>>,
{
    let items = refresh(&state.query, state.filter)?;
    state.replace_items(items);
    Ok(())
}

fn is_key_press(key: KeyEvent) -> bool {
    matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
}

fn is_plain_text_modifier(modifiers: KeyModifiers) -> bool {
    !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaletteChrome {
    Menu,
    List,
    Search,
    FlowSearch,
    Flow,
    Settings,
}

fn render_palette(
    frame: &mut Frame<'_>,
    title: &str,
    state: &PaletteState,
    chrome: PaletteChrome,
    theme: PaletteTheme,
) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new(header_line(title, state, chrome, theme))
            .block(Block::default().borders(Borders::ALL)),
        chunks[0],
    );

    let list_width = usize::from(chunks[1].width.saturating_sub(2));
    let list_height = usize::from(chunks[1].height);
    let rows = palette_rows(state, list_width, list_height, theme);
    frame.render_widget(
        List::new(rows).block(Block::default().borders(Borders::LEFT | Borders::RIGHT)),
        chunks[1],
    );

    frame.render_widget(
        Paragraph::new(footer_text(chrome)).block(Block::default().borders(Borders::ALL)),
        chunks[2],
    );
}

fn header_line(
    title: &str,
    state: &PaletteState,
    chrome: PaletteChrome,
    theme: PaletteTheme,
) -> Line<'static> {
    let title_span = Span::styled(title.to_owned(), theme.title_style());
    let show_query = matches!(chrome, PaletteChrome::Search | PaletteChrome::FlowSearch);
    if !show_query {
        return Line::from(title_span);
    }

    let query = if state.query.is_empty() {
        "<empty>"
    } else {
        state.query.as_str()
    };
    if chrome == PaletteChrome::FlowSearch {
        return Line::from(vec![
            title_span,
            Span::raw("  query: "),
            Span::styled(query.to_owned(), theme.query_style()),
        ]);
    }

    Line::from(vec![
        title_span,
        Span::raw("  query: "),
        Span::styled(query.to_owned(), theme.query_style()),
        Span::raw("  filter: "),
        Span::styled(filter_label(state.filter), theme.filter_style()),
    ])
}

fn footer_text(chrome: PaletteChrome) -> &'static str {
    match chrome {
        PaletteChrome::Menu => menu_footer_text(),
        PaletteChrome::List => list_footer_text(),
        PaletteChrome::Search => search_footer_text(),
        PaletteChrome::FlowSearch => flow_footer_text(),
        PaletteChrome::Flow => flow_footer_text(),
        PaletteChrome::Settings => settings_footer_text(),
    }
}

fn menu_footer_text() -> &'static str {
    "Esc cancel  Enter select  Up/Down move"
}

fn list_footer_text() -> &'static str {
    "Esc cancel  Enter open  Up/Down move  Space expand"
}

fn search_footer_text() -> &'static str {
    "Esc cancel  Enter open  Up/Down move  Space expand  Tab filter"
}

fn flow_footer_text() -> &'static str {
    "Esc cancel  Enter open  Up/Down move  Left/Right navigate  Space expand  Tab pin  Shift+Tab pinned root"
}

fn settings_footer_text() -> &'static str {
    "Esc cancel  Enter save  Up/Down move  Left/Right change"
}

fn palette_rows(
    state: &PaletteState,
    area_width: usize,
    area_height: usize,
    theme: PaletteTheme,
) -> Vec<ListItem<'static>> {
    if state.items.is_empty() {
        return vec![ListItem::new("  No results").style(theme.muted_style())];
    }
    if area_height == 0 {
        return Vec::new();
    }

    let selected = state.selected_index();
    let visible_range = visible_item_range(state, area_height);
    state
        .items
        .iter()
        .take(visible_range.end)
        .skip(visible_range.start)
        .enumerate()
        .map(|(offset, item)| {
            let index = visible_range.start + offset;
            let marker = if Some(index) == selected { ">" } else { " " };
            let suffix = palette_row_suffix(item);
            let suffix_width = if suffix.is_empty() {
                0
            } else {
                suffix.chars().count() + 2
            };
            let label_width = area_width.saturating_sub(suffix_width + 2).max(1);
            let label = state
                .display_label_at(index, label_width)
                .unwrap_or_else(|| item.label.clone());
            let style = if Some(index) == selected {
                theme.selected_style()
            } else {
                Default::default()
            };

            let mut spans = vec![Span::raw(format!("{marker} ")), Span::raw(label)];
            if !suffix.is_empty() {
                spans.push(Span::styled(format!("  {suffix}"), theme.muted_style()));
            }

            ListItem::new(Line::from(spans)).style(style)
        })
        .collect()
}

fn visible_item_range(state: &PaletteState, max_rows: usize) -> Range<usize> {
    if state.items.is_empty() || max_rows == 0 {
        return 0..0;
    }

    let len = state.items.len();
    let selected = state.selected_index().unwrap_or(0);
    if len <= max_rows {
        return 0..len;
    }

    let start = selected.saturating_add(1).saturating_sub(max_rows);
    start..(start + max_rows)
}

fn palette_row_suffix(item: &PaletteItem) -> String {
    let kind = palette_row_kind_text(item);
    match (item.pinned, kind) {
        (true, Some(kind)) => format!("pinned {kind}"),
        (true, None) => "pinned".to_owned(),
        (false, Some(kind)) => kind.to_owned(),
        (false, None) => String::new(),
    }
}

fn palette_row_kind_text(item: &PaletteItem) -> Option<&'static str> {
    match item.kind {
        PaletteItemKind::Menu => None,
        PaletteItemKind::Dir => Some("dir"),
        PaletteItemKind::File => Some("file"),
    }
}

fn filter_label(filter: SearchFilter) -> &'static str {
    match filter {
        SearchFilter::All => "all",
        SearchFilter::Dirs => "dirs",
        SearchFilter::Files => "files",
    }
}

struct TerminalSession {
    tty: File,
    #[cfg(unix)]
    original_stdout_fd: RawFd,
}

impl TerminalSession {
    fn enter() -> Result<Self> {
        let tty = OpenOptions::new().read(true).write(true).open("/dev/tty")?;
        #[cfg(unix)]
        let original_stdout_fd = redirect_stdout_to(&tty)?;

        if let Err(error) = enable_raw_mode() {
            #[cfg(unix)]
            restore_stdout(original_stdout_fd);
            return Err(error.into());
        }
        if let Err(error) = execute!(&tty, EnterAlternateScreen) {
            restore_terminal(&tty);
            #[cfg(unix)]
            restore_stdout(original_stdout_fd);
            return Err(error.into());
        }
        if let Err(error) = execute!(&tty, Hide) {
            restore_terminal(&tty);
            #[cfg(unix)]
            restore_stdout(original_stdout_fd);
            return Err(error.into());
        }
        Ok(Self {
            tty,
            #[cfg(unix)]
            original_stdout_fd,
        })
    }

    fn output(&self) -> io::Result<File> {
        self.tty.try_clone()
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        restore_terminal(&self.tty);
        #[cfg(unix)]
        restore_stdout(self.original_stdout_fd);
    }
}

fn restore_terminal(mut output: impl Write) {
    let _ = execute!(output, Show, LeaveAlternateScreen);
    let _ = disable_raw_mode();
}

#[cfg(unix)]
fn redirect_stdout_to(file: &File) -> io::Result<RawFd> {
    let stdout_fd = io::stdout().as_raw_fd();
    let original_stdout_fd = unsafe { libc::dup(stdout_fd) };
    if original_stdout_fd < 0 {
        return Err(io::Error::last_os_error());
    }

    if unsafe { libc::dup2(file.as_raw_fd(), stdout_fd) } < 0 {
        let error = io::Error::last_os_error();
        unsafe {
            libc::close(original_stdout_fd);
        }
        return Err(error);
    }

    Ok(original_stdout_fd)
}

#[cfg(unix)]
fn restore_stdout(original_stdout_fd: RawFd) {
    let stdout_fd = io::stdout().as_raw_fd();
    unsafe {
        libc::dup2(original_stdout_fd, stdout_fd);
        libc::close(original_stdout_fd);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::palette::{PaletteItem, PaletteItemKind};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    fn item(label: &str) -> PaletteItem {
        PaletteItem {
            label: label.to_owned(),
            path: None,
            kind: PaletteItemKind::Menu,
            source: "test".to_owned(),
            pinned: false,
        }
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl_key(ch: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL)
    }

    fn backtab_key() -> KeyEvent {
        KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT)
    }

    fn dir_item(path: &str) -> PaletteItem {
        PaletteItem::dir(PathBuf::from(path), "test")
    }

    fn file_item(path: &str) -> PaletteItem {
        PaletteItem::file(PathBuf::from(path), "test")
    }

    #[derive(Default)]
    struct FakeFlowDelegate {
        recent: Vec<PaletteItem>,
        browse: HashMap<PathBuf, Vec<PaletteItem>>,
        search: HashMap<String, Vec<PaletteItem>>,
        toggled: Vec<PathBuf>,
        pinned_dirs: Vec<PathBuf>,
    }

    impl FlowDelegate for FakeFlowDelegate {
        fn recent_items(&mut self) -> Result<Vec<PaletteItem>> {
            Ok(self.recent.clone())
        }

        fn browse_items(&mut self, cwd: &Path) -> Result<Vec<PaletteItem>> {
            Ok(self.browse.get(cwd).cloned().unwrap_or_default())
        }

        fn search_items(&mut self, query: &str, _roots: &[PathBuf]) -> Result<Vec<PaletteItem>> {
            Ok(self.search.get(query).cloned().unwrap_or_default())
        }

        fn toggle_pin(&mut self, item: &PaletteItem) -> Result<()> {
            if let Some(path) = item.path.clone() {
                self.toggled.push(path);
            }
            Ok(())
        }

        fn pinned_dirs(&mut self) -> Result<Vec<PathBuf>> {
            Ok(self.pinned_dirs.clone())
        }
    }

    #[test]
    fn menu_enter_on_empty_has_no_outcome() {
        let mut state = PaletteState::new(vec![]);

        let outcome = handle_menu_key(&mut state, key(KeyCode::Enter));

        assert_eq!(outcome, None);
    }

    #[test]
    fn list_shortcuts_do_not_handle_search_or_forced_open_keys() {
        let mut state = PaletteState::new(vec![item("one")]);

        let tab = handle_palette_key(&mut state, key(KeyCode::Tab));
        let typed = handle_palette_key(&mut state, key(KeyCode::Char('x')));
        let editor = handle_palette_key(&mut state, ctrl_key('e'));
        let system = handle_palette_key(&mut state, ctrl_key('o'));

        assert_eq!(tab, None);
        assert_eq!(typed, None);
        assert_eq!(editor, None);
        assert_eq!(system, None);
        assert_eq!(state.query, "");
        assert_eq!(state.filter, SearchFilter::All);
    }

    #[test]
    fn enter_on_non_empty_selects_current_selected_index() {
        let mut state = PaletteState::new(vec![item("one"), item("two")]);
        state.move_down();

        let outcome = handle_palette_key(&mut state, key(KeyCode::Enter));

        assert_eq!(outcome, Some(UiOutcome::Selected(1)));
    }

    #[test]
    fn space_toggles_expansion() {
        let mut state = PaletteState::new(vec![item("one")]);

        let outcome = handle_palette_key(&mut state, key(KeyCode::Char(' ')));

        assert_eq!(outcome, None);
        assert!(state.expanded);
    }

    #[test]
    fn search_typing_refreshes_items_while_preserving_query() {
        let mut state = PaletteState::new(vec![]);
        let mut calls = Vec::new();

        let outcome = handle_search_key(&mut state, key(KeyCode::Char('x')), |query, filter| {
            calls.push((query.to_owned(), filter));
            Ok(vec![item("refreshed")])
        })
        .unwrap();

        assert_eq!(outcome, None);
        assert_eq!(state.query, "x");
        assert_eq!(state.items, vec![item("refreshed")]);
        assert_eq!(calls, vec![("x".to_owned(), SearchFilter::All)]);
    }

    #[test]
    fn search_tab_cycles_filter_and_refreshes() {
        let mut state = PaletteState::new(vec![]);
        state.query = "abc".to_owned();
        let mut calls = Vec::new();

        let outcome = handle_search_key(&mut state, key(KeyCode::Tab), |query, filter| {
            calls.push((query.to_owned(), filter));
            Ok(vec![item("dirs")])
        })
        .unwrap();

        assert_eq!(outcome, None);
        assert_eq!(state.query, "abc");
        assert_eq!(state.filter, SearchFilter::Dirs);
        assert_eq!(state.items, vec![item("dirs")]);
        assert_eq!(calls, vec![("abc".to_owned(), SearchFilter::Dirs)]);
    }

    #[test]
    fn search_refresh_errors_are_propagated() {
        let mut state = PaletteState::new(vec![]);

        let error = handle_search_key(&mut state, key(KeyCode::Char('x')), |_query, _filter| {
            Err(anyhow::anyhow!("search unavailable"))
        })
        .unwrap_err();

        assert_eq!(error.to_string(), "search unavailable");
    }

    #[test]
    fn unified_flow_initial_query_enters_search_mode() {
        let start = PathBuf::from("/tmp/start");
        let mut delegate = FakeFlowDelegate::default();
        delegate
            .search
            .insert("abc".to_owned(), vec![file_item("/tmp/start/abc.rs")]);

        let state = init_unified_flow_state(
            start.clone(),
            vec![start],
            Some("abc".to_owned()),
            &mut delegate,
        )
        .unwrap();

        assert_eq!(state.mode, UnifiedFlowMode::Search);
        assert_eq!(state.palette.query, "abc");
        assert_eq!(
            state.palette.selected_item().unwrap().path.as_deref(),
            Some(Path::new("/tmp/start/abc.rs"))
        );
    }

    #[test]
    fn unified_flow_typing_searches_from_recent_mode() {
        let start = PathBuf::from("/tmp/start");
        let mut delegate = FakeFlowDelegate {
            recent: vec![dir_item("/tmp/recent")],
            ..Default::default()
        };
        delegate
            .search
            .insert("x".to_owned(), vec![file_item("/tmp/start/x.rs")]);
        let mut state =
            init_unified_flow_state(start.clone(), vec![start], None, &mut delegate).unwrap();

        let outcome =
            handle_unified_flow_key(&mut state, key(KeyCode::Char('x')), &mut delegate).unwrap();

        assert_eq!(outcome, None);
        assert_eq!(state.mode, UnifiedFlowMode::Search);
        assert_eq!(state.palette.query, "x");
        assert_eq!(
            state.palette.selected_item().unwrap().path.as_deref(),
            Some(Path::new("/tmp/start/x.rs"))
        );
    }

    #[test]
    fn unified_flow_enter_on_directory_browses_instead_of_exiting() {
        let start = PathBuf::from("/tmp/start");
        let src = PathBuf::from("/tmp/start/src");
        let mut delegate = FakeFlowDelegate {
            recent: vec![PaletteItem::dir(src.clone(), "recent")],
            ..Default::default()
        };
        delegate
            .browse
            .insert(src.clone(), vec![file_item("/tmp/start/src/main.rs")]);
        let mut state =
            init_unified_flow_state(start.clone(), vec![start], None, &mut delegate).unwrap();

        let outcome =
            handle_unified_flow_key(&mut state, key(KeyCode::Enter), &mut delegate).unwrap();

        assert_eq!(outcome, None);
        assert_eq!(state.mode, UnifiedFlowMode::Browse);
        assert_eq!(state.cwd, src);
        assert_eq!(state.palette.query, "");
        assert_eq!(
            state.palette.selected_item().unwrap().path.as_deref(),
            Some(Path::new("/tmp/start/src/main.rs"))
        );
    }

    #[test]
    fn unified_flow_enter_on_file_selects_it() {
        let start = PathBuf::from("/tmp/start");
        let mut delegate = FakeFlowDelegate {
            recent: vec![file_item("/tmp/start/readme.md")],
            ..Default::default()
        };
        let mut state =
            init_unified_flow_state(start.clone(), vec![start], None, &mut delegate).unwrap();

        let outcome =
            handle_unified_flow_key(&mut state, key(KeyCode::Enter), &mut delegate).unwrap();

        assert_eq!(outcome, Some(UiOutcome::Selected(0)));
    }

    #[test]
    fn unified_flow_tab_toggles_pin_and_preserves_selection() {
        let start = PathBuf::from("/tmp/start");
        let target = PathBuf::from("/tmp/recent/two");
        let mut delegate = FakeFlowDelegate {
            recent: vec![
                dir_item("/tmp/recent/one"),
                PaletteItem::dir(target.clone(), "recent"),
            ],
            ..Default::default()
        };
        let mut state =
            init_unified_flow_state(start.clone(), vec![start], None, &mut delegate).unwrap();
        state.palette.selected = 1;

        let outcome =
            handle_unified_flow_key(&mut state, key(KeyCode::Tab), &mut delegate).unwrap();

        assert_eq!(outcome, None);
        assert_eq!(delegate.toggled, vec![target.clone()]);
        assert_eq!(state.palette.selected, 1);
        assert_eq!(
            state.palette.selected_item().unwrap().path.as_deref(),
            Some(target.as_path())
        );
    }

    #[test]
    fn unified_flow_backtab_cycles_to_pinned_dirs() {
        let start = PathBuf::from("/tmp/start");
        let first = PathBuf::from("/tmp/pinned/one");
        let second = PathBuf::from("/tmp/pinned/two");
        let mut delegate = FakeFlowDelegate {
            recent: vec![dir_item("/tmp/recent")],
            pinned_dirs: vec![first.clone(), second.clone()],
            ..Default::default()
        };
        delegate
            .browse
            .insert(first.clone(), vec![file_item("/tmp/pinned/one/a.txt")]);
        delegate
            .browse
            .insert(second.clone(), vec![file_item("/tmp/pinned/two/b.txt")]);
        let mut state =
            init_unified_flow_state(start.clone(), vec![start], None, &mut delegate).unwrap();

        handle_unified_flow_key(&mut state, backtab_key(), &mut delegate).unwrap();
        assert_eq!(state.cwd, first);
        assert_eq!(state.search_roots, vec![PathBuf::from("/tmp/pinned/one")]);

        handle_unified_flow_key(&mut state, backtab_key(), &mut delegate).unwrap();
        assert_eq!(state.cwd, second);
        assert_eq!(state.search_roots, vec![PathBuf::from("/tmp/pinned/two")]);
    }

    #[test]
    fn palette_row_shows_item_kind_instead_of_source() {
        let state = PaletteState::new(vec![
            PaletteItem::menu("Settings"),
            PaletteItem::dir(PathBuf::from("/tmp/project"), "flow"),
            PaletteItem::file(PathBuf::from("/tmp/project/main.rs"), "search"),
        ]);

        assert_eq!(palette_row_kind_text(&state.items[0]), None);
        assert_eq!(palette_row_kind_text(&state.items[1]), Some("dir"));
        assert_eq!(palette_row_kind_text(&state.items[2]), Some("file"));
    }

    #[test]
    fn palette_rows_scroll_to_keep_selected_item_visible() {
        let mut state =
            PaletteState::new((0..8).map(|index| item(&format!("item-{index}"))).collect());
        state.selected = 7;

        assert_eq!(visible_item_range(&state, 3), 5..8);
    }

    #[test]
    fn flow_footer_mentions_navigation_keys() {
        let footer = flow_footer_text();

        assert!(footer.contains("Left/Right"));
        assert!(footer.contains("Tab pin"));
        assert!(footer.contains("Shift+Tab"));
        assert!(!footer.contains("h/l"));
        assert!(!footer.contains("Tab filter"));
        assert!(!footer.contains("Ctrl+E"));
        assert!(!footer.contains("Ctrl+O"));
    }

    #[test]
    fn menu_header_and_footer_are_not_search_controls() {
        let state = PaletteState::new(vec![item("Settings")]);
        let header = header_line(
            "@ Menu",
            &state,
            PaletteChrome::Menu,
            PaletteTheme::from(ThemeName::Mist),
        )
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

        assert_eq!(header, "@ Menu");
        assert!(!menu_footer_text().contains("Tab filter"));
        assert!(!menu_footer_text().contains("Space expand"));
    }

    #[test]
    fn search_header_uses_selected_theme_colors() {
        let mut state = PaletteState::new(vec![item("Settings")]);
        state.query = "abc".to_owned();
        let theme = PaletteTheme::from(ThemeName::Paper);

        let header = header_line("@search", &state, PaletteChrome::Search, theme);

        assert_eq!(header.spans[2].style.fg, Some(theme.query_fg));
        assert_eq!(header.spans[4].style.fg, Some(theme.filter_fg));
    }

    #[test]
    fn flow_search_header_omits_filter_controls() {
        let mut state = PaletteState::new(vec![item("Settings")]);
        state.query = "abc".to_owned();
        let theme = PaletteTheme::from(ThemeName::Paper);

        let header = header_line("@ Flow", &state, PaletteChrome::FlowSearch, theme);
        let text = header
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert_eq!(text, "@ Flow  query: abc");
        assert_eq!(header.spans[2].style.fg, Some(theme.query_fg));
        assert!(!text.contains("filter"));
    }

    #[test]
    fn settings_key_handler_changes_and_returns_saved_config() {
        let mut settings = SettingsState::new(Config::default());

        assert_eq!(
            handle_settings_key(&mut settings, key(KeyCode::Right)),
            None
        );
        let Some(SettingsOutcome::Saved(saved)) =
            handle_settings_key(&mut settings, key(KeyCode::Enter))
        else {
            panic!("expected saved config");
        };

        assert_eq!(saved.general.theme, crate::ui::theme::ThemeName::Ink);
    }

    #[test]
    fn settings_escape_cancels_without_returning_config() {
        let mut settings = SettingsState::new(Config::default());

        let outcome = handle_settings_key(&mut settings, key(KeyCode::Esc));

        assert_eq!(outcome, Some(SettingsOutcome::Cancelled));
    }
}
