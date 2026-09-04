use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum AgentEvent {
    EpisodeCreated {
        episode_id: String,
        created_at: i64,
    },
    TurnStarted {
        episode_id: String,
        turn_id: String,
        parent_node_id: Option<String>,
    },
    TokenDelta {
        turn_id: String,
        delta: String,
    },
    ToolCallDelta {
        turn_id: String,
        tool_call_id: String,
        index: usize,
        name: Option<String>,
        arguments_delta: String,
    },
    ToolCallCompleted {
        turn_id: String,
        tool_call_id: String,
        name: String,
        arguments: Value,
    },
    ApprovalRequired {
        episode_id: String,
        turn_id: String,
        tool_call_id: String,
        tool_name: String,
        arguments: Value,
        reason: String,
    },
    ToolExecutionStarted {
        turn_id: String,
        tool_call_id: String,
        tool_name: String,
    },
    ToolExecutionCompleted {
        turn_id: String,
        tool_call_id: String,
        tool_name: String,
        output: String,
        is_error: bool,
    },
    TurnCompleted {
        episode_id: String,
        turn_id: String,
        node_id: String,
        finish_reason: String,
    },
    Error {
        turn_id: Option<String>,
        message: String,
    },
}

impl AgentEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            AgentEvent::EpisodeCreated { .. } => "episode_created",
            AgentEvent::TurnStarted { .. } => "turn_started",
            AgentEvent::TokenDelta { .. } => "token_delta",
            AgentEvent::ToolCallDelta { .. } => "tool_call_delta",
            AgentEvent::ToolCallCompleted { .. } => "tool_call_completed",
            AgentEvent::ApprovalRequired { .. } => "approval_required",
            AgentEvent::ToolExecutionStarted { .. } => "tool_execution_started",
            AgentEvent::ToolExecutionCompleted { .. } => "tool_execution_completed",
            AgentEvent::TurnCompleted { .. } => "turn_completed",
            AgentEvent::Error { .. } => "error",
        }
    }
}
