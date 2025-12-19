use axum::{
    extract::State,
    routing::{get, post},
    response::Json,
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

use crate::handlers::ApiState;
use crate::handlers::*;

pub fn create_router(state: Arc<ApiState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
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

/// Health check endpoint with component status
async fn health_check(State(state): State<Arc<ApiState>>) -> Json<serde_json::Value> {
    use serde_json::json;
    
    // Check sequencer status
    let sequencer_healthy = state.sequencer.get_current_block_id() >= 0;
    let queue_length = state.sequencer.queue_length();
    
    // Check storage status
    let storage_healthy = state.storage.is_some();
    let storage_available = if let Some(ref storage) = state.storage {
        // Try to read a block to verify storage is working
        storage.get_block(0).is_ok()
    } else {
        false
    };
    
    // Overall health status
    let healthy = sequencer_healthy && storage_healthy && storage_available;
    
    let status = if healthy { "healthy" } else { "degraded" };
    
    Json(json!({
        "status": status,
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        "components": {
            "sequencer": {
                "status": if sequencer_healthy { "healthy" } else { "unhealthy" },
                "current_block_id": state.sequencer.get_current_block_id(),
                "queue_length": queue_length
            },
            "storage": {
                "status": if storage_available { "healthy" } else { "unhealthy" },
                "configured": storage_healthy
            }
        }
    }))
}

/// Readiness check endpoint (for Kubernetes/Docker health checks)
async fn readiness_check(State(state): State<Arc<ApiState>>) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    use serde_json::json;
    
    // Check if all critical components are ready
    let sequencer_ready = state.sequencer.get_current_block_id() >= 0;
    let storage_ready = state.storage.is_some();
    
    if sequencer_ready && storage_ready {
        Ok(Json(json!({
            "status": "ready",
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        })))
    } else {
        Err(axum::http::StatusCode::SERVICE_UNAVAILABLE)
    }
}
