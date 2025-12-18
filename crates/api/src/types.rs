use serde::{Deserialize, Serialize};
use zkclear_types::{Address, AssetId, BlockId, DealId};

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountBalanceResponse {
    pub address: Address,
    pub asset_id: AssetId,
    pub chain_id: zkclear_types::ChainId,
    pub amount: u128,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountStateResponse {
    pub address: Address,
    pub account_id: u64,
    pub balances: Vec<BalanceInfo>,
    pub nonce: u64,
    pub open_deals: Vec<DealId>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceInfo {
    pub asset_id: AssetId,
    pub chain_id: zkclear_types::ChainId,
    pub amount: u128,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DealDetailsResponse {
    pub deal_id: DealId,
    pub maker: Address,
    pub taker: Option<Address>,
    pub asset_base: AssetId,
    pub asset_quote: AssetId,
    pub chain_id_base: zkclear_types::ChainId,
    pub chain_id_quote: zkclear_types::ChainId,
    pub amount_base: u128,
    pub amount_remaining: u128,
    pub price_quote_per_base: u128,
    pub status: String,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub is_cross_chain: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockInfoResponse {
    pub block_id: BlockId,
    pub transaction_count: usize,
    pub timestamp: u64,
    pub transactions: Vec<TransactionInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionInfo {
    pub id: u64,
    pub from: Address,
    pub nonce: u64,
    pub kind: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueueStatusResponse {
    pub pending_transactions: usize,
    pub max_queue_size: usize,
    pub current_block_id: BlockId,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitTxRequest {
    pub tx: String,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitTxResponse {
    pub tx_hash: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<JsonRpcError>,
    pub id: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

