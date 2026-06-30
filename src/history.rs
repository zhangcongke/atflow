use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathKind {
    Dir,
    File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistorySource {
    Atflow,
    ShellCdHook,
    ManualRootScan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub path: PathBuf,
    pub kind: PathKind,
    pub source: HistorySource,
    pub last_opened_at: i64,
    pub open_count: i64,
}

pub struct HistoryDb {
    conn: Connection,
}

impl PathKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dir => "dir",
            Self::File => "file",
        }
    }

    #[expect(
        clippy::should_implement_trait,
        reason = "spec requires an infallible inherent wrapper"
    )]
    pub fn from_str(value: &str) -> Self {
        value.parse().unwrap_or(Self::Dir)
    }
}

impl std::str::FromStr for PathKind {
    type Err = Infallible;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match value {
            "file" => Self::File,
            _ => Self::Dir,
        })
    }
}

impl HistorySource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Atflow => "atflow",
            Self::ShellCdHook => "shell_cd_hook",
            Self::ManualRootScan => "manual_root_scan",
        }
    }

    #[expect(
        clippy::should_implement_trait,
        reason = "spec requires an infallible inherent wrapper"
    )]
    pub fn from_str(value: &str) -> Self {
        value.parse().unwrap_or(Self::Atflow)
    }
}

impl std::str::FromStr for HistorySource {
    type Err = Infallible;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match value {
            "shell_cd_hook" => Self::ShellCdHook,
            "manual_root_scan" => Self::ManualRootScan,
            _ => Self::Atflow,
        })
    }
}

impl HistoryDb {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create history dir {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("failed to open history database {}", path.display()))?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn open_memory() -> Result<Self> {
        let db = Self {
            conn: Connection::open_in_memory().context("failed to open in-memory database")?,
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        self.conn
            .execute_batch(
                r#"
            CREATE TABLE IF NOT EXISTS paths (
              id INTEGER PRIMARY KEY,
              path TEXT NOT NULL UNIQUE,
              kind TEXT NOT NULL,
              source TEXT NOT NULL,
              last_opened_at INTEGER NOT NULL,
              open_count INTEGER NOT NULL DEFAULT 1
            );

            CREATE INDEX IF NOT EXISTS idx_paths_last_opened ON paths(last_opened_at DESC);
            CREATE INDEX IF NOT EXISTS idx_paths_kind ON paths(kind);
            "#,
            )
            .context("failed to migrate history database")?;
        Ok(())
    }

    pub fn record_path_at(
        &self,
        path: &Path,
        kind: PathKind,
        source: HistorySource,
        timestamp: i64,
    ) -> Result<()> {
        let path_text = path.display().to_string();
        self.conn
            .execute(
                r#"
            INSERT INTO paths (path, kind, source, last_opened_at, open_count)
            VALUES (?1, ?2, ?3, ?4, 1)
            ON CONFLICT(path) DO UPDATE SET
              kind = excluded.kind,
              source = excluded.source,
              last_opened_at = excluded.last_opened_at,
              open_count = paths.open_count + 1
            "#,
                params![&path_text, kind.as_str(), source.as_str(), timestamp],
            )
            .with_context(|| format!("failed to record history path {path_text}"))?;
        Ok(())
    }

