use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowState {
    pub cwd: PathBuf,
    pub selected: usize,
}

impl FlowState {
    pub fn new(start: PathBuf) -> Self {
        Self {
            cwd: start,
            selected: 0,
        }
    }

    pub fn entries(&self) -> Result<Vec<FlowEntry>> {
        list_entries(&self.cwd)
    }

    pub fn parent(&mut self) {
        if let Some(parent) = self.cwd.parent() {
            self.cwd = parent.to_path_buf();
            self.selected = 0;
        }
    }

    pub fn enter(&mut self, entry: &FlowEntry) {
        if entry.is_dir && entry.path.is_dir() {
            self.cwd = entry.path.clone();
            self.selected = 0;
        }
    }
}

pub fn list_entries(path: &Path) -> Result<Vec<FlowEntry>> {
    let mut entries = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let name = entry.file_name().to_string_lossy().to_string();
        entries.push(FlowEntry {
            path: entry.path(),
            name,
            is_dir: file_type.is_dir(),
        });
    }

    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
    Ok(entries)
}

pub fn git_root_from(start: &Path) -> Option<PathBuf> {
    let mut current = start;
    loop {
        if current.join(".git").exists() {
            return Some(current.to_path_buf());
        }
        current = current.parent()?;
    }
}

pub fn flow_start(current: &Path, start_from_git_root: bool) -> PathBuf {
    if start_from_git_root {
        git_root_from(current).unwrap_or_else(|| current.to_path_buf())
    } else {
        current.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn lists_dirs_then_files_without_parent_entry() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("Cargo.toml"), "").unwrap();

        let entries = list_entries(dir.path()).unwrap();

        assert_eq!(entries[0].name, "src");
        assert!(entries[0].is_dir);
        assert_eq!(entries[1].name, "Cargo.toml");
        assert!(!entries[1].is_dir);
        assert!(!entries.iter().any(|entry| entry.name == ".."));
    }

    #[test]
    fn sorts_child_directories_without_injected_parent_entry() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("!cache")).unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();

        let entries = list_entries(dir.path()).unwrap();

        assert_eq!(entries[0].name, "!cache");
        assert_eq!(entries[1].name, "src");
        assert!(!entries.iter().any(|entry| entry.name == ".."));
    }

    #[test]
    fn navigation_resets_selection() {
        let dir = tempfile::tempdir().unwrap();
        let child = dir.path().join("src");
        fs::create_dir(&child).unwrap();
        fs::write(dir.path().join("Cargo.toml"), "").unwrap();

        let mut state = FlowState {
            cwd: child.clone(),
            selected: 2,
        };
        state.parent();

        assert_eq!(state.cwd, dir.path());
        assert_eq!(state.selected, 0);

        state.selected = 1;
        state.enter(&FlowEntry {
            path: child.clone(),
            name: "src".to_owned(),
            is_dir: true,
        });

        assert_eq!(state.cwd, child);
        assert_eq!(state.selected, 0);

        state.selected = 1;
        state.enter(&FlowEntry {
            path: dir.path().join("Cargo.toml"),
            name: "Cargo.toml".to_owned(),
            is_dir: false,
        });

        assert_eq!(state.cwd, child);
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn enter_ignores_stale_directory_entries() {
        let dir = tempfile::tempdir().unwrap();
        let child = dir.path().join("src");
        fs::create_dir(&child).unwrap();
        let mut state = FlowState::new(dir.path().to_path_buf());
        let stale_entry = FlowEntry {
            path: child.clone(),
            name: "src".to_owned(),
            is_dir: true,
        };
        fs::remove_dir(&child).unwrap();

        state.enter(&stale_entry);

        assert_eq!(state.cwd, dir.path());
    }

    #[test]
    fn detects_git_root() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::create_dir_all(dir.path().join("src/nested")).unwrap();

        assert_eq!(
            git_root_from(&dir.path().join("src/nested")),
            Some(dir.path().to_path_buf())
        );
    }

    #[test]
    fn starts_from_git_root_when_enabled() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::create_dir_all(dir.path().join("src/nested")).unwrap();

        assert_eq!(
            flow_start(&dir.path().join("src/nested"), true),
            dir.path().to_path_buf()
        );
    }
}
