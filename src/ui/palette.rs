use crate::path_display::display_path;
use crate::search::SearchFilter;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub pinned: bool,
}

impl PaletteItem {
    pub fn dir(path: PathBuf, source: impl Into<String>) -> Self {
        Self {
            label: path_label(&path),
            path: Some(path),
            kind: PaletteItemKind::Dir,
            source: source.into(),
            pinned: false,
        }
    }

    pub fn file(path: PathBuf, source: impl Into<String>) -> Self {
        Self {
            label: path_label(&path),
            path: Some(path),
            kind: PaletteItemKind::File,
            source: source.into(),
            pinned: false,
        }
    }

    pub fn menu(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            path: None,
            kind: PaletteItemKind::Menu,
            source: "menu".to_owned(),
            pinned: false,
        }
    }

    pub fn pinned(mut self) -> Self {
        self.pinned = true;
        self
    }
}

fn path_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
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

    pub fn selected_index(&self) -> Option<usize> {
        if self.items.is_empty() {
            None
        } else {
            Some(self.selected.min(self.items.len() - 1))
        }
    }

    pub fn selected_item(&self) -> Option<&PaletteItem> {
        self.selected_index()
            .and_then(|index| self.items.get(index))
    }

    pub fn move_down(&mut self) {
        if let Some(selected) = self.selected_index() {
            self.selected = (selected + 1) % self.items.len();
            self.expanded = false;
        }
    }

    pub fn move_up(&mut self) {
        if let Some(selected) = self.selected_index() {
            self.selected = if selected == 0 {
                self.items.len() - 1
            } else {
                selected - 1
            };
            self.expanded = false;
        }
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

    pub fn display_label_at(&self, index: usize, width: usize) -> Option<String> {
        let item = self.items.get(index)?;
        Some(format_item_label(
            item,
            self.expanded && Some(index) == self.selected_index(),
            width,
        ))
    }

    pub fn display_label(&self, item: &PaletteItem, width: usize) -> String {
        format_item_label(item, false, width)
    }
}