    pub fn recent_dirs(&self, limit: usize) -> Result<Vec<HistoryEntry>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT path, kind, source, last_opened_at, open_count
            FROM paths
            WHERE kind = 'dir'
            ORDER BY last_opened_at DESC, open_count DESC
            LIMIT ?1
            "#,
        )?;
        let rows = stmt.query_map([limit as i64], |row| {
            Ok(HistoryEntry {
                path: PathBuf::from(row.get::<_, String>(0)?),
                kind: PathKind::from_str(&row.get::<_, String>(1)?),
                source: HistorySource::from_str(&row.get::<_, String>(2)?),
                last_opened_at: row.get(3)?,
                open_count: row.get(4)?,
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("failed to load recent dirs")
    }
}

pub fn default_history_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("at")
        .join("history.sqlite")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_sorts_recent_directories() {
        let db = HistoryDb::open_memory().unwrap();
        db.record_path_at(
            Path::new("/tmp/old"),
            PathKind::Dir,
            HistorySource::Atflow,
            100,
        )
        .unwrap();
        db.record_path_at(
            Path::new("/tmp/new"),
            PathKind::Dir,
            HistorySource::ShellCdHook,
            200,
        )
        .unwrap();
        db.record_path_at(
            Path::new("/tmp/file.rs"),
            PathKind::File,
            HistorySource::Atflow,
            300,
        )
        .unwrap();

        let recent = db.recent_dirs(10).unwrap();

        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].path, PathBuf::from("/tmp/new"));
        assert_eq!(recent[0].source, HistorySource::ShellCdHook);
        assert_eq!(recent[1].path, PathBuf::from("/tmp/old"));
    }

    #[test]
    fn updates_existing_path_count_and_time() {
        let db = HistoryDb::open_memory().unwrap();
        db.record_path_at(
            Path::new("/tmp/project"),
            PathKind::Dir,
            HistorySource::Atflow,
            100,
        )
        .unwrap();
        db.record_path_at(
            Path::new("/tmp/project"),
            PathKind::Dir,
            HistorySource::ShellCdHook,
            300,
        )
        .unwrap();

        let recent = db.recent_dirs(10).unwrap();

        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].last_opened_at, 300);
        assert_eq!(recent[0].open_count, 2);
        assert_eq!(recent[0].source, HistorySource::ShellCdHook);
    }

    #[test]
    fn sorts_tied_timestamps_by_open_count() {
        let db = HistoryDb::open_memory().unwrap();
        db.record_path_at(
            Path::new("/tmp/less-opened"),
            PathKind::Dir,
            HistorySource::Atflow,
            500,
        )
        .unwrap();
        db.record_path_at(
            Path::new("/tmp/more-opened"),
            PathKind::Dir,
            HistorySource::Atflow,
            100,
        )
        .unwrap();
        db.record_path_at(
            Path::new("/tmp/more-opened"),
            PathKind::Dir,
            HistorySource::ShellCdHook,
            500,
        )
        .unwrap();

        let recent = db.recent_dirs(10).unwrap();

        assert_eq!(recent[0].path, PathBuf::from("/tmp/more-opened"));
        assert_eq!(recent[0].open_count, 2);
        assert_eq!(recent[1].path, PathBuf::from("/tmp/less-opened"));
        assert_eq!(recent[1].open_count, 1);
    }

    #[test]
    fn conversion_methods_use_spec_names() {
        assert_eq!(PathKind::from_str("file"), PathKind::File);
        assert_eq!(PathKind::from_str("dir"), PathKind::Dir);
        assert_eq!(
            HistorySource::from_str("shell_cd_hook"),
            HistorySource::ShellCdHook
        );
        assert_eq!(
            HistorySource::from_str("manual_root_scan"),
            HistorySource::ManualRootScan
        );
        assert_eq!(HistorySource::from_str("unknown"), HistorySource::Atflow);
    }

    #[test]
    fn conversion_types_implement_from_str_trait() {
        assert_eq!("file".parse::<PathKind>().unwrap(), PathKind::File);
        assert_eq!("dir".parse::<PathKind>().unwrap(), PathKind::Dir);
        assert_eq!(
            "shell_cd_hook".parse::<HistorySource>().unwrap(),
            HistorySource::ShellCdHook
        );
        assert_eq!(
            "manual_root_scan".parse::<HistorySource>().unwrap(),
            HistorySource::ManualRootScan
        );
        assert_eq!(
            "unknown".parse::<HistorySource>().unwrap(),
            HistorySource::Atflow
        );
    }
}
