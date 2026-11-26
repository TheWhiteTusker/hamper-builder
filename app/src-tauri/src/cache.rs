//! The on-device copy of everything the app has read, in SQLite, so every
//! screen opens with the last copy instantly and refreshes in the background.
//! Scoped by user, so two people on one machine never see each other's view.

use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use std::sync::Mutex;

/// `None` when SQLite could not open at all: every read misses and the app
/// simply starts cold, rather than failing.
pub struct Cache {
    conn: Option<Mutex<Connection>>,
}

const SCHEMA: &str = "
    pragma journal_mode = wal;
    create table if not exists kv (
        scope      text not null,
        key        text not null,
        value      text not null,
        fetched_at integer not null,
        primary key (scope, key)
    );";

fn open_with_schema(path: Option<&Path>) -> rusqlite::Result<Connection> {
    let conn = match path {
        Some(p) => Connection::open(p)?,
        None => Connection::open_in_memory()?,
    };
    conn.execute_batch(SCHEMA)?;
    Ok(conn)
}

impl Cache {
    pub fn open(path: &Path) -> Self {
        let conn = open_with_schema(Some(path))
            .or_else(|e| {
                tracing::error!("cache at {} unavailable, using memory: {e}", path.display());
                open_with_schema(None)
            })
            .map_err(|e| tracing::error!("no cache at all: {e}"))
            .ok();
        Self { conn: conn.map(Mutex::new) }
    }

    pub fn get(&self, scope: &str, key: &str) -> Option<(String, i64)> {
        let conn = self.conn.as_ref()?.lock().ok()?;
        let row = conn
            .query_row("select value, fetched_at from kv where scope = ?1 and key = ?2", params![scope, key], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .optional();
        row.unwrap_or_else(|e| {
            tracing::warn!("cache read {scope}/{key}: {e}");
            None
        })
    }

    pub fn put(&self, scope: &str, key: &str, value: &str, fetched_at: i64) {
        let Some(Ok(conn)) = self.conn.as_ref().map(Mutex::lock) else { return };
        if let Err(e) = conn.execute(
            "insert into kv (scope, key, value, fetched_at) values (?1, ?2, ?3, ?4)
             on conflict (scope, key) do update set value = excluded.value, fetched_at = excluded.fetched_at",
            params![scope, key, value, fetched_at],
        ) {
            tracing::warn!("cache write {scope}/{key}: {e}");
        }
    }

    pub fn remove(&self, scope: &str, key: &str) {
        let Some(Ok(conn)) = self.conn.as_ref().map(Mutex::lock) else { return };
        let _ = conn.execute("delete from kv where scope = ?1 and key = ?2", params![scope, key]);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn put_get_overwrite_remove() {
        let dir = std::env::temp_dir().join(format!("lattice-cache-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cache = Cache::open(&dir.join("c.sqlite"));
        assert_eq!(cache.get("u1", "dashboard"), None);
        cache.put("u1", "dashboard", "{\"a\":1}", 10);
        cache.put("u1", "dashboard", "{\"a\":2}", 20);
        assert_eq!(cache.get("u1", "dashboard"), Some(("{\"a\":2}".into(), 20)));
        assert_eq!(cache.get("u2", "dashboard"), None, "scoped per user");
        cache.remove("u1", "dashboard");
        assert_eq!(cache.get("u1", "dashboard"), None);
        drop(cache);
        let _ = std::fs::remove_dir_all(dir);
    }
}
