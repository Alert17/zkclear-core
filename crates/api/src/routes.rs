use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

use crate::handlers::{ApiState, *};

pub fn create_router(state: Arc<ApiState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route(
            "/api/v1/account/:address/balance/:asset_id",
            get(get_account_balance),
        )
        .route("/api/v1/account/:address", get(get_account_state))
        .route("/api/v1/deal/:deal_id", get(get_deal_details))
        .route("/api/v1/block/:block_id", get(get_block_info))
        .route("/api/v1/queue/status", get(get_queue_status))
        .route("/api/v1/chains", get(get_supported_chains))
        .route("/jsonrpc", post(jsonrpc_handler))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK"
}
