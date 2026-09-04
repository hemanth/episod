use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Json};
use chrono::Utc;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::json;
use std::convert::Infallible;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use super::AppState;
use super::handlers::CancelOnDropStream;
use crate::adapter::StreamItem;
use crate::guardrails::ContextBudgetManager;
use crate::models::{Episode, Message, TokenUsage};

// ==========================================
// 1. OpenAI Responses API Models
// ==========================================

#[derive(Debug, Deserialize)]
pub struct CreateResponseRequest {
    pub model: String,
    pub input: ResponseInput,
    pub previous_response_id: Option<String>,
    pub instructions: Option<String>,
    #[serde(default)]
    pub tools: Option<Vec<crate::models::ToolDefinition>>,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ResponseInput {
    Text(String),
    Items(Vec<ResponseInputItem>),
}

#[derive(Debug, Deserialize)]
pub struct ResponseInputItem {
    pub role: Option<String>,
    pub content: Option<String>,
}

// ==========================================
// 2. OpenAI Chat Completions Models
// ==========================================

#[derive(Debug, Deserialize)]
pub struct ChatCompletionsRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub tools: Option<Vec<crate::models::ToolDefinition>>,
    #[serde(default)]
    pub stream: bool,
}

// ==========================================
// Handler: POST /v1/responses
// ==========================================

pub async fn handle_responses_api(
    State(state): State<AppState>,
    Json(payload): Json<CreateResponseRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let response_id = format!("resp_{}", Uuid::new_v4().simple());
    let now = Utc::now().timestamp();

    // 1. Resolve prompt text from input
    let user_prompt = match &payload.input {
        ResponseInput::Text(t) => t.clone(),
        ResponseInput::Items(items) => items
            .iter()
            .filter_map(|i| i.content.as_deref())
            .collect::<Vec<_>>()
            .join("\n"),
    };

    // 2. Find or create session using previous_response_id for DAG state hydration
    let mut episode = if let Some(ref prev_id) = payload.previous_response_id {
        // Search store for an episode containing this response/node ID
        let episodes = state.store.list_episodes(50, 0).await.unwrap_or_default();
        let found = episodes.into_iter().find(|e| e.nodes.contains_key(prev_id));
        match found {
            Some(mut ep) => {
                let _ = ep.fork(prev_id);
                ep
            }
            None => {
                let mut ep = Episode::new(&payload.model, payload.instructions.clone());
                ep.active_leaf_id = Some(prev_id.clone());
                ep
            }
        }
    } else {
        Episode::new(&payload.model, payload.instructions.clone())
    };

    let user_msg = Message::user(user_prompt.clone());
    let mut history = episode.linearize_history(None);
    history.push(user_msg.clone());

    let pruned_history = state.context_guardrail.prune_history(history);

    let replica = state.router.route_by_session(&episode.id).ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "No healthy backend replica".into(),
        )
    })?;

    let tools = payload.tools.clone().unwrap_or_default();

    if !payload.stream {
        let _session_guard = state.acquire_session_lock(&episode.id).await;
        // Non-streaming Responses API execution
        let mut stream = state
            .adapter
            .stream_chat(&replica.url, &payload.model, &pruned_history, &tools)
            .await
            .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

        let mut output_text = String::new();
        let mut final_usage: Option<TokenUsage> = None;

        while let Some(item_res) = stream.next().await {
            match item_res {
                Ok(StreamItem::Token(t)) => output_text.push_str(&t),
                Ok(StreamItem::Usage(u)) => final_usage = Some(u),
                _ => {}
            }
        }

        let total_tokens = final_usage
            .as_ref()
            .map(|u| u.total_tokens)
            .unwrap_or_else(|| {
                ContextBudgetManager::estimate_total_tokens(&pruned_history)
                    + ContextBudgetManager::estimate_tokens(&output_text)
            });
        let input_tokens = final_usage.as_ref().map(|u| u.prompt_tokens).unwrap_or(0);
        let output_tokens = final_usage
            .as_ref()
            .map(|u| u.completion_tokens)
            .unwrap_or(total_tokens.saturating_sub(input_tokens));

        // Save node in episode DAG
        let assistant_msg = Message::assistant(output_text.clone());
        let node_id = episode.append_turn_with_telemetry(
            Some(user_msg),
            Some(assistant_msg),
            vec![],
            vec![],
            final_usage,
            None,
        );
        let _ = state.store.update_episode(episode).await;

        let res_json = json!({
            "id": node_id,
            "object": "response",
            "created_at": now,
            "model": payload.model,
            "status": "completed",
            "output": [
                {
                    "id": format!("msg_{}", Uuid::new_v4().simple()),
                    "type": "message",
                    "role": "assistant",
                    "content": [
                        {
                            "type": "output_text",
                            "text": output_text
                        }
                    ]
                }
            ],
            "usage": {
                "total_tokens": total_tokens,
                "input_tokens": input_tokens,
                "output_tokens": output_tokens
            }
        });

        Ok(Json(res_json).into_response())
    } else {
        // Streaming Responses API execution
        let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(64);
        let cancel_token = tokio_util::sync::CancellationToken::new();
        let cancel_token_clone = cancel_token.clone();

        let state_clone = state.clone();
        let model = payload.model.clone();

        tokio::spawn(async move {
            let _session_guard = state_clone.acquire_session_lock(&episode.id).await;
            run_responses_api_streaming(
                state_clone,
                episode,
                model,
                user_msg,
                pruned_history,
                replica.url,
                tools,
                response_id,
                now,
                tx,
                cancel_token_clone,
            )
            .await;
        });

        let stream = CancelOnDropStream {
            inner: ReceiverStream::new(rx),
            cancel_token,
        };

        Ok(Sse::new(stream)
            .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
            .into_response())
    }
}

