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

use crate::config::Config;
use crate::flow::{FlowEntry, FlowState};
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

pub fn run_flow_palette(
    title: &str,
    mut flow: FlowState,
    theme_name: ThemeName,
) -> Result<UiResponse> {
    let _session = TerminalSession::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(_session.output()?))?;
    terminal.clear()?;
    let mut state = flow_palette_state(&flow)?;
    let theme = PaletteTheme::from(theme_name);

    loop {
        terminal.draw(|frame| render_palette(frame, title, &state, PaletteChrome::Flow, theme))?;
        if let Event::Key(key) = event::read()? {
            if !is_key_press(key) {
                continue;
            }
            if let Some(outcome) = handle_flow_key(&mut flow, &mut state, key)? {
                return Ok(UiResponse { outcome, state });
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

fn handle_flow_key(
    flow: &mut FlowState,
    state: &mut PaletteState,
    key: KeyEvent,
) -> Result<Option<UiOutcome>> {
    let outcome = match key {
        KeyEvent {
            code: KeyCode::Left,
            ..
        } => {
            flow.parent()?;
            *state = flow_palette_state(flow)?;
            None
        }
        KeyEvent {
            code: KeyCode::Char('h' | 'H'),
            modifiers,
            ..
        } if is_plain_text_modifier(modifiers) => {
            flow.parent()?;
            *state = flow_palette_state(flow)?;
            None
        }
        KeyEvent {
            code: KeyCode::Right,
            ..
        } => {
            let before = flow.cwd.clone();
            if let Some(entry) = selected_flow_entry(state) {
                flow.enter(&entry);
            }
            if flow.cwd != before {
                *state = flow_palette_state(flow)?;
            } else {
                sync_flow_selection(flow, state);
            }
            None
        }
        KeyEvent {
            code: KeyCode::Char('l' | 'L'),
            modifiers,
            ..
        } if is_plain_text_modifier(modifiers) => {
            let before = flow.cwd.clone();
            if let Some(entry) = selected_flow_entry(state) {
                flow.enter(&entry);
            }
            if flow.cwd != before {
                *state = flow_palette_state(flow)?;
            } else {
                sync_flow_selection(flow, state);
            }
            None
        }
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
            sync_flow_selection(flow, state);
            None
        }
        KeyEvent {
            code: KeyCode::Down,
            ..
        } => {
            state.move_down();
            sync_flow_selection(flow, state);
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
    };
    Ok(outcome)
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

fn sync_flow_selection(flow: &mut FlowState, state: &PaletteState) {
    flow.selected = state.selected_index().unwrap_or(0);
}

fn selected_flow_entry(state: &PaletteState) -> Option<FlowEntry> {
    let item = state.selected_item()?;
    Some(FlowEntry {
        path: item.path.clone()?,
        name: item.label.clone(),
        is_dir: matches!(item.kind, PaletteItemKind::Dir),
    })
}

fn flow_palette_state(flow: &FlowState) -> Result<PaletteState> {
    let mut state = PaletteState::new(flow.entries()?.into_iter().map(flow_item).collect());
    if !state.items.is_empty() {
        state.selected = flow.selected.min(state.items.len() - 1);
    }
    Ok(state)
}

fn flow_item(entry: FlowEntry) -> PaletteItem {
    PaletteItem {
        label: entry.name,
        path: Some(entry.path),
        kind: if entry.is_dir {
            PaletteItemKind::Dir
        } else {
            PaletteItemKind::File
        },
        source: "flow".to_owned(),
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
    if chrome != PaletteChrome::Search {
        return Line::from(title_span);
    }

    let query = if state.query.is_empty() {
        "<empty>"
    } else {
        state.query.as_str()
    };
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
    "Esc cancel  Enter open  Up/Down move  Left/Right or h/l navigate  Space expand"
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
            let suffix = palette_row_kind_text(item);
            let suffix_width = suffix.map(|text| text.chars().count() + 2).unwrap_or(0);
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
            if let Some(suffix) = suffix {
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
    use crate::flow::FlowState;
    use crate::ui::palette::{PaletteItem, PaletteItemKind};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::fs;
    use std::path::PathBuf;

    fn item(label: &str) -> PaletteItem {
        PaletteItem {
            label: label.to_owned(),
            path: None,
            kind: PaletteItemKind::Menu,
            source: "test".to_owned(),
        }
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl_key(ch: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL)
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
    fn flow_palette_omits_parent_entry() {
        let dir = tempfile::tempdir().unwrap();
        let child = dir.path().join("child");
        fs::create_dir(&child).unwrap();
        let flow = FlowState::new(child);

        let state = flow_palette_state(&flow).unwrap();

        assert!(!state.items.iter().any(|item| item.label == ".."));
    }

    #[test]
    fn flow_right_enters_selected_directory_and_refreshes_items() {
        let dir = tempfile::tempdir().unwrap();
        let child = dir.path().join("src");
        fs::create_dir(&child).unwrap();
        fs::create_dir(child.join("nested")).unwrap();
        let mut flow = FlowState::new(dir.path().to_path_buf());
        let mut state = flow_palette_state(&flow).unwrap();
        state.selected = 0;

        let outcome = handle_flow_key(&mut flow, &mut state, key(KeyCode::Right)).unwrap();

        assert_eq!(outcome, None);
        assert_eq!(flow.cwd, child);
        assert_eq!(state.selected, 0);
        assert_eq!(state.items[0].label, "nested");
    }

    #[test]
    fn flow_left_moves_to_parent_and_refreshes_items() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("alpha")).unwrap();
        let child = dir.path().join("src");
        fs::create_dir(&child).unwrap();
        let mut flow = FlowState::new(child);
        let mut state = flow_palette_state(&flow).unwrap();

        let outcome = handle_flow_key(&mut flow, &mut state, key(KeyCode::Left)).unwrap();

        assert_eq!(outcome, None);
        assert_eq!(flow.cwd, dir.path());
        assert_eq!(state.items[state.selected].label, "src");
    }

    #[test]
    fn flow_footer_mentions_navigation_keys() {
        let footer = flow_footer_text();

        assert!(footer.contains("Left/Right"));
        assert!(footer.contains("h/l"));
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
