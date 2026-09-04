use async_trait::async_trait;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use reqwest::{Client, StatusCode};

use crate::models::Episode;
use crate::store::{StateStore, StoreError};

#[derive(Clone)]
pub struct HttpStore {
    client: Client,
    endpoint_url: String,
    #[allow(dead_code)]
    auth_header: Option<String>,
}

impl HttpStore {
    pub fn new(endpoint_url: String, auth_header: Option<String>) -> Self {
        let mut headers = HeaderMap::new();
        if let Some(ref token) = auth_header {
            if let Ok(val) = HeaderValue::from_str(token) {
                headers.insert(AUTHORIZATION, val);
            }
        }

        let client = Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        Self {
            client,
            endpoint_url: endpoint_url.trim_end_matches('/').to_string(),
            auth_header,
        }
    }
}

#[async_trait]
impl StateStore for HttpStore {
    async fn create_episode(&self, episode: Episode) -> Result<Episode, StoreError> {
        let res = self
            .client
            .post(&self.endpoint_url)
            .json(&episode)
            .send()
            .await
            .map_err(|e| StoreError::Internal(format!("HTTP create error: {}", e)))?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(StoreError::Internal(format!(
                "HTTP error {}: {}",
                status, body
            )));
        }

        let created: Episode = res.json().await.unwrap_or(episode);

        Ok(created)
    }

    async fn get_episode(&self, id: &str) -> Result<Option<Episode>, StoreError> {
        let url = format!("{}/{}", self.endpoint_url, id);
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| StoreError::Internal(format!("HTTP get error: {}", e)))?;

        if res.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(StoreError::Internal(format!(
                "HTTP error {}: {}",
                status, body
            )));
        }

        let episode: Episode = res
            .json()
            .await
            .map_err(|e| StoreError::Internal(format!("Failed to parse episode JSON: {}", e)))?;

        Ok(Some(episode))
    }

    async fn update_episode(&self, episode: Episode) -> Result<Episode, StoreError> {
        let url = format!("{}/{}", self.endpoint_url, episode.id);
        let res = self
            .client
            .put(&url)
            .json(&episode)
            .send()
            .await
            .map_err(|e| StoreError::Internal(format!("HTTP update error: {}", e)))?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(StoreError::Internal(format!(
                "HTTP error {}: {}",
                status, body
            )));
        }

        let updated: Episode = res.json().await.unwrap_or(episode);

        Ok(updated)
    }

    async fn delete_episode(&self, id: &str) -> Result<bool, StoreError> {
        let url = format!("{}/{}", self.endpoint_url, id);
        let res = self
            .client
            .delete(&url)
            .send()
            .await
            .map_err(|e| StoreError::Internal(format!("HTTP delete error: {}", e)))?;

        if res.status() == StatusCode::NOT_FOUND {
            return Ok(false);
        }

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(StoreError::Internal(format!(
                "HTTP error {}: {}",
                status, body
            )));
        }

        Ok(true)
    }

    async fn list_episodes(&self, limit: usize, offset: usize) -> Result<Vec<Episode>, StoreError> {
        let url = format!("{}?limit={}&offset={}", self.endpoint_url, limit, offset);
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| StoreError::Internal(format!("HTTP list error: {}", e)))?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(StoreError::Internal(format!(
                "HTTP error {}: {}",
                status, body
            )));
        }

        let episodes: Vec<Episode> = res.json().await.unwrap_or_default();

        Ok(episodes)
    }
}