async fn run_responses_api_streaming(
    state: AppState,
    mut episode: Episode,
    model: String,
    user_msg: Message,
    pruned_history: Vec<Message>,
    replica_url: String,
    tools: Vec<crate::models::ToolDefinition>,
    response_id: String,
    created_at: i64,
    tx: mpsc::Sender<Result<Event, Infallible>>,
    cancel_token: tokio_util::sync::CancellationToken,
) {
    // 1. response.created
    let _ = tx
        .send(Ok(Event::default().event("response.created").data(
            json!({
                "response": {
                    "id": response_id,
                    "object": "response",
                    "created_at": created_at,
                    "model": model,
                    "status": "in_progress"
                }
            })
            .to_string(),
        )))
        .await;

    let item_id = format!("msg_{}", Uuid::new_v4().simple());

    // 2. response.output_item.added
    let _ = tx
        .send(Ok(Event::default()
            .event("response.output_item.added")
            .data(
                json!({
                    "response_id": response_id,
                    "output_index": 0,
                    "item": {
                        "id": item_id,
                        "type": "message",
                        "role": "assistant",
                        "content": []
                    }
                })
                .to_string(),
            )))
        .await;

    let mut stream = match state
        .adapter
        .stream_chat(&replica_url, &model, &pruned_history, &tools)
        .await
    {
        Ok(s) => s,
        Err(e) => {
            let _ = tx
                .send(Ok(Event::default()
                    .event("error")
                    .data(json!({ "message": e.to_string() }).to_string())))
                .await;
            return;
        }
    };

    let mut accumulated_text = String::new();
    let mut final_usage: Option<TokenUsage> = None;

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => return,
            item_opt = stream.next() => {
                let item = match item_opt {
                    Some(Ok(i)) => i,
                    _ => break,
                };

                match item {
                    StreamItem::Token(t) => {
                        accumulated_text.push_str(&t);
                        let _ = tx.send(Ok(Event::default()
                            .event("response.output_text.delta")
                            .data(json!({
                                "response_id": response_id,
                                "output_index": 0,
                                "content_index": 0,
                                "delta": t
                            }).to_string()))).await;
                    }
                    StreamItem::Usage(u) => final_usage = Some(u),
                    StreamItem::Finish { .. } => break,
                    _ => {}
                }
            }
        }
    }

    // 3. response.output_text.done
    let _ = tx
        .send(Ok(Event::default()
            .event("response.output_text.done")
            .data(
                json!({
                    "response_id": response_id,
                    "output_index": 0,
                    "content_index": 0,
                    "text": accumulated_text
                })
                .to_string(),
            )))
        .await;

    // 4. response.completed
    let input_tokens = final_usage.as_ref().map(|u| u.prompt_tokens).unwrap_or(0);
    let output_tokens = final_usage
        .as_ref()
        .map(|u| u.completion_tokens)
        .unwrap_or(0);

    let _ = tx
        .send(Ok(Event::default().event("response.completed").data(
            json!({
                "response": {
                    "id": response_id,
                    "object": "response",
                    "status": "completed",
                    "usage": {
                        "input_tokens": input_tokens,
                        "output_tokens": output_tokens,
                        "total_tokens": input_tokens + output_tokens
                    }
                }
            })
            .to_string(),
        )))
        .await;

    // Save in DAG
    let assistant_msg = Message::assistant(accumulated_text);
    let _ = episode.append_turn_with_telemetry(
        Some(user_msg),
        Some(assistant_msg),
        vec![],
        vec![],
        final_usage,
        None,
    );
    let _ = state.store.update_episode(episode).await;
}

