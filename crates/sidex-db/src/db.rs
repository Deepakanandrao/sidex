//! SQLite database connection with automatic migrations.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::Connection;

/// Wraps a SQLite connection and ensures schema migrations run on open.
pub struct Database {
    conn: Connection,
    path: PathBuf,
}

impl Database {
    /// Opens (or creates) a database at `path` and runs migrations.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("failed to open database at {}", path.display()))?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .context("failed to set pragmas")?;

        let mut db = Self {
            conn,
            path: path.to_path_buf(),
        };
        db.run_migrations()?;
        Ok(db)
    }

    /// Opens the database at the default path: `~/.sidex/state.db`.
    pub fn open_default() -> Result<Self> {
        let dir = dirs::home_dir()
            .context("could not determine home directory")?
            .join(".sidex");
        Self::open(&dir.join("state.db"))
    }

    /// Returns a reference to the underlying `rusqlite::Connection`.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Returns the path this database was opened from.
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn run_migrations(&mut self) -> Result<()> {
        self.conn
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS state_kv (
                    scope   TEXT NOT NULL,
                    key     TEXT NOT NULL,
                    value   TEXT NOT NULL,
                    PRIMARY KEY (scope, key)
                );

                CREATE TABLE IF NOT EXISTS recent_files (
                    path        TEXT PRIMARY KEY,
                    last_opened TEXT NOT NULL DEFAULT (datetime('now'))
                );

                CREATE TABLE IF NOT EXISTS recent_workspaces (
                    path        TEXT PRIMARY KEY,
                    last_opened TEXT NOT NULL DEFAULT (datetime('now'))
                );

                CREATE TABLE IF NOT EXISTS window_state (
                    id              INTEGER PRIMARY KEY CHECK (id = 1),
                    x               INTEGER NOT NULL,
                    y               INTEGER NOT NULL,
                    width           INTEGER NOT NULL,
                    height          INTEGER NOT NULL,
                    is_maximized    INTEGER NOT NULL DEFAULT 0,
                    sidebar_width   REAL    NOT NULL DEFAULT 260.0,
                    panel_height    REAL    NOT NULL DEFAULT 200.0,
                    active_editor   TEXT
                );
                ",
            )
            .context("failed to run migrations")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db = Database::open(&tmp.path().join("test.db")).unwrap();
        assert!(db.path().exists());
    }

    #[test]
    fn migrations_are_idempotent() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("test.db");
        let _db1 = Database::open(&path).unwrap();
        let _db2 = Database::open(&path).unwrap();
    }
}
