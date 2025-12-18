use crate::config::ChainConfig;
use crate::event_processor::EventProcessor;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use zkclear_sequencer::Sequencer;

pub struct ChainWatcher {
    config: ChainConfig,
    processor: EventProcessor,
    processed_txs: Arc<tokio::sync::Mutex<HashSet<[u8; 32]>>>,
    last_processed_block: Arc<tokio::sync::Mutex<u64>>,
}

impl ChainWatcher {
    pub fn new(config: ChainConfig, sequencer: Arc<Sequencer>) -> anyhow::Result<Self> {
        let processor = EventProcessor::new(sequencer);
        Ok(Self {
            config,
            processor,
            processed_txs: Arc::new(tokio::sync::Mutex::new(HashSet::new())),
            last_processed_block: Arc::new(tokio::sync::Mutex::new(0)),
        })
    }

    pub async fn watch(&self) -> anyhow::Result<()> {
        println!(
            "Starting watcher for chain {} (RPC: {})",
            self.config.chain_id, self.config.rpc_url
        );

        let mut interval_timer = interval(Duration::from_secs(self.config.poll_interval_seconds));

        loop {
            interval_timer.tick().await;

            if let Err(e) = self.poll_events().await {
                eprintln!("Error polling events for chain {}: {}", self.config.chain_id, e);
            }
        }
    }

    async fn poll_events(&self) -> anyhow::Result<()> {
        let latest_block = self.get_latest_block_number().await?;
        let last_processed = *self.last_processed_block.lock().await;

        if latest_block < last_processed + self.config.required_confirmations {
            return Ok(());
        }

        let from_block = last_processed.saturating_sub(10);
        let to_block = latest_block - self.config.required_confirmations;

        if to_block <= from_block {
            return Ok(());
        }

        println!(
            "Polling blocks {}..{} on chain {}",
            from_block, to_block, self.config.chain_id
        );

        for block_num in from_block..=to_block {
            if let Err(e) = self.process_block(block_num).await {
                eprintln!("Error processing block {} on chain {}: {}", block_num, self.config.chain_id, e);
            }
        }

        *self.last_processed_block.lock().await = to_block;

        Ok(())
    }

    async fn process_block(&self, block_number: u64) -> anyhow::Result<()> {
        let logs = self.get_deposit_logs(block_number).await?;

        for log in logs {
            let tx_hash = self.parse_tx_hash(&log)?;
            
            let mut processed = self.processed_txs.lock().await;
            if processed.contains(&tx_hash) {
                continue;
            }

            let (account, asset_id, amount) = self.parse_deposit_log(&log)?;

            self.processor.process_deposit_event(
                self.config.chain_id,
                tx_hash,
                account,
                asset_id,
                amount,
            )?;

            processed.insert(tx_hash);
            println!(
                "Processed deposit: chain={}, tx_hash={:?}, account={:?}, asset={}, amount={}",
                self.config.chain_id, tx_hash, account, asset_id, amount
            );
        }

        Ok(())
    }

    async fn get_latest_block_number(&self) -> anyhow::Result<u64> {
        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_blockNumber",
            "params": [],
            "id": 1
        });

        let response: serde_json::Value = client
            .post(&self.config.rpc_url)
            .json(&payload)
            .send()
            .await?
            .json()
            .await?;

        let hex_str = response["result"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?;

        let block_num = u64::from_str_radix(hex_str.trim_start_matches("0x"), 16)
            .map_err(|e| anyhow::anyhow!("Failed to parse block number: {}", e))?;

        Ok(block_num)
    }

    async fn get_deposit_logs(&self, block_number: u64) -> anyhow::Result<Vec<serde_json::Value>> {
        let client = reqwest::Client::new();
        let block_hex = format!("0x{:x}", block_number);
        
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getLogs",
            "params": [{
                "fromBlock": block_hex.clone(),
                "toBlock": block_hex,
                "address": self.config.deposit_contract_address
            }],
            "id": 1
        });

        let response: serde_json::Value = client
            .post(&self.config.rpc_url)
            .json(&payload)
            .send()
            .await?
            .json()
            .await?;

        let logs = response["result"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        Ok(logs)
    }

    fn parse_tx_hash(&self, log: &serde_json::Value) -> anyhow::Result<[u8; 32]> {
        let tx_hash_hex = log["transactionHash"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing transactionHash in log"))?;

        let tx_hash_bytes = hex::decode(tx_hash_hex.trim_start_matches("0x"))
            .map_err(|e| anyhow::anyhow!("Failed to decode tx hash: {}", e))?;

        if tx_hash_bytes.len() != 32 {
            return Err(anyhow::anyhow!("Invalid tx hash length"));
        }

        let mut hash = [0u8; 32];
        hash.copy_from_slice(&tx_hash_bytes);
        Ok(hash)
    }

    fn parse_deposit_log(&self, log: &serde_json::Value) -> anyhow::Result<(zkclear_types::Address, zkclear_types::AssetId, u128)> {
        let topics = log["topics"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Missing topics in log"))?;

        if topics.len() < 3 {
            return Err(anyhow::anyhow!("Invalid topics length, expected at least 3"));
        }

        let account_hex = topics[1]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing account in topics"))?;
        
        let account_bytes = hex::decode(account_hex.trim_start_matches("0x"))
            .map_err(|e| anyhow::anyhow!("Failed to decode account: {}", e))?;

        if account_bytes.len() != 32 {
            return Err(anyhow::anyhow!("Invalid account length in topic, expected 32 bytes"));
        }

        let mut account = [0u8; 20];
        account.copy_from_slice(&account_bytes[12..32]);

        let asset_id_hex = topics[2]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing asset_id in topics"))?;
        
        let asset_id_bytes = hex::decode(asset_id_hex.trim_start_matches("0x"))
            .map_err(|e| anyhow::anyhow!("Failed to decode asset_id: {}", e))?;

        if asset_id_bytes.len() != 32 {
            return Err(anyhow::anyhow!("Invalid asset_id length in topic"));
        }

        let asset_id = u16::from_be_bytes([
            asset_id_bytes[30],
            asset_id_bytes[31],
        ]);

        let data = log["data"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing data in log"))?;

        let data_bytes = hex::decode(data.trim_start_matches("0x"))
            .map_err(|e| anyhow::anyhow!("Failed to decode data: {}", e))?;

        if data_bytes.len() < 32 {
            return Err(anyhow::anyhow!("Invalid data length, expected at least 32 bytes"));
        }

        let amount_bytes = &data_bytes[0..32];
        let mut amount_array = [0u8; 16];
        amount_array.copy_from_slice(&amount_bytes[16..32]);
        let amount = u128::from_be_bytes(amount_array);

        Ok((account, asset_id, amount))
    }
}

