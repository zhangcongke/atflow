use anyhow::Result;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchFilter {
    All,
    Dirs,
    Files,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub path: PathBuf,
    pub is_dir: bool,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct SearchRequest {
    pub roots: Vec<PathBuf>,
    pub query: Option<String>,
    pub filter: SearchFilter,
    pub ignore_names: Vec<String>,
    pub limit: usize,
}

pub fn search(request: &SearchRequest) -> Result<Vec<SearchResult>> {
    let query = request.query.as_deref().unwrap_or_default().to_lowercase();
    let mut results = Vec::new();
    if request.limit == 0 {
        return Ok(results);
    }

    for root in &request.roots {
        if !root.exists() {
            continue;
        }
        let mut builder = WalkBuilder::new(root);
        builder.hidden(false).git_ignore(true).git_exclude(true);

        for entry in builder.build().filter_map(|entry| entry.ok()) {
            let path = entry.path();
            if path == root {
                continue;
            }
            if ignored_by_name(path, &request.ignore_names) {
                continue;
            }

            let file_type = match entry.file_type() {
                Some(file_type) => file_type,
                None => continue,
            };
            let is_dir = file_type.is_dir();
            if request.filter == SearchFilter::Dirs && !is_dir {
                continue;
            }
            if request.filter == SearchFilter::Files && is_dir {
                continue;
            }

            let text = path.display().to_string().to_lowercase();
            if !query.is_empty() && !text.contains(&query) {
                continue;
            }

            results.push(SearchResult {
                path: path.to_path_buf(),
                is_dir,
                source: root.display().to_string(),
            });

            if results.len() >= request.limit {
                return Ok(results);
            }
        }
    }

    Ok(results)
}

fn ignored_by_name(path: &Path, ignore_names: &[String]) -> bool {
    path.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        ignore_names.iter().any(|ignored| ignored == &name)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn searches_files_and_dirs_with_query() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("data/nightlight")).unwrap();
        fs::write(dir.path().join("data/nightlight_loader.rs"), "").unwrap();
        fs::write(dir.path().join("notes.txt"), "").unwrap();

        let results = search(&SearchRequest {
            roots: vec![dir.path().to_path_buf()],
            query: Some("nightlight".to_owned()),
            filter: SearchFilter::All,
            ignore_names: vec![],
            limit: 10,
        })
        .unwrap();

        assert_eq!(results.len(), 2);
        assert!(
            results
                .iter()
                .any(|result| result.path.ends_with("nightlight"))
        );
        assert!(
            results
                .iter()
                .any(|result| result.path.ends_with("nightlight_loader.rs"))
        );
    }

    #[test]
    fn filters_dirs_only() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/main.rs"), "").unwrap();

        let results = search(&SearchRequest {
            roots: vec![dir.path().to_path_buf()],
            query: Some("src".to_owned()),
            filter: SearchFilter::Dirs,
            ignore_names: vec![],
            limit: 10,
        })
        .unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].is_dir);
    }

    #[test]
    fn skips_ignored_names() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("target/debug")).unwrap();
        fs::write(dir.path().join("target/debug/app"), "").unwrap();

        let results = search(&SearchRequest {
            roots: vec![dir.path().to_path_buf()],
            query: Some("app".to_owned()),
            filter: SearchFilter::All,
            ignore_names: vec!["target".to_owned()],
            limit: 10,
        })
        .unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn respects_zero_limit() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("one.txt"), "").unwrap();

        let results = search(&SearchRequest {
            roots: vec![dir.path().to_path_buf()],
            query: None,
            filter: SearchFilter::All,
            ignore_names: vec![],
            limit: 0,
        })
        .unwrap();

        assert!(results.is_empty());
    }
}
