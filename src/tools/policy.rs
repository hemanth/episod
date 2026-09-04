use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolPolicy {
    AutoApprove,
    RequireApproval,
    Deny,
}

impl Default for ToolPolicy {
    fn default() -> Self {
        ToolPolicy::AutoApprove
    }
}