// ==========================================
// Handler: POST /v1/chat/completions
// ==========================================

pub async fn handle_chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ChatCompletionsRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let session_header = headers
        .get("x-episod-session")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let episode = if let Some(ref sid) = session_header {
        state
            .store
            .get_episode(sid)
            .await
            .unwrap_or(None)
            .unwrap_or_else(|| Episode::new(&payload.model, None))
    } else {
        Episode::new(&payload.model, None)
    };

    let replica = state.router.route_by_session(&episode.id).ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "No healthy backend replica".into(),
        )
    })?;

    let tools = payload.tools.clone().unwrap_or_default();
    let pruned_messages = state.context_guardrail.prune_history(payload.messages);

    if !payload.stream {
        let mut stream = state
            .adapter
            .stream_chat(&replica.url, &payload.model, &pruned_messages, &tools)
            .await
            .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

        let mut content = String::new();
        let mut usage: Option<TokenUsage> = None;

        while let Some(item_res) = stream.next().await {
            match item_res {
                Ok(StreamItem::Token(t)) => content.push_str(&t),
                Ok(StreamItem::Usage(u)) => usage = Some(u),
                _ => {}
            }
        }

        let completion_id = format!("chatcmpl_{}", Uuid::new_v4().simple());
        let now = Utc::now().timestamp();

        let res = json!({
            "id": completion_id,
            "object": "chat.completion",
            "created": now,
            "model": payload.model,
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": content
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": usage
        });

        Ok(Json(res).into_response())
    } else {
        let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(64);
        let cancel_token = tokio_util::sync::CancellationToken::new();
        let cancel_token_clone = cancel_token.clone();

        let state_clone = state.clone();
        let model = payload.model.clone();

        tokio::spawn(async move {
            let completion_id = format!("chatcmpl_{}", Uuid::new_v4().simple());
            let now = Utc::now().timestamp();

            let mut stream = match state_clone
                .adapter
                .stream_chat(&replica.url, &model, &pruned_messages, &tools)
                .await
            {
                Ok(s) => s,
                Err(_) => return,
            };

            loop {
                tokio::select! {
                    _ = cancel_token_clone.cancelled() => return,
                    item_opt = stream.next() => {
                        let item = match item_opt {
                            Some(Ok(i)) => i,
                            _ => break,
                        };

                        match item {
                            StreamItem::Token(t) => {
                                let chunk = json!({
                                    "id": completion_id,
                                    "object": "chat.completion.chunk",
                                    "created": now,
                                    "model": model,
                                    "choices": [
                                        {
                                            "index": 0,
                                            "delta": { "content": t },
                                            "finish_reason": null
                                        }
                                    ]
                                });
                                let _ = tx.send(Ok(Event::default().data(chunk.to_string()))).await;
                            }
                            StreamItem::Finish { reason } => {
                                let chunk = json!({
                                    "id": completion_id,
                                    "object": "chat.completion.chunk",
                                    "created": now,
                                    "model": model,
                                    "choices": [
                                        {
                                            "index": 0,
                                            "delta": {},
                                            "finish_reason": reason
                                        }
                                    ]
                                });
                                let _ = tx.send(Ok(Event::default().data(chunk.to_string()))).await;
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }

            let _ = tx.send(Ok(Event::default().data("[DONE]"))).await;
        });

        let stream = CancelOnDropStream {
            inner: ReceiverStream::new(rx),
            cancel_token,
        };

        Ok(Sse::new(stream)
            .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
            .into_response())
    }
}
