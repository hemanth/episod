use async_trait::async_trait;
use thiserror::Error;

use crate::models::Episode;

pub mod memory;

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("Episode not found: {0}")]
    NotFound(String),
    #[error("Storage error: {0}")]
    Internal(String),
}

#[async_trait]
pub trait StateStore: Send + Sync {
    async fn create_episode(&self, episode: Episode) -> Result<Episode, StoreError>;
    async fn get_episode(&self, id: &str) -> Result<Option<Episode>, StoreError>;
    async fn update_episode(&self, episode: Episode) -> Result<Episode, StoreError>;
    async fn delete_episode(&self, id: &str) -> Result<bool, StoreError>;
    async fn list_episodes(&self, limit: usize, offset: usize) -> Result<Vec<Episode>, StoreError>;
}