fn format_item_label(item: &PaletteItem, expanded: bool, width: usize) -> String {
    match &item.path {
        Some(path) => display_path(path, expanded, width),
        None => item.label.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path_item(label: &str, path: &str, kind: PaletteItemKind) -> PaletteItem {
        PaletteItem {
            label: label.to_owned(),
            path: Some(PathBuf::from(path)),
            kind,
            source: "test".to_owned(),
            pinned: false,
        }
    }

    fn menu_item(label: &str) -> PaletteItem {
        PaletteItem {
            label: label.to_owned(),
            path: None,
            kind: PaletteItemKind::Menu,
            source: "menu".to_owned(),
            pinned: false,
        }
    }

    #[test]
    fn new_defaults_to_collapsed_all_filter_with_first_item_selected() {
        let state = PaletteState::new(vec![menu_item("Open settings")]);

        assert_eq!(state.query, "");
        assert_eq!(state.selected, 0);
        assert_eq!(state.selected_index(), Some(0));
        assert!(!state.expanded);
        assert_eq!(state.filter, SearchFilter::All);
        assert_eq!(state.selected_item().unwrap().label, "Open settings");
    }

    #[test]
    fn empty_palette_has_no_selected_index() {
        let state = PaletteState::new(vec![]);

        assert_eq!(state.selected_index(), None);
        assert_eq!(state.selected_item(), None);
    }

    #[test]
    fn movement_wraps_and_resets_expansion_on_non_empty_lists() {
        let mut state = PaletteState::new(vec![menu_item("one"), menu_item("two")]);

        state.toggle_expanded();
        state.move_down();
        assert_eq!(state.selected, 1);
        assert!(!state.expanded);

        state.toggle_expanded();
        state.move_down();
        assert_eq!(state.selected, 0);
        assert!(!state.expanded);

        state.toggle_expanded();
        state.move_up();
        assert_eq!(state.selected, 1);
        assert!(!state.expanded);

        state.toggle_expanded();
        state.move_up();
        assert_eq!(state.selected, 0);
        assert!(!state.expanded);
    }

    #[test]
    fn cycle_filter_rotates_all_dirs_files() {
        let mut state = PaletteState::new(vec![]);

        state.cycle_filter();
        assert_eq!(state.filter, SearchFilter::Dirs);

        state.cycle_filter();
        assert_eq!(state.filter, SearchFilter::Files);

        state.cycle_filter();
        assert_eq!(state.filter, SearchFilter::All);
    }

    #[test]
    fn expanded_selected_path_displays_full_path() {
        let path = "/home/congke/work/at-flow/src/ui/palette.rs";
        let mut state = PaletteState::new(vec![path_item("palette", path, PaletteItemKind::File)]);

        state.toggle_expanded();

        assert_eq!(
            state.display_label_at(0, 18).unwrap(),
            display_path(&PathBuf::from(path), true, 18)
        );
    }

    #[test]
    fn index_based_display_does_not_need_item_pointer_identity() {
        let path = "/home/congke/work/at-flow/src/ui/palette.rs";
        let mut state = PaletteState::new(vec![path_item("palette", path, PaletteItemKind::File)]);
        let cloned_item = state.items[0].clone();

        state.toggle_expanded();

        assert_eq!(
            state.display_label_at(0, 18).unwrap(),
            display_path(cloned_item.path.as_ref().unwrap(), true, 18)
        );
    }

    #[test]
    fn non_selected_paths_stay_clipped_when_another_item_is_expanded() {
        let selected_path = "/home/congke/work/at-flow/src/main.rs";
        let other_path = "/home/congke/work/at-flow/src/ui/palette.rs";
        let mut state = PaletteState::new(vec![
            path_item("main", selected_path, PaletteItemKind::File),
            path_item("palette", other_path, PaletteItemKind::File),
        ]);

        state.toggle_expanded();

        assert_eq!(
            state.display_label_at(1, 18).unwrap(),
            display_path(&PathBuf::from(other_path), false, 18)
        );
    }

    #[test]
    fn display_label_does_not_expand_equal_non_selected_duplicates() {
        let path = "/home/congke/work/at-flow/src/ui/palette.rs";
        let item = path_item("palette", path, PaletteItemKind::File);
        let mut state = PaletteState::new(vec![item.clone(), item]);

        state.toggle_expanded();

        assert_eq!(
            state.display_label(&state.items[1], 18),
            display_path(&PathBuf::from(path), false, 18)
        );
    }

    #[test]
    fn menu_items_display_their_label() {
        let state = PaletteState::new(vec![menu_item("Open settings")]);

        assert_eq!(state.display_label_at(0, 3).unwrap(), "Open settings");
    }

    #[test]
    fn constructors_use_file_name_labels_for_paths() {
        let dir_path = PathBuf::from("/tmp/at-flow");
        let file_path = PathBuf::from("/tmp/at-flow/src/main.rs");

        let dir = PaletteItem::dir(dir_path.clone(), "recent");
        let file = PaletteItem::file(file_path.clone(), "search");

        assert_eq!(dir.label, "at-flow");
        assert_eq!(dir.path, Some(dir_path));
        assert_eq!(dir.kind, PaletteItemKind::Dir);
        assert_eq!(dir.source, "recent");
        assert_eq!(file.label, "main.rs");
        assert_eq!(file.path, Some(file_path));
        assert_eq!(file.kind, PaletteItemKind::File);
        assert_eq!(file.source, "search");
    }

    #[test]
    fn path_constructors_fall_back_to_path_string_for_labels() {
        let path = PathBuf::from("/");

        let item = PaletteItem::dir(path.clone(), "root");

        assert_eq!(item.label, path.display().to_string());
        assert_eq!(item.path, Some(path));
    }

    #[test]
    fn menu_constructor_builds_non_path_item() {
        let item = PaletteItem::menu("Settings");

        assert_eq!(item.label, "Settings");
        assert_eq!(item.path, None);
        assert_eq!(item.kind, PaletteItemKind::Menu);
        assert_eq!(item.source, "menu");
        assert!(!item.pinned);
    }

    #[test]
    fn stale_selected_index_is_clamped_for_public_selection_methods() {
        let mut state = PaletteState::new(vec![menu_item("one"), menu_item("two")]);
        state.selected = 99;

        assert_eq!(state.selected_index(), Some(1));
        assert_eq!(state.selected_item().unwrap().label, "two");

        state.toggle_expanded();
        state.move_up();

        assert_eq!(state.selected_index(), Some(0));
        assert_eq!(state.selected, 0);
        assert!(!state.expanded);
    }

    #[test]
    fn stale_selected_index_expands_the_clamped_item() {
        let path = "/home/congke/work/at-flow/src/ui/palette.rs";
        let mut state = PaletteState::new(vec![
            menu_item("Open settings"),
            path_item("palette", path, PaletteItemKind::File),
        ]);
        state.selected = 99;
        state.toggle_expanded();

        assert_eq!(
            state.display_label_at(1, 18).unwrap(),
            display_path(&PathBuf::from(path), true, 18)
        );
        assert_eq!(state.display_label_at(99, 18), None);
    }

    #[test]
    fn replace_items_resets_selection_and_expansion() {
        let mut state = PaletteState::new(vec![menu_item("one"), menu_item("two")]);
        state.move_down();
        state.toggle_expanded();

        state.replace_items(vec![menu_item("replacement")]);

        assert_eq!(state.selected, 0);
        assert!(!state.expanded);
        assert_eq!(state.selected_item().unwrap().label, "replacement");
    }
}
