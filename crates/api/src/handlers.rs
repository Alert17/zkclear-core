use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use zkclear_sequencer::Sequencer;
use zkclear_storage::Storage;
use zkclear_types::{AssetId, BlockId, DealId};

use crate::types::*;

pub struct ApiState {
    pub sequencer: Arc<Sequencer>,
    pub storage: Option<Arc<dyn Storage>>,
}

pub async fn get_account_balance(
    State(state): State<Arc<ApiState>>,
    Path((address, asset_id)): Path<(String, AssetId)>,
) -> Result<Json<AccountBalanceResponse>, (StatusCode, Json<ErrorResponse>)> {
    let address_bytes = hex::decode(address.trim_start_matches("0x"))
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "InvalidAddress".to_string(),
                    message: "Invalid address format".to_string(),
                }),
            )
        })?;

    if address_bytes.len() != 20 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "InvalidAddress".to_string(),
                message: "Address must be 20 bytes".to_string(),
            }),
        ));
    }

    let mut addr = [0u8; 20];
    addr.copy_from_slice(&address_bytes);

    let state_handle = state.sequencer.get_state();
    let state_guard = state_handle.lock().unwrap();
    
    let account = state_guard
        .get_account_by_address(addr)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "AccountNotFound".to_string(),
                    message: "Account not found".to_string(),
                }),
            )
        })?;

    let balance = account
        .balances
        .iter()
        .find(|b| b.asset_id == asset_id)
        .map(|b| (b.chain_id, b.amount))
        .unwrap_or((zkclear_types::chain_ids::ETHEREUM, 0));

    Ok(Json(AccountBalanceResponse {
        address: addr,
        asset_id,
        chain_id: balance.0,
        amount: balance.1,
    }))
}

pub async fn get_account_state(
    State(state): State<Arc<ApiState>>,
    Path(address): Path<String>,
) -> Result<Json<AccountStateResponse>, (StatusCode, Json<ErrorResponse>)> {
    let address_bytes = hex::decode(address.trim_start_matches("0x"))
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "InvalidAddress".to_string(),
                    message: "Invalid address format".to_string(),
                }),
            )
        })?;

    if address_bytes.len() != 20 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "InvalidAddress".to_string(),
                message: "Address must be 20 bytes".to_string(),
            }),
        ));
    }

    let mut addr = [0u8; 20];
    addr.copy_from_slice(&address_bytes);

    let state_handle = state.sequencer.get_state();
    let state_guard = state_handle.lock().unwrap();
    
    let account = state_guard
        .get_account_by_address(addr)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "AccountNotFound".to_string(),
                    message: "Account not found".to_string(),
                }),
            )
        })?;

    let balances: Vec<BalanceInfo> = account
        .balances
        .iter()
        .map(|b| BalanceInfo {
            asset_id: b.asset_id,
            chain_id: b.chain_id,
            amount: b.amount,
        })
        .collect();

    let open_deals: Vec<DealId> = state_guard
        .deals
        .values()
        .filter(|deal| {
            (deal.maker == addr || deal.taker == Some(addr))
                && matches!(deal.status, zkclear_types::DealStatus::Pending)
        })
        .map(|deal| deal.id)
        .collect();

    Ok(Json(AccountStateResponse {
        address: addr,
        account_id: account.id,
        balances,
        nonce: account.nonce,
        open_deals,
    }))
}

pub async fn get_deal_details(
    State(state): State<Arc<ApiState>>,
    Path(deal_id): Path<DealId>,
) -> Result<Json<DealDetailsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let state_handle = state.sequencer.get_state();
    let state_guard = state_handle.lock().unwrap();
    
    let deal = state_guard
        .get_deal(deal_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "DealNotFound".to_string(),
                    message: format!("Deal {} not found", deal_id),
                }),
            )
        })?;

    Ok(Json(DealDetailsResponse {
        deal_id: deal.id,
        maker: deal.maker,
        taker: deal.taker,
        asset_base: deal.asset_base,
        asset_quote: deal.asset_quote,
        chain_id_base: deal.chain_id_base,
        chain_id_quote: deal.chain_id_quote,
        amount_base: deal.amount_base,
        amount_remaining: deal.amount_remaining,
        price_quote_per_base: deal.price_quote_per_base,
        status: format!("{:?}", deal.status),
        created_at: deal.created_at,
        expires_at: deal.expires_at,
        is_cross_chain: deal.is_cross_chain,
    }))
}

