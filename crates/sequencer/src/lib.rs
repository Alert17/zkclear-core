mod config;
mod validation;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use zkclear_state::State;
use zkclear_stf::{apply_block, StfError};
use zkclear_types::Tx;

use config::{DEFAULT_MAX_QUEUE_SIZE, DEFAULT_MAX_TXS_PER_BLOCK};
use validation::{validate_tx, ValidationError};

pub type BlockId = u64;

#[derive(Debug, Clone)]
pub struct Block {
    pub id: BlockId,
    pub transactions: Vec<Tx>,
    pub timestamp: u64,
}

#[derive(Debug)]
pub enum SequencerError {
    QueueFull,
    ExecutionFailed(StfError),
    NoTransactions,
    InvalidBlockId,
    InvalidSignature,
    InvalidNonce,
    ValidationFailed,
}

#[derive(Debug)]
pub struct Sequencer {
    state: Arc<Mutex<State>>,
    tx_queue: Arc<Mutex<VecDeque<Tx>>>,
    max_queue_size: usize,
    current_block_id: Arc<Mutex<BlockId>>,
    max_txs_per_block: usize,
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
        }
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

