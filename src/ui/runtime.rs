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
    let _session = TerminalSession::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
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
    F: FnMut(&str, SearchFilter) -> Vec<PaletteItem>,
{
    let _session = TerminalSession::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
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
    F: FnMut(&str, SearchFilter) -> Vec<PaletteItem>,
{
    let outcome = match key {
        KeyEvent {
            code: KeyCode::Tab, ..
        } => {
            state.cycle_filter();
            refresh_items(state, &mut refresh);
            None
        }
        KeyEvent {
            code: KeyCode::Backspace,
            ..
        } => {
            state.query.pop();
            refresh_items(state, &mut refresh);
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
            refresh_items(state, &mut refresh);
            None
        }
        _ => handle_palette_key(state, key),
    };
    Ok(outcome)
}

fn refresh_items<F>(state: &mut PaletteState, refresh: &mut F)
where
    F: FnMut(&str, SearchFilter) -> Vec<PaletteItem>,
{
    let items = refresh(&state.query, state.filter);
    state.replace_items(items);
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
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen, Hide) {
            let _ = disable_raw_mode();
            return Err(error.into());
        }
        Ok(Self)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, Show, LeaveAlternateScreen);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::palette::{PaletteItem, PaletteItemKind};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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

    #[test]
    fn enter_on_empty_has_no_outcome() {
        let mut state = PaletteState::new(vec![]);

        let outcome = handle_palette_key(&mut state, key(KeyCode::Enter));

        assert_eq!(outcome, None);
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
            vec![item("refreshed")]
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
            vec![item("dirs")]
        })
        .unwrap();

        assert_eq!(outcome, None);
        assert_eq!(state.query, "abc");
        assert_eq!(state.filter, SearchFilter::Dirs);
        assert_eq!(state.items, vec![item("dirs")]);
        assert_eq!(calls, vec![("abc".to_owned(), SearchFilter::Dirs)]);
    }
}
