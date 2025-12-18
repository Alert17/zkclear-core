use serde::{Deserialize, Serialize};
use zkclear_types::ChainId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub chain_id: ChainId,
    pub rpc_url: String,
    pub deposit_contract_address: String,
    pub required_confirmations: u64,
    pub poll_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherConfig {
    pub chains: Vec<ChainConfig>,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            chains: vec![
                ChainConfig {
                    chain_id: zkclear_types::chain_ids::ETHEREUM,
                    rpc_url: "https://eth.llamarpc.com".to_string(),
                    deposit_contract_address: "0x0000000000000000000000000000000000000000".to_string(),
                    required_confirmations: 12,
                    poll_interval_seconds: 3,
                },
                ChainConfig {
                    chain_id: zkclear_types::chain_ids::POLYGON,
                    rpc_url: "https://polygon.llamarpc.com".to_string(),
                    deposit_contract_address: "0x0000000000000000000000000000000000000000".to_string(),
                    required_confirmations: 128,
                    poll_interval_seconds: 3,
                },
            ],
        }
    }
}

