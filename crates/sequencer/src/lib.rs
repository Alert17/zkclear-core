mod config;
mod validation;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use zkclear_state::State;
use zkclear_stf::{apply_block, StfError};
use zkclear_storage::Storage;
use zkclear_types::{Block, BlockId, Tx};

use config::{DEFAULT_MAX_QUEUE_SIZE, DEFAULT_MAX_TXS_PER_BLOCK, DEFAULT_SNAPSHOT_INTERVAL};
use validation::{validate_tx, ValidationError};

#[derive(Debug)]
pub enum SequencerError {
    QueueFull,
    ExecutionFailed(StfError),
    NoTransactions,
    InvalidBlockId,
    InvalidSignature,
    InvalidNonce,
    ValidationFailed,
    StorageError(String),
}

pub struct Sequencer {
    state: Arc<Mutex<State>>,
    tx_queue: Arc<Mutex<VecDeque<Tx>>>,
    max_queue_size: usize,
    current_block_id: Arc<Mutex<BlockId>>,
    max_txs_per_block: usize,
    storage: Option<Arc<dyn Storage>>,
    snapshot_interval: BlockId,
    last_snapshot_block_id: Arc<Mutex<BlockId>>,
}

impl Sequencer {
    pub fn new() -> Self {
        Self::with_config(DEFAULT_MAX_QUEUE_SIZE, DEFAULT_MAX_TXS_PER_BLOCK)
    }

    pub fn with_config(max_queue_size: usize, max_txs_per_block: usize) -> Self {
        Self {
            state: Arc::new(Mutex::new(State::new())),
            tx_queue: Arc::new(Mutex::new(VecDeque::new())),
            max_queue_size,
            current_block_id: Arc::new(Mutex::new(0)),
            max_txs_per_block,
            storage: None,
            snapshot_interval: DEFAULT_SNAPSHOT_INTERVAL,
            last_snapshot_block_id: Arc::new(Mutex::new(0)),
        }
    }

    pub fn with_snapshot_interval(mut self, interval: BlockId) -> Self {
        self.snapshot_interval = interval;
        self
    }

    pub fn with_storage<S: Storage + 'static>(storage: S) -> Result<Self, SequencerError> {
        let mut sequencer = Self::with_config(DEFAULT_MAX_QUEUE_SIZE, DEFAULT_MAX_TXS_PER_BLOCK);
        sequencer.load_state_from_storage(Arc::new(storage))?;
        Ok(sequencer)
    }

    pub fn set_storage<S: Storage + 'static>(&mut self, storage: S) -> Result<(), SequencerError> {
        self.load_state_from_storage(Arc::new(storage))?;
        Ok(())
    }

    fn load_state_from_storage(&mut self, storage: Arc<dyn Storage>) -> Result<(), SequencerError> {
        let latest_block_id = storage.get_latest_block_id()
            .map_err(|e| SequencerError::StorageError(format!("Failed to get latest block ID: {:?}", e)))?
            .unwrap_or(0);
        
        match storage.get_latest_state_snapshot() {
            Ok(Some((snapshot_state, snapshot_block_id))) => {
                *self.state.lock().unwrap() = snapshot_state;
                *self.last_snapshot_block_id.lock().unwrap() = snapshot_block_id;
                
                if latest_block_id > snapshot_block_id {
                    self.replay_blocks_from_storage(&*storage, snapshot_block_id + 1, latest_block_id)?;
                }
                
                *self.current_block_id.lock().unwrap() = latest_block_id + 1;
            }
            Ok(None) => {
                if latest_block_id > 0 {
                    self.replay_blocks_from_storage(&*storage, 0, latest_block_id)?;
                }
                *self.current_block_id.lock().unwrap() = latest_block_id + 1;
                *self.last_snapshot_block_id.lock().unwrap() = 0;
            }
            Err(e) => return Err(SequencerError::StorageError(format!("Failed to load state: {:?}", e))),
        }
        
        self.storage = Some(storage);
        Ok(())
    }

    fn replay_blocks_from_storage(&self, storage: &dyn Storage, from_block: BlockId, to_block: BlockId) -> Result<(), SequencerError> {
        let mut state = self.state.lock().unwrap();
        
        for block_id in from_block..=to_block {
            match storage.get_block(block_id) {
                Ok(Some(block)) => {
                    apply_block(&mut state, &block.transactions, block.timestamp)
                        .map_err(SequencerError::ExecutionFailed)?;
                }
                Ok(None) => {
                    return Err(SequencerError::StorageError(format!("Block {} not found", block_id)));
                }
                Err(e) => {
                    return Err(SequencerError::StorageError(format!("Failed to load block {}: {:?}", block_id, e)));
                }
            }
        }
        
        Ok(())
    }

    pub fn submit_tx(&self, tx: Tx) -> Result<(), SequencerError> {
        self.submit_tx_with_validation(tx, true)
    }

    pub fn submit_tx_with_validation(&self, tx: Tx, validate: bool) -> Result<(), SequencerError> {
        if validate {
            let state = self.state.lock().unwrap();
            
            match validate_tx(&state, &tx) {
                Ok(()) => {}
                Err(ValidationError::InvalidSignature) => return Err(SequencerError::InvalidSignature),
                Err(ValidationError::InvalidNonce) => return Err(SequencerError::InvalidNonce),
                Err(ValidationError::SignatureRecoveryFailed) => return Err(SequencerError::InvalidSignature),
            }
            
            drop(state);
        }
        
        let mut queue = self.tx_queue.lock().unwrap();
        
        if queue.len() >= self.max_queue_size {
            return Err(SequencerError::QueueFull);
        }
        
        queue.push_back(tx);
        Ok(())
    }

    pub fn build_block(&self) -> Result<Block, SequencerError> {
        let mut queue = self.tx_queue.lock().unwrap();
        let block_id = *self.current_block_id.lock().unwrap();
        
        if queue.is_empty() {
            return Err(SequencerError::NoTransactions);
        }
        
        let mut transactions = Vec::new();
        let count = queue.len().min(self.max_txs_per_block);
        
        for _ in 0..count {
            if let Some(tx) = queue.pop_front() {
                transactions.push(tx);
            } else {
                break;
            }
        }
        
        let block = Block {
            id: block_id,
            transactions,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        Ok(block)
    }

    pub fn execute_block(&self, block: Block) -> Result<(), SequencerError> {
        let expected_id = *self.current_block_id.lock().unwrap();
        if block.id != expected_id {
            return Err(SequencerError::InvalidBlockId);
        }
        
        let mut state = self.state.lock().unwrap();
        
        match apply_block(&mut state, &block.transactions, block.timestamp) {
            Ok(()) => {
                let mut block_id = self.current_block_id.lock().unwrap();
                *block_id += 1;
                drop(block_id);
                
                if let Some(ref storage) = self.storage {
                    storage.save_block(&block)
                        .map_err(|e| SequencerError::StorageError(format!("Failed to save block: {:?}", e)))?;
                    
                    for (index, tx) in block.transactions.iter().enumerate() {
                        storage.save_transaction(tx, block.id, index)
                            .map_err(|e| SequencerError::StorageError(format!("Failed to save transaction: {:?}", e)))?;
                    }
                    
                    for deal in state.deals.values() {
                        storage.save_deal(deal)
                            .map_err(|e| SequencerError::StorageError(format!("Failed to save deal: {:?}", e)))?;
                    }
                    
                    let last_snapshot = *self.last_snapshot_block_id.lock().unwrap();
                    let blocks_since_snapshot = block.id.saturating_sub(last_snapshot);
                    
                    if blocks_since_snapshot >= self.snapshot_interval {
                        let state_clone = state.clone();
                        drop(state);
                        
                        storage.save_state_snapshot(&state_clone, block.id)
                            .map_err(|e| SequencerError::StorageError(format!("Failed to save state snapshot: {:?}", e)))?;
                        
                        *self.last_snapshot_block_id.lock().unwrap() = block.id;
                    }
                }
                
                Ok(())
            }
            Err(e) => {
                Err(SequencerError::ExecutionFailed(e))
            }
        }
    }

    pub fn build_and_execute_block(&self) -> Result<Block, SequencerError> {
        let block = self.build_block()?;
        self.execute_block(block.clone())?;
        Ok(block)
    }

    pub fn get_state(&self) -> Arc<Mutex<State>> {
        Arc::clone(&self.state)
    }

    pub fn get_current_block_id(&self) -> BlockId {
        *self.current_block_id.lock().unwrap()
    }

    pub fn queue_length(&self) -> usize {
        self.tx_queue.lock().unwrap().len()
    }

    pub fn has_pending_txs(&self) -> bool {
        !self.tx_queue.lock().unwrap().is_empty()
    }

    pub fn create_state_snapshot(&self) -> Result<(), SequencerError> {
        if let Some(ref storage) = self.storage {
            let state = self.state.lock().unwrap();
            let block_id = *self.current_block_id.lock().unwrap();
            
            let state_clone = state.clone();
            drop(state);
            
            storage.save_state_snapshot(&state_clone, block_id)
                .map_err(|e| SequencerError::StorageError(format!("Failed to save state snapshot: {:?}", e)))?;
        }
        Ok(())
    }
}

