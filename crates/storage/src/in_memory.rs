use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use zkclear_state::State;
use zkclear_types::{Deal, DealId, Tx};
use zkclear_sequencer::{Block, BlockId};
use crate::storage_trait::{Storage, StorageError, TxId};

pub struct InMemoryStorage {
    blocks: Arc<RwLock<HashMap<BlockId, Block>>>,
    transactions: Arc<RwLock<HashMap<TxId, Tx>>>,
    deals: Arc<RwLock<HashMap<DealId, Deal>>>,
    state_snapshots: Arc<RwLock<HashMap<BlockId, State>>>,
    latest_block_id: Arc<RwLock<Option<BlockId>>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            blocks: Arc::new(RwLock::new(HashMap::new())),
            transactions: Arc::new(RwLock::new(HashMap::new())),
            deals: Arc::new(RwLock::new(HashMap::new())),
            state_snapshots: Arc::new(RwLock::new(HashMap::new())),
            latest_block_id: Arc::new(RwLock::new(None)),
        }
    }
}

impl Storage for InMemoryStorage {
    fn save_block(&self, block: &Block) -> Result<(), StorageError> {
        let mut blocks = self.blocks.write().unwrap();
        blocks.insert(block.id, block.clone());
        
        let mut latest = self.latest_block_id.write().unwrap();
        *latest = Some(block.id);
        
        Ok(())
    }

    fn get_block(&self, block_id: BlockId) -> Result<Option<Block>, StorageError> {
        let blocks = self.blocks.read().unwrap();
        Ok(blocks.get(&block_id).cloned())
    }

    fn get_latest_block_id(&self) -> Result<Option<BlockId>, StorageError> {
        let latest = self.latest_block_id.read().unwrap();
        Ok(*latest)
    }

    fn save_transaction(&self, tx: &Tx, block_id: BlockId, index: usize) -> Result<(), StorageError> {
        let mut transactions = self.transactions.write().unwrap();
        transactions.insert((block_id, index), tx.clone());
        Ok(())
    }

    fn get_transaction(&self, block_id: BlockId, index: usize) -> Result<Option<Tx>, StorageError> {
        let transactions = self.transactions.read().unwrap();
        Ok(transactions.get(&(block_id, index)).cloned())
    }

    fn get_transactions_by_block(&self, block_id: BlockId) -> Result<Vec<Tx>, StorageError> {
        let transactions = self.transactions.read().unwrap();
        let mut txs: Vec<(usize, Tx)> = transactions
            .iter()
            .filter(|((bid, _), _)| *bid == block_id)
            .map(|((_, idx), tx)| (*idx, tx.clone()))
            .collect();
        txs.sort_by_key(|(idx, _)| *idx);
        Ok(txs.into_iter().map(|(_, tx)| tx).collect())
    }

    fn save_deal(&self, deal: &Deal) -> Result<(), StorageError> {
        let mut deals = self.deals.write().unwrap();
        deals.insert(deal.id, deal.clone());
        Ok(())
    }

    fn get_deal(&self, deal_id: DealId) -> Result<Option<Deal>, StorageError> {
        let deals = self.deals.read().unwrap();
        Ok(deals.get(&deal_id).cloned())
    }

    fn get_all_deals(&self) -> Result<Vec<Deal>, StorageError> {
        let deals = self.deals.read().unwrap();
        Ok(deals.values().cloned().collect())
    }

    fn save_state_snapshot(&self, state: &State, block_id: BlockId) -> Result<(), StorageError> {
        let mut snapshots = self.state_snapshots.write().unwrap();
        snapshots.insert(block_id, state.clone());
        Ok(())
    }

    fn get_latest_state_snapshot(&self) -> Result<Option<(State, BlockId)>, StorageError> {
        let latest_id = self.latest_block_id.read().unwrap();
        if let Some(block_id) = *latest_id {
            let snapshots = self.state_snapshots.read().unwrap();
            if let Some(state) = snapshots.get(&block_id) {
                return Ok(Some((state.clone(), block_id)));
            }
        }
        Ok(None)
    }

    fn flush(&self) -> Result<(), StorageError> {
        Ok(())
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

