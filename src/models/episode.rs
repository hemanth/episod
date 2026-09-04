use std::collections::HashMap;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::events::{TokenUsage, TurnTiming};
use super::message::{Message, ToolCall};
use super::tool::ToolExecutionResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub children_ids: Vec<String>,
    pub user_message: Option<Message>,
    pub assistant_message: Option<Message>,
    pub tool_calls: Vec<ToolCall>,
    pub tool_results: Vec<ToolExecutionResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing: Option<TurnTiming>,
    pub created_at: i64,
}

impl TurnNode {
    pub fn new(
        parent_id: Option<String>,
        user_message: Option<Message>,
        assistant_message: Option<Message>,
        tool_calls: Vec<ToolCall>,
        tool_results: Vec<ToolExecutionResult>,
    ) -> Self {
        Self {
            id: format!("turn_{}", Uuid::new_v4().simple()),
            parent_id,
            children_ids: Vec::new(),
            user_message,
            assistant_message,
            tool_calls,
            tool_results,
            usage: None,
            timing: None,
            created_at: Utc::now().timestamp_millis(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingApproval {
    pub turn_id: String,
    pub tool_call: ToolCall,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub root_node_id: Option<String>,
    pub active_leaf_id: Option<String>,
    pub nodes: HashMap<String, TurnNode>,
    pub pending_approvals: HashMap<String, PendingApproval>,
    pub metadata: HashMap<String, String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Episode {
    pub fn new(model: impl Into<String>, system_prompt: Option<String>) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            id: format!("ep_{}", Uuid::new_v4().simple()),
            model: model.into(),
            system_prompt,
            root_node_id: None,
            active_leaf_id: None,
            nodes: HashMap::new(),
            pending_approvals: HashMap::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Append a new turn to the DAG under the active leaf (or root if first turn)
    pub fn append_turn(
        &mut self,
        user_msg: Option<Message>,
        assistant_msg: Option<Message>,
        tool_calls: Vec<ToolCall>,
        tool_results: Vec<ToolExecutionResult>,
    ) -> String {
        let parent_id = self.active_leaf_id.clone();
        let new_node = TurnNode::new(
            parent_id.clone(),
            user_msg,
            assistant_msg,
            tool_calls,
            tool_results,
        );
        let new_id = new_node.id.clone();

        if let Some(ref pid) = parent_id {
            if let Some(parent) = self.nodes.get_mut(pid) {
                parent.children_ids.push(new_id.clone());
            }
        } else {
            self.root_node_id = Some(new_id.clone());
        }

        self.nodes.insert(new_id.clone(), new_node);
        self.active_leaf_id = Some(new_id.clone());
        self.updated_at = Utc::now().timestamp_millis();
        new_id
    }

    /// Append a new turn to the DAG with telemetry (token usage and timing metrics)
    pub fn append_turn_with_telemetry(
        &mut self,
        user_msg: Option<Message>,
        assistant_msg: Option<Message>,
        tool_calls: Vec<ToolCall>,
        tool_results: Vec<ToolExecutionResult>,
        usage: Option<TokenUsage>,
        timing: Option<TurnTiming>,
    ) -> String {
        let parent_id = self.active_leaf_id.clone();
        let mut new_node = TurnNode::new(
            parent_id.clone(),
            user_msg,
            assistant_msg,
            tool_calls,
            tool_results,
        );
        new_node.usage = usage;
        new_node.timing = timing;
        let new_id = new_node.id.clone();

        if let Some(ref pid) = parent_id {
            if let Some(parent) = self.nodes.get_mut(pid) {
                parent.children_ids.push(new_id.clone());
            }
        } else {
            self.root_node_id = Some(new_id.clone());
        }

        self.nodes.insert(new_id.clone(), new_node);
        self.active_leaf_id = Some(new_id.clone());
        self.updated_at = Utc::now().timestamp_millis();
        new_id
    }

    /// Fork conversation branch from a specific historical node
    pub fn fork(&mut self, from_node_id: &str) -> Result<String, String> {
        if !self.nodes.contains_key(from_node_id) {
            return Err(format!("Node {} not found in episode", from_node_id));
        }
        self.active_leaf_id = Some(from_node_id.to_string());
        self.updated_at = Utc::now().timestamp_millis();
        Ok(from_node_id.to_string())
    }

    /// Linearize the conversation history from the active leaf (or specified node) back to root
    pub fn linearize_history(&self, target_node_id: Option<&str>) -> Vec<Message> {
        let mut messages = Vec::new();

        // 1. Prepend system prompt if exists
        if let Some(ref sys) = self.system_prompt {
            messages.push(Message::system(sys.clone()));
        }

        let start_id = target_node_id
            .or(self.active_leaf_id.as_deref());

        let mut current_id = start_id;
        let mut turn_chain = Vec::new();

        while let Some(node_id) = current_id {
            if let Some(node) = self.nodes.get(node_id) {
                turn_chain.push(node);
                current_id = node.parent_id.as_deref();
            } else {
                break;
            }
        }

        // Reverse to get chronological order from root to leaf
        turn_chain.reverse();

        for node in turn_chain {
            if let Some(ref u_msg) = node.user_message {
                messages.push(u_msg.clone());
            }

            if let Some(ref a_msg) = node.assistant_message {
                messages.push(a_msg.clone());
            }

            // If there were tool calls and responses in this node
            for result in &node.tool_results {
                messages.push(Message::tool_response(&result.tool_call_id, &result.output));
            }
        }

        messages
    }
}
