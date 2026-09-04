use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use super::handlers::*;
use super::AppState;

pub fn create_routes(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health_check))
        .route("/v1/episodes", post(create_episode).get(list_episodes))
        .route("/v1/episodes/{id}", get(get_episode))
        .route("/v1/episodes/{id}/turns", post(submit_turn))
        .route("/v1/episodes/{id}/fork", post(fork_episode))
        .route("/v1/episodes/{id}/approve", post(approve_tool))
        .route("/v1/responses", post(super::openai_compat::handle_responses_api))
        .route("/v1/chat/completions", post(super::openai_compat::handle_chat_completions))
        .route("/dashboard", get(super::dashboard::render_dashboard))
        .route("/", get(super::landing::render_landing_page))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
