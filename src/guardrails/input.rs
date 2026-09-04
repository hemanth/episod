#[derive(Debug, Clone)]
pub struct InputGuardrail {
    pub max_message_chars: usize,
    pub max_turns_per_episode: usize,
}

impl Default for InputGuardrail {
    fn default() -> Self {
        Self {
            max_message_chars: 32_000,
            max_turns_per_episode: 500,
        }
    }
}

impl InputGuardrail {
    pub fn new(max_message_chars: usize, max_turns_per_episode: usize) -> Self {
        Self {
            max_message_chars,
            max_turns_per_episode,
        }
    }

    pub fn validate_message(&self, message: Option<&str>) -> Result<(), String> {
        if let Some(msg) = message {
            if msg.len() > self.max_message_chars {
                return Err(format!(
                    "Message length ({} chars) exceeds maximum allowed limit of {} chars",
                    msg.len(),
                    self.max_message_chars
                ));
            }
        }
        Ok(())
    }

    pub fn validate_episode_turn_count(&self, current_turns: usize) -> Result<(), String> {
        if current_turns >= self.max_turns_per_episode {
            return Err(format!(
                "Episode has reached the maximum allowed limit of {} turns",
                self.max_turns_per_episode
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_guardrail_validation() {
        let guard = InputGuardrail::new(100, 10);

        assert!(guard.validate_message(Some("Short message")).is_ok());
        assert!(guard.validate_message(None).is_ok());

        let giant_msg = "A".repeat(101);
        assert!(guard.validate_message(Some(&giant_msg)).is_err());

        assert!(guard.validate_episode_turn_count(9).is_ok());
        assert!(guard.validate_episode_turn_count(10).is_err());
    }
}
