use crate::config::Config;
use crate::ui::palette::PaletteItem;
use crate::ui::theme::ThemeName;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingField {
    Theme,
    MaxRecent,
    StartFromGitRoot,
    Editor,
    GuiEditor,
    FileOpener,
    PreferTerminalEditor,
    RecordAtflowOpens,
    RecordShellCd,
    SearchRoots,
    IgnoreNames,
}

const FIELDS: &[SettingField] = &[
    SettingField::Theme,
    SettingField::MaxRecent,
    SettingField::StartFromGitRoot,
    SettingField::Editor,
    SettingField::GuiEditor,
    SettingField::FileOpener,
    SettingField::PreferTerminalEditor,
    SettingField::RecordAtflowOpens,
    SettingField::RecordShellCd,
    SettingField::SearchRoots,
    SettingField::IgnoreNames,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsState {
    config: Config,
    selected: usize,
}

impl SettingsState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            selected: 0,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn into_config(self) -> Config {
        self.config
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn move_up(&mut self) {
        self.selected = if self.selected == 0 {
            FIELDS.len() - 1
        } else {
            self.selected - 1
        };
    }

    pub fn move_down(&mut self) {
        self.selected = (self.selected + 1) % FIELDS.len();
    }

    pub fn change_left(&mut self) {
        self.change(-1);
    }

    pub fn change_right(&mut self) {
        self.change(1);
    }

    pub fn palette_items(&self) -> Vec<PaletteItem> {
        FIELDS
            .iter()
            .map(|field| PaletteItem::menu(row_label(field.label(), self.value_for(*field))))
            .collect()
    }

    fn change(&mut self, step: isize) {
        match FIELDS[self.selected] {
            SettingField::Theme => {
                self.config.general.theme = cycle_copy(
                    self.config.general.theme,
                    &[ThemeName::Mist, ThemeName::Ink, ThemeName::Paper],
                    step,
                );
            }
            SettingField::MaxRecent => {
                self.config.general.max_recent = cycle_copy(
                    self.config.general.max_recent,
                    &[25, 50, 100, 200, 500],
                    step,
                );
            }
            SettingField::StartFromGitRoot => {
                self.config.general.start_from_git_root = !self.config.general.start_from_git_root;
            }
            SettingField::Editor => {
                self.config.open.editor = cycle_string(
                    &self.config.open.editor,
                    &["nvim", "vim", "vi", "nano", "hx", "code"],
                    step,
                );
            }
            SettingField::GuiEditor => {
                self.config.open.gui_editor = cycle_string(
                    &self.config.open.gui_editor,
                    &["code", "zed", "cursor", "subl"],
                    step,
                );
            }
            SettingField::FileOpener => {
                self.config.open.file_opener =
                    cycle_string(&self.config.open.file_opener, &["xdg-open", "open"], step);
            }
            SettingField::PreferTerminalEditor => {
                self.config.open.prefer_terminal_editor = !self.config.open.prefer_terminal_editor;
            }
            SettingField::RecordAtflowOpens => {
                self.config.history.record_atflow_opens = !self.config.history.record_atflow_opens;
            }
            SettingField::RecordShellCd => {
                self.config.history.record_shell_cd = !self.config.history.record_shell_cd;
            }
            SettingField::SearchRoots => {
                self.config.search.roots = cycle_string_list(
                    &self.config.search.roots,
                    &[
                        vec!["~/work", "~/code", "~/Documents"],
                        vec!["~/work"],
                        vec!["~"],
                        vec![],
                    ],
                    step,
                );
            }
            SettingField::IgnoreNames => {
                self.config.search.ignore = cycle_string_list(
                    &self.config.search.ignore,
                    &[
                        vec![
                            ".git",
                            "node_modules",
                            "__pycache__",
                            ".venv",
                            "target",
                            "dist",
                        ],
                        vec![".git", "node_modules", "target"],
                        vec![],
                    ],
                    step,
                );
            }
        }
    }

    fn value_for(&self, field: SettingField) -> String {
        match field {
            SettingField::Theme => self.config.general.theme.as_str().to_owned(),
            SettingField::MaxRecent => self.config.general.max_recent.to_string(),
            SettingField::StartFromGitRoot => bool_label(self.config.general.start_from_git_root),
            SettingField::Editor => self.config.open.editor.clone(),
            SettingField::GuiEditor => self.config.open.gui_editor.clone(),
            SettingField::FileOpener => self.config.open.file_opener.clone(),
            SettingField::PreferTerminalEditor => {
                bool_label(self.config.open.prefer_terminal_editor)
            }
            SettingField::RecordAtflowOpens => bool_label(self.config.history.record_atflow_opens),
            SettingField::RecordShellCd => bool_label(self.config.history.record_shell_cd),
            SettingField::SearchRoots => list_label(&self.config.search.roots),
            SettingField::IgnoreNames => list_label(&self.config.search.ignore),
        }
    }
}

impl SettingField {
    fn label(self) -> &'static str {
        match self {
            Self::Theme => "Theme",
            Self::MaxRecent => "Max recent",
            Self::StartFromGitRoot => "Git root start",
            Self::Editor => "Editor",
            Self::GuiEditor => "GUI editor",
            Self::FileOpener => "File opener",
            Self::PreferTerminalEditor => "Terminal editor",
            Self::RecordAtflowOpens => "Record @ opens",
            Self::RecordShellCd => "Record shell cd",
            Self::SearchRoots => "Search roots",
            Self::IgnoreNames => "Ignore names",
        }
    }
}

