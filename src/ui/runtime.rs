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
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::io;

use crate::flow::{FlowEntry, FlowState};
use crate::search::SearchFilter;
use crate::ui::palette::{PaletteItem, PaletteItemKind, PaletteState};

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
    let _session = TerminalSession::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stderr()))?;
    terminal.clear()?;

    loop {
        terminal.draw(|frame| render_palette(frame, title, &state))?;
        if let Event::Key(key) = event::read()? {
            if !is_key_press(key) {
                continue;
            }
            if let Some(outcome) = handle_palette_key(&mut state, key) {
                return Ok(outcome);
            }
        }
    }
}

pub fn run_search_palette<F>(
    title: &str,
    mut state: PaletteState,
    mut refresh: F,
) -> Result<UiResponse>
where
    F: FnMut(&str, SearchFilter) -> Result<Vec<PaletteItem>>,
{
    let _session = TerminalSession::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stderr()))?;
    terminal.clear()?;

    loop {
        terminal.draw(|frame| render_palette(frame, title, &state))?;
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

pub fn run_flow_palette(title: &str, mut flow: FlowState) -> Result<UiResponse> {
    let _session = TerminalSession::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stderr()))?;
    terminal.clear()?;
    let mut state = flow_palette_state(&flow)?;

    loop {
        terminal.draw(|frame| render_palette(frame, title, &state))?;
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
            code: KeyCode::Tab, ..
        } => {
            state.cycle_filter();
            None
        }
        KeyEvent {
            code: KeyCode::Backspace,
            ..
        } => {
            state.query.pop();
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
            code: KeyCode::Char('e' | 'E'),
            modifiers,
            ..
        } if modifiers.contains(KeyModifiers::CONTROL) => {
            state.selected_index().map(UiOutcome::Editor)
        }
        KeyEvent {
            code: KeyCode::Char('o' | 'O'),
            modifiers,
            ..
        } if modifiers.contains(KeyModifiers::CONTROL) => {
            state.selected_index().map(UiOutcome::System)
        }
        KeyEvent {
            code: KeyCode::Char(ch),
            modifiers,
            ..
        } if is_plain_text_modifier(modifiers) => {
            state.query.push(ch);
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
            flow.parent();
            *state = flow_palette_state(flow)?;
            None
        }
        KeyEvent {
            code: KeyCode::Char('h' | 'H'),
            modifiers,
            ..
        } if is_plain_text_modifier(modifiers) => {
            flow.parent();
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
        KeyEvent {
            code: KeyCode::Char('e' | 'E'),
            modifiers,
            ..
        } if modifiers.contains(KeyModifiers::CONTROL) => {
            state.selected_index().map(UiOutcome::Editor)
        }
        KeyEvent {
            code: KeyCode::Char('o' | 'O'),
            modifiers,
            ..
        } if modifiers.contains(KeyModifiers::CONTROL) => {
            state.selected_index().map(UiOutcome::System)
        }
        _ => None,
    };
    Ok(outcome)
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

fn render_palette(frame: &mut Frame<'_>, title: &str, state: &PaletteState) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

    let query = if state.query.is_empty() {
        "<empty>"
    } else {
        state.query.as_str()
    };
    let header = Line::from(vec![
        Span::styled(
            title.to_owned(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("  query: "),
        Span::styled(query.to_owned(), Style::default().fg(Color::Yellow)),
        Span::raw("  filter: "),
        Span::styled(filter_label(state.filter), Style::default().fg(Color::Cyan)),
    ]);
    frame.render_widget(
        Paragraph::new(header).block(Block::default().borders(Borders::ALL)),
        chunks[0],
    );

    let list_width = usize::from(chunks[1].width.saturating_sub(2));
    let rows = palette_rows(state, list_width);
    frame.render_widget(
        List::new(rows).block(Block::default().borders(Borders::LEFT | Borders::RIGHT)),
        chunks[1],
    );

    let footer = Line::from(vec![
        Span::raw("Esc cancel  Enter select  "),
        Span::raw("Up/Down move  Space expand  "),
        Span::raw("Tab filter  Ctrl+E editor  Ctrl+O system"),
    ]);
    frame.render_widget(
        Paragraph::new(footer).block(Block::default().borders(Borders::ALL)),
        chunks[2],
    );
}

fn palette_rows(state: &PaletteState, area_width: usize) -> Vec<ListItem<'static>> {
    if state.items.is_empty() {
        return vec![ListItem::new("  No results").style(Style::default().fg(Color::DarkGray))];
    }

    let selected = state.selected_index();
    state
        .items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let marker = if Some(index) == selected { ">" } else { " " };
            let source_width = item.source.chars().count() + 3;
            let label_width = area_width.saturating_sub(source_width + 2).max(1);
            let label = state
                .display_label_at(index, label_width)
                .unwrap_or_else(|| item.label.clone());
            let style = if Some(index) == selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::raw(format!("{marker} ")),
                Span::raw(label),
                Span::styled(
                    format!("  {}", item.source),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
            .style(style)
        })
        .collect()
}

