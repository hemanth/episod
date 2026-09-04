use async_trait::async_trait;
use redis::AsyncCommands;
use redis::aio::ConnectionManager;

use crate::models::Episode;
use crate::store::{StateStore, StoreError};

#[derive(Clone)]
pub struct RedisStore {
    conn: ConnectionManager,
    key_prefix: String,
    ttl_seconds: u64,
}

impl RedisStore {
    pub async fn new(url: &str) -> Result<Self, StoreError> {
        let client = redis::Client::open(url)
            .map_err(|e| StoreError::Internal(format!("Invalid Redis URL: {}", e)))?;
        let conn = ConnectionManager::new(client)
            .await
            .map_err(|e| StoreError::Internal(format!("Failed to connect to Redis: {}", e)))?;

        Ok(Self {
            conn,
            key_prefix: "episod:episodes:".to_string(),
            ttl_seconds: 604_800, // 7 days default retention
        })
    }

    pub fn with_ttl(mut self, ttl_seconds: u64) -> Self {
        self.ttl_seconds = ttl_seconds;
        self
    }

    fn episode_key(&self, id: &str) -> String {
        format!("{}{}", self.key_prefix, id)
    }

    fn index_key(&self) -> String {
        format!("{}index", self.key_prefix)
    }
}

#[async_trait]
impl StateStore for RedisStore {
    async fn create_episode(&self, episode: Episode) -> Result<Episode, StoreError> {
        let mut conn = self.conn.clone();
        let key = self.episode_key(&episode.id);
        let index_key = self.index_key();

        let json = serde_json::to_string(&episode)
            .map_err(|e| StoreError::Internal(format!("Serialization error: {}", e)))?;

        if self.ttl_seconds > 0 {
            conn.set_ex::<_, _, ()>(&key, &json, self.ttl_seconds)
                .await
                .map_err(|e| StoreError::Internal(e.to_string()))?;
        } else {
            conn.set::<_, _, ()>(&key, &json)
                .await
                .map_err(|e| StoreError::Internal(e.to_string()))?;
        }

        // Add to sorted set index by updated_at timestamp
        let score = episode.updated_at as f64;
        let _: () = conn
            .zadd(index_key, &episode.id, score)
            .await
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        Ok(episode)
    }

    async fn get_episode(&self, id: &str) -> Result<Option<Episode>, StoreError> {
        let mut conn = self.conn.clone();
        let key = self.episode_key(id);

        let raw: Option<String> = conn
            .get(&key)
            .await
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        match raw {
            Some(data) => {
                let episode: Episode = serde_json::from_str(&data)
                    .map_err(|e| StoreError::Internal(format!("Deserialization error: {}", e)))?;
                Ok(Some(episode))
            }
            None => Ok(None),
        }
    }

    async fn update_episode(&self, episode: Episode) -> Result<Episode, StoreError> {
        self.create_episode(episode).await
    }

    async fn delete_episode(&self, id: &str) -> Result<bool, StoreError> {
        let mut conn = self.conn.clone();
        let key = self.episode_key(id);
        let index_key = self.index_key();

        let deleted: usize = conn
            .del(&key)
            .await
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        let _: () = conn
            .zrem(index_key, id)
            .await
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        Ok(deleted > 0)
    }

    async fn list_episodes(&self, limit: usize, offset: usize) -> Result<Vec<Episode>, StoreError> {
        let mut conn = self.conn.clone();
        let index_key = self.index_key();

        // Retrieve IDs in reverse chronological order (newest first)
        let start = offset as isize;
        let stop = (offset + limit).saturating_sub(1) as isize;

        let ids: Vec<String> = conn
            .zrevrange(index_key, start, stop)
            .await
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        let mut episodes = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(ep) = self.get_episode(&id).await? {
                episodes.push(ep);
            }
        }

        Ok(episodes)
    }
}
