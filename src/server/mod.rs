pub mod handlers;
pub mod openai_compat;
pub mod routes;

use std::sync::Arc;
use axum::Router;

use crate::adapter::InferenceAdapter;
use crate::guardrails::{ContextBudgetManager, InputGuardrail, ToolGuardrail};
use crate::router::ConsistentHashRouter;
use crate::store::StateStore;
use crate::tools::ToolRegistry;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<dyn StateStore>,
    pub router: ConsistentHashRouter,
    pub adapter: Arc<dyn InferenceAdapter>,
    pub tool_registry: ToolRegistry,
    pub context_guardrail: ContextBudgetManager,
    pub tool_guardrail: ToolGuardrail,
    pub input_guardrail: InputGuardrail,
}

impl AppState {
    pub fn new(
        store: Arc<dyn StateStore>,
        router: ConsistentHashRouter,
        adapter: Arc<dyn InferenceAdapter>,
        tool_registry: ToolRegistry,
    ) -> Self {
        Self {
            store,
            router,
            adapter,
            tool_registry,
            context_guardrail: ContextBudgetManager::default(),
            tool_guardrail: ToolGuardrail::default(),
            input_guardrail: InputGuardrail::default(),
        }
    }
}

pub fn create_router(state: AppState) -> Router {
    routes::create_routes(state)
}
