use async_trait::async_trait;
use futures_util::stream;
use parking_lot::Mutex;
use std::sync::Arc;

use super::{AdapterError, InferenceAdapter, ItemStream, StreamItem};
use crate::models::{Message, ToolDefinition};

#[derive(Clone, Debug)]
pub struct MockToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Clone, Debug)]
pub enum MockBehavior {
    TextReply(String),
    ToolCall {
        id: String,
        name: String,
        arguments: String,
    },
    ParallelToolCalls(Vec<MockToolCall>),
    Sequential(Vec<MockBehavior>),
}

#[derive(Clone)]
pub struct MockAdapter {
    behaviors: Arc<Mutex<Vec<MockBehavior>>>,
}

impl MockAdapter {
    pub fn new() -> Self {
        Self {
            behaviors: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_behavior(behavior: MockBehavior) -> Self {
        let adapter = Self::new();
        adapter.add_behavior(behavior);
        adapter
    }

    pub fn add_behavior(&self, behavior: MockBehavior) {
        self.behaviors.lock().push(behavior);
    }
}

#[async_trait]
impl InferenceAdapter for MockAdapter {
    async fn stream_chat(
        &self,
        _endpoint_url: &str,
        _model: &str,
        _messages: &[Message],
        _tools: &[ToolDefinition],
    ) -> Result<ItemStream, AdapterError> {
        let behavior = {
            let mut list = self.behaviors.lock();
            if list.is_empty() {
                MockBehavior::TextReply("Mock default answer.".to_string())
            } else {
                list.remove(0)
            }
        };

        let mut items: Vec<Result<StreamItem, AdapterError>> = Vec::new();

        match behavior {
            MockBehavior::TextReply(text) => {
                // Split words into tokens
                for word in text.split_inclusive(' ') {
                    items.push(Ok(StreamItem::Token(word.to_string())));
                }
                items.push(Ok(StreamItem::Finish {
                    reason: "stop".to_string(),
                }));
            }
            MockBehavior::ToolCall {
                id,
                name,
                arguments,
            } => {
                items.push(Ok(StreamItem::ToolCallDelta {
                    index: 0,
                    id: Some(id),
                    name: Some(name),
                    arguments_delta: arguments,
                }));
                items.push(Ok(StreamItem::Finish {
                    reason: "tool_calls".to_string(),
                }));
            }
            MockBehavior::ParallelToolCalls(calls) => {
                for (idx, call) in calls.into_iter().enumerate() {
                    items.push(Ok(StreamItem::ToolCallDelta {
                        index: idx,
                        id: Some(call.id),
                        name: Some(call.name),
                        arguments_delta: call.arguments,
                    }));
                }
                items.push(Ok(StreamItem::Finish {
                    reason: "tool_calls".to_string(),
                }));
            }
            MockBehavior::Sequential(_) => {
                items.push(Ok(StreamItem::Token("Sequential step".to_string())));
                items.push(Ok(StreamItem::Finish {
                    reason: "stop".to_string(),
                }));
            }
        }

        Ok(Box::pin(stream::iter(items)))
    }

    async fn health_check(&self, _endpoint_url: &str) -> bool {
        true
    }
}
