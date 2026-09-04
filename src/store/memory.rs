use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

use super::{StateStore, StoreError};
use crate::models::Episode;

#[derive(Clone)]
pub struct InMemoryStore {
    episodes: Arc<DashMap<String, Episode>>,
    max_episodes: usize,
    ttl_millis: Option<i64>,
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self::with_options(50_000, Some(86400 * 7)) // 50,000 episodes, 7 days default TTL
    }

    pub fn with_options(max_episodes: usize, ttl_secs: Option<u64>) -> Self {
        Self {
            episodes: Arc::new(DashMap::new()),
            max_episodes: max_episodes.max(10),
            ttl_millis: ttl_secs.map(|s| (s as i64) * 1000),
        }
    }

    /// Enforce LRU eviction if store exceeds capacity limit
    fn enforce_capacity(&self) {
        if self.episodes.len() >= self.max_episodes {
            // Find oldest updated episode to evict
            let oldest = self
                .episodes
                .iter()
                .min_by_key(|entry| entry.value().updated_at)
                .map(|entry| entry.key().clone());

            if let Some(oldest_id) = oldest {
                self.episodes.remove(&oldest_id);
            }
        }
    }

    /// Evict expired episodes based on TTL
    pub fn cleanup_expired(&self) {
        if let Some(ttl) = self.ttl_millis {
            let threshold = Utc::now().timestamp_millis() - ttl;
            self.episodes.retain(|_, ep| ep.updated_at >= threshold);
        }
    }
}

#[async_trait]
impl StateStore for InMemoryStore {
    async fn create_episode(&self, episode: Episode) -> Result<Episode, StoreError> {
        self.enforce_capacity();
        let id = episode.id.clone();
        self.episodes.insert(id, episode.clone());
        Ok(episode)
    }

    async fn get_episode(&self, id: &str) -> Result<Option<Episode>, StoreError> {
        self.cleanup_expired();
        Ok(self.episodes.get(id).map(|r| r.value().clone()))
    }

    async fn update_episode(&self, mut episode: Episode) -> Result<Episode, StoreError> {
        let id = episode.id.clone();
        if !self.episodes.contains_key(&id) {
            return Err(StoreError::NotFound(id));
        }
        episode.updated_at = Utc::now().timestamp_millis();
        self.episodes.insert(id, episode.clone());
        Ok(episode)
    }

    async fn delete_episode(&self, id: &str) -> Result<bool, StoreError> {
        Ok(self.episodes.remove(id).is_some())
    }

    async fn list_episodes(&self, limit: usize, offset: usize) -> Result<Vec<Episode>, StoreError> {
        self.cleanup_expired();
        let mut all: Vec<Episode> = self
            .episodes
            .iter()
            .map(|entry| entry.value().clone())
            .collect();

        all.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        let res = all.into_iter().skip(offset).take(limit).collect();
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_store_crud() {
        let store = InMemoryStore::new();
        let ep = Episode::new("gpt-4o", Some("You are helpful.".to_string()));
        let ep_id = ep.id.clone();

        let created = store.create_episode(ep).await.unwrap();
        assert_eq!(created.id, ep_id);

        let retrieved = store.get_episode(&ep_id).await.unwrap().unwrap();
        assert_eq!(retrieved.model, "gpt-4o");

        let list = store.list_episodes(10, 0).await.unwrap();
        assert_eq!(list.len(), 1);

        let deleted = store.delete_episode(&ep_id).await.unwrap();
        assert!(deleted);

        let missing = store.get_episode(&ep_id).await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_in_memory_store_capacity_eviction() {
        let store = InMemoryStore::with_options(10, None);

        // Fill 10 items
        for i in 0..10 {
            let mut ep = Episode::new(format!("model-{}", i), None);
            ep.updated_at = 1000 + i;
            store.create_episode(ep).await.unwrap();
        }

        assert_eq!(store.list_episodes(50, 0).await.unwrap().len(), 10);

        // Add 11th item
        let mut ep_new = Episode::new("model-new", None);
        ep_new.updated_at = 2000;
        store.create_episode(ep_new).await.unwrap();

        // Oldest item should be evicted so length stays 10
        let list = store.list_episodes(50, 0).await.unwrap();
        assert_eq!(list.len(), 10);
        assert!(list.iter().any(|e| e.model == "model-new"));
        assert!(!list.iter().any(|e| e.model == "model-0"));
    }
}
