use std::collections::HashMap;
use std::convert::Infallible;
use std::time::Duration;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Json};
use futures_util::{Stream, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{error, warn};
use uuid::Uuid;

use super::AppState;
use crate::adapter::StreamItem;
use crate::models::{AgentEvent, Episode, Message, PendingApproval, ToolCall, ToolExecutionResult};
use crate::tools::ToolPolicy;

#[derive(Debug, Deserialize)]
pub struct CreateEpisodeRequest {
    pub model: String,
    pub system_prompt: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTurnRequest {
    pub message: Option<String>,
    pub parent_node_id: Option<String>,
    #[serde(default = "default_true")]
    pub execute_tools: bool,
    #[serde(default = "default_max_iterations")]
    pub max_tool_iterations: usize,
}

fn default_true() -> bool {
    true
}

fn default_max_iterations() -> usize {
    5
}

#[derive(Debug, Deserialize)]
pub struct ForkRequest {
    pub from_node_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ApprovalRequest {
    pub tool_call_id: String,
    pub approved: bool,
    pub feedback: Option<String>,
}

pub async fn health_check() -> impl IntoResponse {
    Json(json!({ "status": "ok", "service": "episod" }))
}

pub async fn create_episode(
    State(state): State<AppState>,
    Json(payload): Json<CreateEpisodeRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut episode = Episode::new(payload.model, payload.system_prompt);
    if let Some(meta) = payload.metadata {
        episode.metadata = meta;
    }

    let created = state
        .store
        .create_episode(episode)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn list_episodes(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let list = state
        .store
        .list_episodes(100, 0)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(list))
}

pub async fn get_episode(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let ep = state
        .store
        .get_episode(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Episode '{}' not found", id)))?;

    Ok(Json(ep))
}

pub async fn fork_episode(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<ForkRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut ep = state
        .store
        .get_episode(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Episode '{}' not found", id)))?;

    ep.fork(&payload.from_node_id)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let updated = state
        .store
        .update_episode(ep)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(updated))
}

pub async fn approve_tool(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<ApprovalRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut ep = state
        .store
        .get_episode(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Episode '{}' not found", id)))?;

    let pending = ep
        .pending_approvals
        .remove(&payload.tool_call_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!(
                    "No pending approval found for tool_call_id '{}'",
                    payload.tool_call_id
                ),
            )
        })?;

    let result = if payload.approved {
        let output = match state
            .tool_guardrail
            .execute_with_timeout(state.tool_registry.execute(
                &pending.tool_call.function.name,
                &pending.tool_call.function.arguments,
            ))
            .await
        {
            Ok(out) => out,
            Err(e) => format!("Execution error: {}", e),
        };
        ToolExecutionResult {
            tool_call_id: payload.tool_call_id.clone(),
            tool_name: pending.tool_call.function.name.clone(),
            output,
            is_error: false,
        }
    } else {
        ToolExecutionResult {
            tool_call_id: payload.tool_call_id.clone(),
            tool_name: pending.tool_call.function.name.clone(),
            output: payload
                .feedback
                .unwrap_or_else(|| "Tool execution rejected by human operator.".to_string()),
            is_error: true,
        }
    };

    // Append this approved tool execution result to the DAG
    ep.append_turn(
        None,
        None,
        vec![pending.tool_call],
        vec![result.clone()],
    );

    let updated = state
        .store
        .update_episode(ep)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({
        "status": if payload.approved { "approved" } else { "rejected" },
        "result": result,
        "episode": updated
    })))
}

use tokio_util::sync::CancellationToken;

struct CancelOnDropStream<S> {
    inner: S,
    cancel_token: CancellationToken,
}

impl<S: Stream + Unpin> Stream for CancelOnDropStream<S> {
    type Item = S::Item;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        std::pin::Pin::new(&mut self.inner).poll_next(cx)
    }
}

impl<S> Drop for CancelOnDropStream<S> {
    fn drop(&mut self) {
        self.cancel_token.cancel();
    }
}

