//! Generic key-value state store backed by SQLite.
//!
//! Keys are scoped so that different subsystems (global settings, per-workspace
//! data, per-extension data) can coexist without collision.  Typical scopes:
//!
//! - `"global"`
//! - `"workspace:<path>"`
//! - `"extension:<id>"`

use anyhow::{Context, Result};
use rusqlite::params;

use crate::db::Database;

/// Scoped key-value state store.
pub struct StateStore<'db> {
    db: &'db Database,
}

impl<'db> StateStore<'db> {
    /// Creates a `StateStore` backed by the given database.
    pub fn new(db: &'db Database) -> Self {
        Self { db }
    }

    /// Retrieves a value for `(scope, key)`, returning `None` if absent.
    pub fn get(&self, scope: &str, key: &str) -> Result<Option<String>> {
        let mut stmt = self
            .db
            .conn()
            .prepare_cached("SELECT value FROM state_kv WHERE scope = ?1 AND key = ?2")
            .context("prepare get")?;

        let result = stmt
            .query_row(params![scope, key], |row| row.get::<_, String>(0))
            .optional()
            .context("query get")?;

        Ok(result)
    }

    /// Sets (upserts) the value for `(scope, key)`.
    pub fn set(&self, scope: &str, key: &str, value: &str) -> Result<()> {
        self.db
            .conn()
            .execute(
                "INSERT INTO state_kv (scope, key, value) VALUES (?1, ?2, ?3)
                 ON CONFLICT(scope, key) DO UPDATE SET value = excluded.value",
                params![scope, key, value],
            )
            .context("upsert state")?;
        Ok(())
    }

    /// Deletes the entry for `(scope, key)`.
    pub fn delete(&self, scope: &str, key: &str) -> Result<()> {
        self.db
            .conn()
            .execute(
                "DELETE FROM state_kv WHERE scope = ?1 AND key = ?2",
                params![scope, key],
            )
            .context("delete state")?;
        Ok(())
    }

    /// Returns all keys in the given scope.
    pub fn keys(&self, scope: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .db
            .conn()
            .prepare_cached("SELECT key FROM state_kv WHERE scope = ?1 ORDER BY key")
            .context("prepare keys")?;

        let rows = stmt
            .query_map(params![scope], |row| row.get::<_, String>(0))
            .context("query keys")?;

        let mut keys = Vec::new();
        for row in rows {
            keys.push(row.context("read key row")?);
        }
        Ok(keys)
    }
}

/// Extension trait on [`rusqlite::Statement`] results for optional single-row queries.
trait OptionalExt<T> {
    fn optional(self) -> rusqlite::Result<Option<T>>;
}

impl<T> OptionalExt<T> for rusqlite::Result<T> {
    fn optional(self) -> rusqlite::Result<Option<T>> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        let tmp = tempfile::TempDir::new().unwrap();
        Database::open(&tmp.path().join("test.db")).unwrap()
    }

    #[test]
    fn get_missing_returns_none() {
        let db = test_db();
        let store = StateStore::new(&db);
        assert!(store.get("global", "missing").unwrap().is_none());
    }

    #[test]
    fn set_and_get() {
        let db = test_db();
        let store = StateStore::new(&db);
        store.set("global", "theme", "dark").unwrap();
        assert_eq!(store.get("global", "theme").unwrap().unwrap(), "dark");
    }

    #[test]
    fn upsert_overwrites() {
        let db = test_db();
        let store = StateStore::new(&db);
        store.set("global", "k", "v1").unwrap();
        store.set("global", "k", "v2").unwrap();
        assert_eq!(store.get("global", "k").unwrap().unwrap(), "v2");
    }

    #[test]
    fn delete_key() {
        let db = test_db();
        let store = StateStore::new(&db);
        store.set("global", "k", "v").unwrap();
        store.delete("global", "k").unwrap();
        assert!(store.get("global", "k").unwrap().is_none());
    }

    #[test]
    fn keys_in_scope() {
        let db = test_db();
        let store = StateStore::new(&db);
        store.set("ws:/proj", "a", "1").unwrap();
        store.set("ws:/proj", "b", "2").unwrap();
        store.set("global", "c", "3").unwrap();
        let keys = store.keys("ws:/proj").unwrap();
        assert_eq!(keys, vec!["a", "b"]);
    }

    #[test]
    fn scopes_are_isolated() {
        let db = test_db();
        let store = StateStore::new(&db);
        store.set("global", "k", "global_v").unwrap();
        store.set("extension:foo", "k", "ext_v").unwrap();
        assert_eq!(store.get("global", "k").unwrap().unwrap(), "global_v");
        assert_eq!(
            store.get("extension:foo", "k").unwrap().unwrap(),
            "ext_v"
        );
    }
}