pub async fn get_block_info(
    State(state): State<Arc<ApiState>>,
    Path(block_id): Path<BlockId>,
) -> Result<Json<BlockInfoResponse>, (StatusCode, Json<ErrorResponse>)> {
    let block = if let Some(ref storage) = state.storage {
        storage
            .get_block(block_id)
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "StorageError".to_string(),
                        message: "Failed to load block from storage".to_string(),
                    }),
                )
            })?
            .ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: "BlockNotFound".to_string(),
                        message: format!("Block {} not found", block_id),
                    }),
                )
            })?
    } else {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "StorageNotAvailable".to_string(),
                message: "Storage not configured".to_string(),
            }),
        ));
    };

    let transactions: Vec<TransactionInfo> = block
        .transactions
        .iter()
        .map(|tx| TransactionInfo {
            id: tx.id,
            from: tx.from,
            nonce: tx.nonce,
            kind: format!("{:?}", tx.kind),
        })
        .collect();

    Ok(Json(BlockInfoResponse {
        block_id: block.id,
        transaction_count: block.transactions.len(),
        timestamp: block.timestamp,
        transactions,
    }))
}

pub async fn get_queue_status(
    State(state): State<Arc<ApiState>>,
) -> Json<QueueStatusResponse> {
    Json(QueueStatusResponse {
        pending_transactions: state.sequencer.queue_length(),
        max_queue_size: 10000,
        current_block_id: state.sequencer.get_current_block_id(),
    })
}

pub async fn get_supported_chains() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "chains": [
            {
                "chain_id": zkclear_types::chain_ids::ETHEREUM,
                "name": "Ethereum"
            },
            {
                "chain_id": zkclear_types::chain_ids::POLYGON,
                "name": "Polygon"
            },
            {
                "chain_id": zkclear_types::chain_ids::MANTLE,
                "name": "Mantle"
            },
            {
                "chain_id": zkclear_types::chain_ids::ARBITRUM,
                "name": "Arbitrum"
            },
            {
                "chain_id": zkclear_types::chain_ids::OPTIMISM,
                "name": "Optimism"
            },
            {
                "chain_id": zkclear_types::chain_ids::BASE,
                "name": "Base"
            }
        ]
    }))
}

pub async fn jsonrpc_handler(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    if request.jsonrpc != "2.0" {
        return Json(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32600,
                message: "Invalid Request".to_string(),
                data: None,
            }),
            id: request.id,
        });
    }

    let result = match request.method.as_str() {
        "submit_tx" => {
            let tx_hex = match request.params.get("tx") {
                Some(serde_json::Value::String(hex_str)) => hex_str,
                _ => {
                    return Json(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32602,
                            message: "Invalid params: 'tx' must be a hex string".to_string(),
                            data: None,
                        }),
                        id: request.id,
                    });
                }
            };

            let tx_bytes = match hex::decode(tx_hex.trim_start_matches("0x")) {
                Ok(bytes) => bytes,
                Err(_) => {
                    return Json(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32602,
                            message: "Invalid params: 'tx' must be valid hex".to_string(),
                            data: None,
                        }),
                        id: request.id,
                    });
                }
            };

            let tx: zkclear_types::Tx = match bincode::deserialize(&tx_bytes) {
                Ok(tx) => tx,
                Err(_) => {
                    return Json(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32602,
                            message: "Invalid params: failed to deserialize transaction".to_string(),
                            data: None,
                        }),
                        id: request.id,
                    });
                }
            };

            match state.sequencer.submit_tx(tx) {
                Ok(()) => {
                    let tx_hash = hex::encode(&tx_bytes);
                    Some(serde_json::json!({
                        "tx_hash": tx_hash,
                        "status": "queued"
                    }))
                }
                Err(zkclear_sequencer::SequencerError::QueueFull) => {
                    return Json(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32000,
                            message: "Queue full".to_string(),
                            data: None,
                        }),
                        id: request.id,
                    });
                }
                Err(zkclear_sequencer::SequencerError::InvalidSignature) => {
                    return Json(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32001,
                            message: "Invalid signature".to_string(),
                            data: None,
                        }),
                        id: request.id,
                    });
                }
                Err(zkclear_sequencer::SequencerError::InvalidNonce) => {
                    return Json(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32002,
                            message: "Invalid nonce".to_string(),
                            data: None,
                        }),
                        id: request.id,
                    });
                }
                Err(e) => {
                    return Json(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32003,
                            message: format!("Submission failed: {:?}", e),
                            data: None,
                        }),
                        id: request.id,
                    });
                }
            }
        }
        "get_account_balance" => {
            return Json(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: "Use REST endpoint /api/v1/account/:address/balance/:asset_id instead".to_string(),
                    data: None,
                }),
                id: request.id,
            });
        }
        _ => {
            return Json(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: "Method not found".to_string(),
                    data: None,
                }),
                id: request.id,
            });
        }
    };

    Json(JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result,
        error: None,
        id: request.id,
    })
}

