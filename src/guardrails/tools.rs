use std::time::Duration;
use tokio::time::timeout;

#[derive(Debug, Clone)]
pub struct ToolGuardrail {
    pub execution_timeout: Duration,
    pub max_output_chars: usize,
}

impl Default for ToolGuardrail {
    fn default() -> Self {
        Self {
            execution_timeout: Duration::from_secs(15),
            max_output_chars: 8_000,
        }
    }
}

impl ToolGuardrail {
    pub fn new(timeout_secs: u64, max_output_chars: usize) -> Self {
        Self {
            execution_timeout: Duration::from_secs(timeout_secs),
            max_output_chars,
        }
    }

    /// Execute a tool future with strict timeout protection
    pub async fn execute_with_timeout<Fut>(&self, fut: Fut) -> Result<String, String>
    where
        Fut: std::future::Future<Output = Result<String, String>>,
    {
        match timeout(self.execution_timeout, fut).await {
            Ok(Ok(output)) => Ok(self.sanitize_output(output)),
            Ok(Err(tool_err)) => Err(tool_err),
            Err(_) => Err(format!(
                "Tool execution timed out after {} seconds",
                self.execution_timeout.as_secs()
            )),
        }
    }

    /// Truncate overly long tool outputs to prevent blowing up the LLM context window
    pub fn sanitize_output(&self, output: String) -> String {
        if output.len() <= self.max_output_chars {
            output
        } else {
            let keep_len = self.max_output_chars.saturating_sub(100);
            let truncated = &output[..keep_len];
            format!(
                "{}\n\n[...output truncated by guardrail: exceeded {} characters limit...]",
                truncated, self.max_output_chars
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_guardrail_timeout() {
        let guard = ToolGuardrail::new(1, 1000);
        let slow_tool = async {
            tokio::time::sleep(Duration::from_millis(1500)).await;
            Ok("done".to_string())
        };

        let result = guard.execute_with_timeout(slow_tool).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("timed out"));
    }

    #[tokio::test]
    async fn test_tool_output_truncation() {
        let guard = ToolGuardrail::new(5, 200);
        let giant_output = "X".repeat(500);

        let sanitized = guard.sanitize_output(giant_output);
        assert!(sanitized.len() < 300);
        assert!(sanitized.contains("output truncated by guardrail"));
    }
}
