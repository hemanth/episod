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
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