fn filter_label(filter: SearchFilter) -> &'static str {
    match filter {
        SearchFilter::All => "all",
        SearchFilter::Dirs => "dirs",
        SearchFilter::Files => "files",
    }
}

struct TerminalSession;

impl TerminalSession {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        let mut stderr = io::stderr();
        if let Err(error) = execute!(stderr, EnterAlternateScreen) {
            restore_terminal();
            return Err(error.into());
        }
        if let Err(error) = execute!(stderr, Hide) {
            restore_terminal();
            return Err(error.into());
        }
        Ok(Self)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        restore_terminal();
    }
}

fn restore_terminal() {
    let mut stderr = io::stderr();
    let _ = execute!(stderr, Show, LeaveAlternateScreen);
    let _ = disable_raw_mode();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::FlowState;
    use crate::ui::palette::{PaletteItem, PaletteItemKind};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::fs;

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
    fn enter_on_empty_has_no_outcome() {
        let mut state = PaletteState::new(vec![]);

        let outcome = handle_palette_key(&mut state, key(KeyCode::Enter));

        assert_eq!(outcome, None);
    }

    #[test]
    fn editor_and_system_on_empty_have_no_outcome() {
        let mut state = PaletteState::new(vec![]);

        let editor = handle_palette_key(&mut state, ctrl_key('e'));
        let system = handle_palette_key(&mut state, ctrl_key('o'));

        assert_eq!(editor, None);
        assert_eq!(system, None);
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
    fn flow_palette_preserves_parent_entry_label() {
        let dir = tempfile::tempdir().unwrap();
        let child = dir.path().join("child");
        fs::create_dir(&child).unwrap();
        let flow = FlowState::new(child);

        let state = flow_palette_state(&flow).unwrap();

        assert_eq!(state.items[0].label, "..");
    }

    #[test]
    fn flow_right_enters_selected_directory_and_refreshes_items() {
        let dir = tempfile::tempdir().unwrap();
        let child = dir.path().join("src");
        fs::create_dir(&child).unwrap();
        let mut flow = FlowState::new(dir.path().to_path_buf());
        let mut state = flow_palette_state(&flow).unwrap();
        state.selected = 1;

        let outcome = handle_flow_key(&mut flow, &mut state, key(KeyCode::Right)).unwrap();

        assert_eq!(outcome, None);
        assert_eq!(flow.cwd, child);
        assert_eq!(state.selected, 0);
        assert_eq!(state.items[0].label, "..");
    }

    #[test]
    fn flow_left_moves_to_parent_and_refreshes_items() {
        let dir = tempfile::tempdir().unwrap();
        let child = dir.path().join("src");
        fs::create_dir(&child).unwrap();
        let mut flow = FlowState::new(child);
        let mut state = flow_palette_state(&flow).unwrap();

        let outcome = handle_flow_key(&mut flow, &mut state, key(KeyCode::Left)).unwrap();

        assert_eq!(outcome, None);
        assert_eq!(flow.cwd, dir.path());
        assert_eq!(state.selected, 0);
    }
}
