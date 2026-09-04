use async_trait::async_trait;
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::Arc;

use super::{StateStore, StoreError};
use crate::models::Episode;

#[derive(Clone)]
pub struct SqliteStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteStore {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, StoreError> {
        let conn = Connection::open(path)
            .map_err(|e| StoreError::Internal(format!("Failed to open SQLite database: {}", e)))?;

        // Configure high-performance WAL mode and pragmas
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;
             PRAGMA mmap_size = 268435456;",
        )
        .map_err(|e| StoreError::Internal(format!("Failed to set SQLite pragmas: {}", e)))?;

        // Initialize schema
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS episodes (
                id TEXT PRIMARY KEY,
                model TEXT NOT NULL,
                system_prompt TEXT,
                active_leaf_id TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                data TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_episodes_updated_at ON episodes(updated_at DESC);",
        )
        .map_err(|e| StoreError::Internal(format!("Failed to create schema: {}", e)))?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| StoreError::Internal(format!("Failed to open in-memory SQLite: {}", e)))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS episodes (
                id TEXT PRIMARY KEY,
                model TEXT NOT NULL,
                system_prompt TEXT,
                active_leaf_id TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                data TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_episodes_updated_at ON episodes(updated_at DESC);",
        )
        .map_err(|e| StoreError::Internal(format!("Failed to create schema: {}", e)))?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

#[async_trait]
impl StateStore for SqliteStore {
    async fn create_episode(&self, episode: Episode) -> Result<Episode, StoreError> {
        let conn = self.conn.clone();
        let ep = episode.clone();

        tokio::task::spawn_blocking(move || {
            let data_json = serde_json::to_string(&ep)
                .map_err(|e| StoreError::Internal(format!("Failed to serialize episode: {}", e)))?;

            let lock = conn.lock();
            lock.execute(
                "INSERT INTO episodes (id, model, system_prompt, active_leaf_id, created_at, updated_at, data)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    ep.id,
                    ep.model,
                    ep.system_prompt,
                    ep.active_leaf_id,
                    ep.created_at,
                    ep.updated_at,
                    data_json
                ],
            )
            .map_err(|e| StoreError::Internal(format!("Failed to insert episode: {}", e)))?;

            Ok(ep)
        })
        .await
        .map_err(|e| StoreError::Internal(format!("Task spawn error: {}", e)))?
    }

    async fn get_episode(&self, id: &str) -> Result<Option<Episode>, StoreError> {
        let conn = self.conn.clone();
        let id_str = id.to_string();

        tokio::task::spawn_blocking(move || {
            let lock = conn.lock();
            let mut stmt = lock
                .prepare("SELECT data FROM episodes WHERE id = ?1")
                .map_err(|e| StoreError::Internal(format!("Failed to prepare select: {}", e)))?;

            let mut rows = stmt
                .query(params![id_str])
                .map_err(|e| StoreError::Internal(format!("Query failed: {}", e)))?;

            if let Some(row) = rows
                .next()
                .map_err(|e| StoreError::Internal(e.to_string()))?
            {
                let data_str: String = row
                    .get(0)
                    .map_err(|e| StoreError::Internal(e.to_string()))?;
                let episode: Episode = serde_json::from_str(&data_str)
                    .map_err(|e| StoreError::Internal(format!("Failed to deserialize: {}", e)))?;
                Ok(Some(episode))
            } else {
                Ok(None)
            }
        })
        .await
        .map_err(|e| StoreError::Internal(format!("Task spawn error: {}", e)))?
    }

    async fn update_episode(&self, episode: Episode) -> Result<Episode, StoreError> {
        let conn = self.conn.clone();
        let ep = episode.clone();

        tokio::task::spawn_blocking(move || {
            let data_json = serde_json::to_string(&ep)
                .map_err(|e| StoreError::Internal(format!("Failed to serialize episode: {}", e)))?;

            let lock = conn.lock();
            let rows_affected = lock
                .execute(
                    "UPDATE episodes SET 
                        model = ?2,
                        system_prompt = ?3,
                        active_leaf_id = ?4,
                        updated_at = ?5,
                        data = ?6
                     WHERE id = ?1",
                    params![
                        ep.id,
                        ep.model,
                        ep.system_prompt,
                        ep.active_leaf_id,
                        ep.updated_at,
                        data_json
                    ],
                )
                .map_err(|e| StoreError::Internal(format!("Failed to update episode: {}", e)))?;

            if rows_affected == 0 {
                Err(StoreError::NotFound(ep.id))
            } else {
                Ok(ep)
            }
        })
        .await
        .map_err(|e| StoreError::Internal(format!("Task spawn error: {}", e)))?
    }

    async fn delete_episode(&self, id: &str) -> Result<bool, StoreError> {
        let conn = self.conn.clone();
        let id_str = id.to_string();

        tokio::task::spawn_blocking(move || {
            let lock = conn.lock();
            let rows = lock
                .execute("DELETE FROM episodes WHERE id = ?1", params![id_str])
                .map_err(|e| StoreError::Internal(format!("Failed to delete: {}", e)))?;
            Ok(rows > 0)
        })
        .await
        .map_err(|e| StoreError::Internal(format!("Task spawn error: {}", e)))?
    }

    async fn list_episodes(&self, limit: usize, offset: usize) -> Result<Vec<Episode>, StoreError> {
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || {
            let lock = conn.lock();
            let mut stmt = lock
                .prepare("SELECT data FROM episodes ORDER BY updated_at DESC LIMIT ?1 OFFSET ?2")
                .map_err(|e| StoreError::Internal(format!("Failed to prepare query: {}", e)))?;

            let rows = stmt
                .query_map(params![limit as i64, offset as i64], |row| {
                    let data: String = row.get(0)?;
                    Ok(data)
                })
                .map_err(|e| StoreError::Internal(format!("Query failed: {}", e)))?;

            let mut episodes = Vec::new();
            for data_res in rows {
                let data_str = data_res.map_err(|e| StoreError::Internal(e.to_string()))?;
                let ep: Episode = serde_json::from_str(&data_str)
                    .map_err(|e| StoreError::Internal(format!("Failed to parse: {}", e)))?;
                episodes.push(ep);
            }

            Ok(episodes)
        })
        .await
        .map_err(|e| StoreError::Internal(format!("Task spawn error: {}", e)))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sqlite_store_crud() {
        let store = SqliteStore::in_memory().unwrap();

        let ep = Episode::new("test-model", Some("You are helpful.".into()));
        let ep_id = ep.id.clone();

        let created = store.create_episode(ep).await.unwrap();
        assert_eq!(created.id, ep_id);

        let retrieved = store.get_episode(&ep_id).await.unwrap();
        assert!(retrieved.is_some());
        let mut ep_retrieved = retrieved.unwrap();
        assert_eq!(ep_retrieved.model, "test-model");

        // Add a turn and update
        let node_id = ep_retrieved.append_turn(
            Some(crate::models::Message::user("Hello")),
            Some(crate::models::Message::assistant("World")),
            vec![],
            vec![],
        );
        store.update_episode(ep_retrieved).await.unwrap();

        // Verify update persisted
        let ep_updated = store.get_episode(&ep_id).await.unwrap().unwrap();
        assert_eq!(ep_updated.active_leaf_id, Some(node_id));
        assert_eq!(ep_updated.nodes.len(), 1);

        // List
        let list = store.list_episodes(10, 0).await.unwrap();
        assert_eq!(list.len(), 1);

        // Delete
        let deleted = store.delete_episode(&ep_id).await.unwrap();
        assert!(deleted);
        assert!(store.get_episode(&ep_id).await.unwrap().is_none());
    }
}
