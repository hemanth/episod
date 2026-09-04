use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde_json::{json, Value};

use super::policy::ToolPolicy;
use crate::models::ToolDefinition;

#[async_trait]
pub trait ToolHandler: Send + Sync {
    async fn execute(&self, arguments: Value) -> Result<String, String>;
}

struct FnHandler<F> {
    func: F,
}

#[async_trait]
impl<F, Fut> ToolHandler for FnHandler<F>
where
    F: Fn(Value) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<String, String>> + Send,
{
    async fn execute(&self, arguments: Value) -> Result<String, String> {
        (self.func)(arguments).await
    }
}

#[derive(Clone)]
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, (ToolDefinition, Arc<dyn ToolHandler>)>>>,
    policies: Arc<RwLock<HashMap<String, ToolPolicy>>>,
    default_policy: ToolPolicy,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        let registry = Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            policies: Arc::new(RwLock::new(HashMap::new())),
            default_policy: ToolPolicy::AutoApprove,
        };

        // Register default built-in tools
        registry.register_default_tools();
        registry
    }

    pub fn set_policy(&self, tool_name: &str, policy: ToolPolicy) {
        self.policies.write().insert(tool_name.to_string(), policy);
    }

    pub fn get_policy(&self, tool_name: &str) -> ToolPolicy {
        self.policies
            .read()
            .get(tool_name)
            .copied()
            .unwrap_or(self.default_policy)
    }

    pub fn register_tool<F, Fut>(
        &self,
        definition: ToolDefinition,
        policy: ToolPolicy,
        handler: F,
    ) where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<String, String>> + Send + 'static,
    {
        let name = definition.function.name.clone();
        let boxed_handler = Arc::new(FnHandler { func: handler });
        self.tools.write().insert(name.clone(), (definition, boxed_handler));
        self.policies.write().insert(name, policy);
    }

    pub fn get_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.read().values().map(|(def, _)| def.clone()).collect()
    }

    pub async fn execute(&self, name: &str, arguments_raw: &str) -> Result<String, String> {
        let handler = {
            let tools = self.tools.read();
            tools
                .get(name)
                .map(|(_, h)| h.clone())
                .ok_or_else(|| format!("Tool '{}' not found in registry", name))?
        };

        let parsed_args: Value = if arguments_raw.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str(arguments_raw)
                .map_err(|e| format!("Invalid JSON arguments for tool '{}': {}", name, e))?
        };

        handler.execute(parsed_args).await
    }

    fn register_default_tools(&self) {
        // 1. Calculator
        let calc_def = ToolDefinition::new_function(
            "calculator",
            "Evaluate basic arithmetic expressions (add, sub, mul, div)",
            json!({
                "type": "object",
                "properties": {
                    "expression": { "type": "string", "description": "e.g. 42 + 8" }
                },
                "required": ["expression"]
            }),
        );
        self.register_tool(calc_def, ToolPolicy::AutoApprove, |args| async move {
            let expr = args
                .get("expression")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            // Simple arithmetic evaluation parser
            let sanitized: String = expr.chars().filter(|c| !c.is_whitespace()).collect();
            if let Some((a, b)) = sanitized.split_once('+') {
                let n1: f64 = a.parse().map_err(|_| "Invalid number")?;
                let n2: f64 = b.parse().map_err(|_| "Invalid number")?;
                return Ok((n1 + n2).to_string());
            } else if let Some((a, b)) = sanitized.split_once('-') {
                let n1: f64 = a.parse().map_err(|_| "Invalid number")?;
                let n2: f64 = b.parse().map_err(|_| "Invalid number")?;
                return Ok((n1 - n2).to_string());
            } else if let Some((a, b)) = sanitized.split_once('*') {
                let n1: f64 = a.parse().map_err(|_| "Invalid number")?;
                let n2: f64 = b.parse().map_err(|_| "Invalid number")?;
                return Ok((n1 * n2).to_string());
            } else if let Some((a, b)) = sanitized.split_once('/') {
                let n1: f64 = a.parse().map_err(|_| "Invalid number")?;
                let n2: f64 = b.parse().map_err(|_| "Invalid number")?;
                if n2 == 0.0 {
                    return Err("Division by zero".to_string());
                }
                return Ok((n1 / n2).to_string());
            }
            Ok(format!("Evaluated: {}", expr))
        });

        // 2. Sensitive Action (Requires Approval)
        let exec_def = ToolDefinition::new_function(
            "system_command",
            "Execute a sensitive system command (requires human-in-the-loop approval)",
            json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "The command to run" }
                },
                "required": ["command"]
            }),
        );
        self.register_tool(exec_def, ToolPolicy::RequireApproval, |args| async move {
            let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
            Ok(format!("Executed command: '{}' (success)", cmd))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_registry_and_policy() {
        let reg = ToolRegistry::new();

        assert_eq!(reg.get_policy("calculator"), ToolPolicy::AutoApprove);
        assert_eq!(reg.get_policy("system_command"), ToolPolicy::RequireApproval);

        let res = reg.execute("calculator", r#"{"expression":"10 + 25"}"#).await.unwrap();
        assert_eq!(res, "35");

        let custom_def = ToolDefinition::new_function("custom", "test", json!({}));
        reg.register_tool(custom_def, ToolPolicy::Deny, |_| async {
            Ok("custom ok".to_string())
        });
        assert_eq!(reg.get_policy("custom"), ToolPolicy::Deny);
    }
}
