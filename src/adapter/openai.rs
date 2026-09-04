use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures_util::{StreamExt, TryStreamExt};
use reqwest::Client;
use serde_json::{json, Value};
use tracing::debug;

use super::{AdapterError, InferenceAdapter, ItemStream, StreamItem};
use crate::models::{Message, ToolDefinition};

#[derive(Clone)]
pub struct OpenAIAdapter {
    client: Client,
    api_key: Option<String>,
}

impl OpenAIAdapter {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: Client::builder().build().unwrap_or_default(),
            api_key,
        }
    }
}

#[async_trait]
impl InferenceAdapter for OpenAIAdapter {
    async fn stream_chat(
        &self,
        endpoint_url: &str,
        model: &str,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<ItemStream, AdapterError> {
        let clean_endpoint = endpoint_url.trim_end_matches('/');
        let url = if clean_endpoint.ends_with("/chat/completions") {
            clean_endpoint.to_string()
        } else if clean_endpoint.ends_with("/v1") {
            format!("{}/chat/completions", clean_endpoint)
        } else {
            format!("{}/v1/chat/completions", clean_endpoint)
        };

        let mut payload = json!({
            "model": model,
            "messages": messages,
            "stream": true
        });

        if !tools.is_empty() {
            payload["tools"] = json!(tools);
        }

        let mut req = self.client.post(&url).json(&payload);

        if let Some(ref key) = self.api_key {
            req = req.bearer_auth(key);
        }

        let res = req
            .send()
            .await
            .map_err(|e| AdapterError::Network(e.to_string()))?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AdapterError::Provider(format!(
                "Upstream returned HTTP {}: {}",
                status, body
            )));
        }

        let event_stream = res.bytes_stream().eventsource();

        let mapped_stream = event_stream
            .map_err(|e| AdapterError::Decode(e.to_string()))
            .map(|event_result| {
                match event_result {
                    Err(e) => vec![Err(e)],
                    Ok(event) => {
                        let data = event.data.trim();
                        if data.is_empty() {
                            return vec![];
                        }
                        if data == "[DONE]" {
                            return vec![Ok(StreamItem::Finish {
                                reason: "stop".to_string(),
                            })];
                        }

                        let parsed: Value = match serde_json::from_str(data) {
                            Ok(v) => v,
                            Err(e) => {
                                debug!("Failed to parse SSE JSON: {}", e);
                                return vec![];
                            }
                        };

                        let choice = match parsed.get("choices").and_then(|c| c.get(0)) {
                            Some(c) => c,
                            None => return vec![],
                        };

                        let mut items = Vec::new();

                        // 1. Text token delta
                        if let Some(content) = choice
                            .get("delta")
                            .and_then(|d| d.get("content"))
                            .and_then(|c| c.as_str())
                        {
                            if !content.is_empty() {
                                items.push(Ok(StreamItem::Token(content.to_string())));
                            }
                        }

                        // 2. Parallel tool call deltas (handles any number of simultaneous tools)
                        if let Some(tool_calls) = choice
                            .get("delta")
                            .and_then(|d| d.get("tool_calls"))
                            .and_then(|tc| tc.as_array())
                        {
                            for tc in tool_calls {
                                let index = tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                                let id = tc.get("id").and_then(|i| i.as_str()).map(|s| s.to_string());
                                let name = tc
                                    .get("function")
                                    .and_then(|f| f.get("name"))
                                    .and_then(|n| n.as_str())
                                    .map(|s| s.to_string());
                                let args_delta = tc
                                    .get("function")
                                    .and_then(|f| f.get("arguments"))
                                    .and_then(|a| a.as_str())
                                    .unwrap_or("")
                                    .to_string();

                                items.push(Ok(StreamItem::ToolCallDelta {
                                    index,
                                    id,
                                    name,
                                    arguments_delta: args_delta,
                                }));
                            }
                        }

                        // 3. Finish reason
                        if let Some(reason) = choice.get("finish_reason").and_then(|r| r.as_str()) {
                            if !reason.is_empty() && reason != "null" {
                                items.push(Ok(StreamItem::Finish {
                                    reason: reason.to_string(),
                                }));
                            }
                        }

                        items
                    }
                }
            })
            .flat_map(futures_util::stream::iter);

        Ok(Box::pin(mapped_stream))
    }

    async fn health_check(&self, endpoint_url: &str) -> bool {
        let clean = endpoint_url.trim_end_matches('/');
        let url = format!("{}/health", clean);
        match self.client.get(&url).send().await {
            Ok(res) => res.status().is_success(),
            Err(_) => false,
        }
    }
}