fn row_label(label: &str, value: String) -> String {
    format!("{label:<18} {value}")
}

fn bool_label(value: bool) -> String {
    if value { "on" } else { "off" }.to_owned()
}

fn list_label(values: &[String]) -> String {
    if values.is_empty() {
        "<none>".to_owned()
    } else {
        values.join(", ")
    }
}

fn cycle_copy<T: Copy + Eq>(current: T, base_options: &[T], step: isize) -> T {
    let options = copy_options(current, base_options);
    options[cycle_index(
        options
            .iter()
            .position(|item| *item == current)
            .unwrap_or(0),
        options.len(),
        step,
    )]
}

fn copy_options<T: Copy + Eq>(current: T, base_options: &[T]) -> Vec<T> {
    if base_options.contains(&current) {
        return base_options.to_vec();
    }
    let mut options = vec![current];
    options.extend_from_slice(base_options);
    options
}

fn cycle_string(current: &str, base_options: &[&str], step: isize) -> String {
    let options = string_options(current, base_options);
    let current_index = options
        .iter()
        .position(|value| value.as_str() == current)
        .unwrap_or(0);
    options[cycle_index(current_index, options.len(), step)].clone()
}

fn string_options(current: &str, base_options: &[&str]) -> Vec<String> {
    if base_options.contains(&current) {
        return base_options
            .iter()
            .map(|value| (*value).to_owned())
            .collect();
    }
    let mut options = vec![current.to_owned()];
    options.extend(base_options.iter().map(|value| (*value).to_owned()));
    options
}

fn cycle_string_list(current: &[String], base_options: &[Vec<&str>], step: isize) -> Vec<String> {
    let options = string_list_options(current, base_options);
    let current_index = options
        .iter()
        .position(|value| value.as_slice() == current)
        .unwrap_or(0);
    options[cycle_index(current_index, options.len(), step)].clone()
}

fn string_list_options(current: &[String], base_options: &[Vec<&str>]) -> Vec<Vec<String>> {
    let base: Vec<Vec<String>> = base_options
        .iter()
        .map(|option| option.iter().map(|value| (*value).to_owned()).collect())
        .collect();
    if base.iter().any(|option| option.as_slice() == current) {
        return base;
    }
    let mut options = vec![current.to_vec()];
    options.extend(base);
    options
}

fn cycle_index(current: usize, len: usize, step: isize) -> usize {
    if len == 0 {
        return 0;
    }
    (current as isize + step).rem_euclid(len as isize) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_and_left_cycle_theme() {
        let mut state = SettingsState::new(Config::default());

        state.change_right();
        assert_eq!(state.config().general.theme, ThemeName::Ink);

        state.change_left();
        assert_eq!(state.config().general.theme, ThemeName::Mist);
    }

    #[test]
    fn selected_boolean_toggles_with_either_direction() {
        let mut state = SettingsState::new(Config::default());
        state.move_down();
        state.move_down();

        state.change_right();
        assert!(!state.config().general.start_from_git_root);

        state.change_left();
        assert!(state.config().general.start_from_git_root);
    }

    #[test]
    fn movement_wraps_between_first_and_last_setting_rows() {
        let mut state = SettingsState::new(Config::default());

        state.move_up();
        assert_eq!(state.selected(), FIELDS.len() - 1);

        state.move_down();
        assert_eq!(state.selected(), 0);
    }

    #[test]
    fn string_options_keep_custom_value_in_cycle() {
        let mut config = Config::default();
        config.open.editor = "custom-editor".to_owned();
        let mut state = SettingsState::new(config);
        state.move_down();
        state.move_down();
        state.move_down();

        state.change_right();
        assert_eq!(state.config().open.editor, "nvim");
    }

    #[test]
    fn palette_rows_include_all_current_values() {
        let state = SettingsState::new(Config::default());
        let rows: Vec<String> = state
            .palette_items()
            .into_iter()
            .map(|item| item.label)
            .collect();

        assert!(rows.iter().any(|row| row.contains("Theme")));
        assert!(rows.iter().any(|row| row.contains("mist")));
        assert!(rows.iter().any(|row| row.contains("Search roots")));
        assert!(rows.iter().any(|row| row.contains("~/work")));
    }
}
