use std::pin::Pin;
use async_trait::async_trait;
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::models::{Message, ToolDefinition};

pub mod mock;
pub mod openai;

pub use mock::{MockAdapter, MockBehavior, MockToolCall};
pub use openai::OpenAIAdapter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamItem {
    Token(String),
    ToolCallDelta {
        index: usize,
        id: Option<String>,
        name: Option<String>,
        arguments_delta: String,
    },
    Finish {
        reason: String,
    },
}

#[derive(Error, Debug)]
pub enum AdapterError {
    #[error("Network / HTTP error: {0}")]
    Network(String),
    #[error("Stream decoding error: {0}")]
    Decode(String),
    #[error("Provider error: {0}")]
    Provider(String),
}

pub type ItemStream = Pin<Box<dyn Stream<Item = Result<StreamItem, AdapterError>> + Send>>;

#[async_trait]
pub trait InferenceAdapter: Send + Sync {
    async fn stream_chat(
        &self,
        endpoint_url: &str,
        model: &str,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<ItemStream, AdapterError>;

    async fn health_check(&self, endpoint_url: &str) -> bool;
}