pub async fn submit_turn(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<CreateTurnRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, (StatusCode, String)> {
    let ep = state
        .store
        .get_episode(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Episode '{}' not found", id)))?;

    // 1. Validate input message size
    state
        .input_guardrail
        .validate_message(payload.message.as_deref())
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    // 2. Validate max episode turns
    state
        .input_guardrail
        .validate_episode_turn_count(ep.nodes.len())
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(64);
    let cancel_token = CancellationToken::new();
    let cancel_token_clone = cancel_token.clone();

    let state_clone = state.clone();

    tokio::spawn(async move {
        run_turn_orchestration(state_clone, ep, payload, tx, cancel_token_clone).await;
    });

    let stream = CancelOnDropStream {
        inner: ReceiverStream::new(rx),
        cancel_token,
    };
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}

async fn emit_event(
    tx: &mpsc::Sender<Result<Event, Infallible>>,
    event: AgentEvent,
) -> bool {
    let event_name = event.event_type();
    let data = serde_json::to_string(&event).unwrap_or_default();
    let sse_event = Event::default().event(event_name).data(data);
    tx.send(Ok(sse_event)).await.is_ok()
}

async fn run_turn_orchestration(
    state: AppState,
    mut ep: Episode,
    req: CreateTurnRequest,
    tx: mpsc::Sender<Result<Event, Infallible>>,
    cancel_token: CancellationToken,
) {
    let turn_id = format!("turn_{}", Uuid::new_v4().simple());
    let parent_node_id = req.parent_node_id.clone().or_else(|| ep.active_leaf_id.clone());

    emit_event(
        &tx,
        AgentEvent::TurnStarted {
            episode_id: ep.id.clone(),
            turn_id: turn_id.clone(),
            parent_node_id: parent_node_id.clone(),
        },
    )
    .await;

    // 1. Build initial message history from DAG
    let raw_history = ep.linearize_history(parent_node_id.as_deref());
    let mut history = state.context_guardrail.prune_history(raw_history);

    let user_msg = if let Some(ref text) = req.message {
        let msg = Message::user(text.clone());
        history.push(msg.clone());
        Some(msg)
    } else {
        None
    };

    // 2. Select backend replica via consistent hash router
    let replica = match state.router.route_by_session(&ep.id) {
        Some(r) => r,
        None => {
            emit_event(
                &tx,
                AgentEvent::Error {
                    turn_id: Some(turn_id),
                    message: "No healthy inference backend replicas available in cluster".to_string(),
                },
            )
            .await;
            return;
        }
    };

    let tools = if req.execute_tools {
        state.tool_registry.get_definitions()
    } else {
        Vec::new()
    };
    let mut iteration = 0;
    let mut accumulated_tool_calls: Vec<ToolCall> = Vec::new();
    let mut accumulated_tool_results: Vec<ToolExecutionResult> = Vec::new();
    let mut final_assistant_content = String::new();

    loop {
        // Cancellation check
        if cancel_token.is_cancelled() || tx.is_closed() {
            warn!("Episode {} turn {} cancelled due to client disconnect", ep.id, turn_id);
            return;
        }

        iteration += 1;
        if iteration > req.max_tool_iterations {
            warn!("Episode {} reached max tool iterations ({})", ep.id, req.max_tool_iterations);
            break;
        }

        // Call inference adapter stream
        let mut stream = match state
            .adapter
            .stream_chat(&replica.url, &ep.model, &history, &tools)
            .await
        {
            Ok(s) => s,
            Err(e) => {
                emit_event(
                    &tx,
                    AgentEvent::Error {
                        turn_id: Some(turn_id),
                        message: format!("Inference adapter stream error: {}", e),
                    },
                )
                .await;
                return;
            }
        };

        #[derive(Default)]
        struct ToolCallAccumulator {
            id: Option<String>,
            name: Option<String>,
            arguments: String,
        }
        let mut tool_accumulators: std::collections::BTreeMap<usize, ToolCallAccumulator> =
            std::collections::BTreeMap::new();

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    warn!("Inference stream cancelled by client disconnect");
                    return;
                }
                item_opt = stream.next() => {
                    let item_res = match item_opt {
                        Some(res) => res,
                        None => break,
                    };

                    match item_res {
                        Err(e) => {
                            emit_event(
                                &tx,
                                AgentEvent::Error {
                                    turn_id: Some(turn_id.clone()),
                                    message: format!("Stream decode error: {}", e),
                                },
                            )
                            .await;
                            return;
                        }
                        Ok(StreamItem::Token(token)) => {
                            final_assistant_content.push_str(&token);
                            emit_event(
                                &tx,
                                AgentEvent::TokenDelta {
                                    turn_id: turn_id.clone(),
                                    delta: token,
                                },
                            )
                            .await;
                        }
                        Ok(StreamItem::ToolCallDelta {
                            index,
                            id,
                            name,
                            arguments_delta,
                        }) => {
                            let entry = tool_accumulators.entry(index).or_default();
                            if let Some(tc_id) = id {
                                entry.id = Some(tc_id);
                            }
                            if let Some(tc_name) = name {
                                entry.name = Some(tc_name);
                            }
                            entry.arguments.push_str(&arguments_delta);

                            emit_event(
                                &tx,
                                AgentEvent::ToolCallDelta {
                                    turn_id: turn_id.clone(),
                                    tool_call_id: entry.id.clone().unwrap_or_default(),
                                    index,
                                    name: entry.name.clone(),
                                    arguments_delta,
                                },
                            )
                            .await;
                        }
                        Ok(StreamItem::Finish { reason: _ }) => {}
                    }
                }
            }
        }

        // Flush all parsed parallel tool calls
        let mut current_tool_calls: Vec<ToolCall> = Vec::new();
        for (_idx, acc) in tool_accumulators {
            let tc_id = acc.id.unwrap_or_else(|| format!("call_{}", Uuid::new_v4().simple()));
            let tc_name = acc.name.unwrap_or_else(|| "unknown".to_string());
            let parsed_args: Value = serde_json::from_str(&acc.arguments)
                .unwrap_or_else(|_| json!({ "raw": acc.arguments }));

            emit_event(
                &tx,
                AgentEvent::ToolCallCompleted {
                    turn_id: turn_id.clone(),
                    tool_call_id: tc_id.clone(),
                    name: tc_name.clone(),
                    arguments: parsed_args,
                },
            )
            .await;

            current_tool_calls.push(ToolCall {
                id: tc_id,
                call_type: "function".to_string(),
                function: crate::models::FunctionCall {
                    name: tc_name,
                    arguments: acc.arguments,
                },
            });
        }

        if current_tool_calls.is_empty() {
            // Finished without tool calls, this turn is complete
            break;
        }

        // Handle tool calls
        let mut pause_for_approval = false;
        accumulated_tool_calls.extend(current_tool_calls.clone());

        for tc in current_tool_calls {
            let policy = state.tool_registry.get_policy(&tc.function.name);

            match policy {
                ToolPolicy::RequireApproval => {
                    // Emit ApprovalRequired event & persist in pending approvals
                    let args_val: Value = serde_json::from_str(&tc.function.arguments)
                        .unwrap_or_else(|_| json!({ "raw": tc.function.arguments }));

                    emit_event(
                        &tx,
                        AgentEvent::ApprovalRequired {
                            episode_id: ep.id.clone(),
                            turn_id: turn_id.clone(),
                            tool_call_id: tc.id.clone(),
                            tool_name: tc.function.name.clone(),
                            arguments: args_val,
                            reason: "Sensitive tool execution requires human approval".to_string(),
                        },
                    )
                    .await;

                    ep.pending_approvals.insert(
                        tc.id.clone(),
                        PendingApproval {
                            turn_id: turn_id.clone(),
                            tool_call: tc.clone(),
                            created_at: chrono::Utc::now().timestamp_millis(),
                        },
                    );

                    pause_for_approval = true;
                }
                ToolPolicy::AutoApprove if req.execute_tools => {
                    emit_event(
                        &tx,
                        AgentEvent::ToolExecutionStarted {
                            turn_id: turn_id.clone(),
                            tool_call_id: tc.id.clone(),
                            tool_name: tc.function.name.clone(),
                        },
                    )
                    .await;

                    let tool_fut = state
                        .tool_guardrail
                        .execute_with_timeout(state.tool_registry.execute(&tc.function.name, &tc.function.arguments));

                    let (output, is_error) = tokio::select! {
                        _ = cancel_token.cancelled() => {
                            warn!("Tool execution aborted due to client disconnect");
                            return;
                        }
                        res = tool_fut => {
                            match res {
                                Ok(out) => (out, false),
                                Err(err) => (format!("Tool execution error: {}", err), true),
                            }
                        }
                    };

                    emit_event(
                        &tx,
                        AgentEvent::ToolExecutionCompleted {
                            turn_id: turn_id.clone(),
                            tool_call_id: tc.id.clone(),
                            tool_name: tc.function.name.clone(),
                            output: output.clone(),
                            is_error,
                        },
                    )
                    .await;

                    let result = ToolExecutionResult {
                        tool_call_id: tc.id.clone(),
                        tool_name: tc.function.name.clone(),
                        output: output.clone(),
                        is_error,
                    };

                    accumulated_tool_results.push(result);

                    // Add assistant tool call + tool response into history for next LLM iteration
                    history.push(Message::assistant_with_tools(vec![tc.clone()]));
                    history.push(Message::tool_response(&tc.id, &output));
                }
                _ => {
                    // Denied or client-delegated execution
                    let result = ToolExecutionResult {
                        tool_call_id: tc.id.clone(),
                        tool_name: tc.function.name.clone(),
                        output: "Tool execution omitted or denied by policy".to_string(),
                        is_error: true,
                    };
                    accumulated_tool_results.push(result);
                    pause_for_approval = true;
                }
            }
        }

        if pause_for_approval {
            break;
        }
    }

    // Append turn node into DAG
    let assistant_msg = if !final_assistant_content.is_empty() {
        Some(Message::assistant(final_assistant_content))
    } else {
        None
    };

    let node_id = ep.append_turn(
        user_msg,
        assistant_msg,
        accumulated_tool_calls,
        accumulated_tool_results,
    );

    if let Err(e) = state.store.update_episode(ep.clone()).await {
        error!("Failed to update episode {} in store: {}", ep.id, e);
    }

    emit_event(
        &tx,
        AgentEvent::TurnCompleted {
            episode_id: ep.id,
            turn_id,
            node_id,
            finish_reason: "stop".to_string(),
        },
    )
    .await;
}
