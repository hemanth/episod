use crate::models::{Message, Role};

#[derive(Debug, Clone)]
pub struct ContextBudgetManager {
    pub max_context_tokens: usize,
    pub reserved_output_tokens: usize,
}

impl Default for ContextBudgetManager {
    fn default() -> Self {
        Self {
            max_context_tokens: 8192,
            reserved_output_tokens: 2048,
        }
    }
}

impl ContextBudgetManager {
    pub fn new(max_context_tokens: usize, reserved_output_tokens: usize) -> Self {
        Self {
            max_context_tokens,
            reserved_output_tokens,
        }
    }

    /// Fast, robust token estimator (~4 characters per token heuristic)
    pub fn estimate_tokens(text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }
        // Count characters / 4 + word boundary bonus
        (text.len() / 4).max(1)
    }

    pub fn estimate_message_tokens(msg: &Message) -> usize {
        let mut tokens = 4; // Role + formatting overhead
        if let Some(ref content) = msg.content {
            tokens += Self::estimate_tokens(content);
        }
        if let Some(ref tool_calls) = msg.tool_calls {
            for tc in tool_calls {
                tokens += 8; // Tool call overhead
                tokens += Self::estimate_tokens(&tc.function.name);
                tokens += Self::estimate_tokens(&tc.function.arguments);
            }
        }
        tokens
    }

    pub fn estimate_total_tokens(messages: &[Message]) -> usize {
        messages.iter().map(Self::estimate_message_tokens).sum()
    }

    /// Prune history to fit strictly within the allowed context budget.
    /// Preserves:
    /// 1. System prompt (if any) at position 0
    /// 2. Most recent turns (at least the last user prompt and its context)
    /// Truncates oldest intermediate turns from the middle if over budget.
    pub fn prune_history(&self, messages: Vec<Message>) -> Vec<Message> {
        let budget = self
            .max_context_tokens
            .saturating_sub(self.reserved_output_tokens)
            .max(512);

        let total_tokens = Self::estimate_total_tokens(&messages);
        if total_tokens <= budget || messages.len() <= 2 {
            return messages;
        }

        let mut system_msg = None;
        let mut remaining = Vec::new();

        for (idx, msg) in messages.into_iter().enumerate() {
            if idx == 0 && msg.role == Role::System {
                system_msg = Some(msg);
            } else {
                remaining.push(msg);
            }
        }

        let system_tokens = system_msg
            .as_ref()
            .map(Self::estimate_message_tokens)
            .unwrap_or(0);

        let notice = Message::system(
            "[Notice: Earlier conversation turns have been compacted to fit the active context window budget.]",
        );
        let notice_tokens = Self::estimate_message_tokens(&notice);

        let remaining_budget = budget
            .saturating_sub(system_tokens)
            .saturating_sub(notice_tokens);

        // Keep the most recent messages from the end that fit in remaining_budget
        let mut accumulated_tokens = 0;
        let mut kept_from_end = Vec::new();

        while let Some(msg) = remaining.pop() {
            let cost = Self::estimate_message_tokens(&msg);
            if accumulated_tokens + cost <= remaining_budget {
                accumulated_tokens += cost;
                kept_from_end.push(msg);
            } else {
                // If nothing fit, at least keep this latest message
                if kept_from_end.is_empty() {
                    kept_from_end.push(msg);
                }
                break;
            }
        }

        kept_from_end.reverse();

        let mut final_messages = Vec::new();
        if let Some(sys) = system_msg {
            final_messages.push(sys);
        }

        final_messages.push(notice);
        final_messages.extend(kept_from_end);
        final_messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_budget_pruning() {
        let manager = ContextBudgetManager::new(500, 100); // 400 token budget

        let mut messages = vec![Message::system("You are a helpful assistant.")];

        // Add 10 turns of long text
        for i in 1..=10 {
            messages.push(Message::user(format!("Turn {}: {}", i, "A".repeat(200))));
            messages.push(Message::assistant(format!(
                "Ans {}: {}",
                i,
                "B".repeat(200)
            )));
        }

        let original_tokens = ContextBudgetManager::estimate_total_tokens(&messages);
        assert!(original_tokens > 500);

        let pruned = manager.prune_history(messages);
        let pruned_tokens = ContextBudgetManager::estimate_total_tokens(&pruned);

        // Must keep system prompt at index 0
        assert_eq!(pruned[0].role, Role::System);
        assert!(pruned.len() < 22);
        // Must fit within budget
        assert!(pruned_tokens <= 500);
    }
}