impl Default for Sequencer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zkclear_types::{Tx, TxKind, TxPayload, Deposit, Address};

    fn dummy_tx(id: u64, from: Address, nonce: u64) -> Tx {
        Tx {
            id,
            from,
            nonce,
            kind: TxKind::Deposit,
            payload: TxPayload::Deposit(Deposit {
                tx_hash: [0u8; 32],
                account: from,
                asset_id: 0,
                amount: 100,
            }),
            signature: [0u8; 65],
        }
    }

    #[test]
    fn test_submit_and_build_block() {
        let sequencer = Sequencer::with_config(100, 10);
        let addr = [1u8; 20];
        
        for i in 0..5 {
            sequencer.submit_tx_with_validation(dummy_tx(i, addr, i), false).unwrap();
        }
        
        let block = sequencer.build_block().unwrap();
        assert_eq!(block.transactions.len(), 5);
        assert_eq!(sequencer.queue_length(), 0);
    }

    #[test]
    fn test_queue_full() {
        let sequencer = Sequencer::with_config(5, 10);
        let addr = [1u8; 20];
        
        for i in 0..5 {
            sequencer.submit_tx_with_validation(dummy_tx(i, addr, i), false).unwrap();
        }
        
        match sequencer.submit_tx_with_validation(dummy_tx(5, addr, 5), false) {
            Err(SequencerError::QueueFull) => {},
            _ => panic!("Expected QueueFull error"),
        }
    }

    #[test]
    fn test_execute_block() {
        let sequencer = Sequencer::new();
        let addr = [1u8; 20];
        
        sequencer.submit_tx_with_validation(dummy_tx(0, addr, 0), false).unwrap();
        let block = sequencer.build_block().unwrap();
        
        sequencer.execute_block(block).unwrap();
        assert_eq!(sequencer.get_current_block_id(), 1);
    }

    #[test]
    fn test_build_and_execute() {
        let sequencer = Sequencer::new();
        let addr = [1u8; 20];
        
        sequencer.submit_tx_with_validation(dummy_tx(0, addr, 0), false).unwrap();
        let block = sequencer.build_and_execute_block().unwrap();
        
        assert_eq!(block.id, 0);
        assert_eq!(sequencer.get_current_block_id(), 1);
    }
}

