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
        if entry.is_dir {
            self.cwd = entry.path.clone();
            self.selected = 0;
        }
    }
}

pub fn list_entries(path: &Path) -> Result<Vec<FlowEntry>> {
    let mut entries = Vec::new();
    if let Some(parent) = path.parent() {
        entries.push(FlowEntry {
            path: parent.to_path_buf(),
            name: "..".to_owned(),
            is_dir: true,
        });
    }

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
    fn lists_parent_dirs_then_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("Cargo.toml"), "").unwrap();

        let entries = list_entries(dir.path()).unwrap();

        assert_eq!(entries[0].name, "..");
        assert_eq!(entries[1].name, "src");
        assert!(entries[1].is_dir);
        assert_eq!(entries[2].name, "Cargo.toml");
        assert!(!entries[2].is_dir);
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
