use zkclear_state::State;
use zkclear_types::{Block, BlockProof, Withdraw, WithdrawalProof, Address};
use crate::error::ProverError;
use crate::merkle::{MerkleTree, hash_withdrawal, verify_merkle_proof};
use crate::nullifier::generate_nullifier_from_withdrawal;
use crate::stark::StarkProver;
use crate::snark::SnarkProver;

/// Configuration for the ZK prover
#[derive(Debug, Clone)]
pub struct ProverConfig {
    /// Whether to use placeholder implementations (for testing)
    pub use_placeholders: bool,
    /// Path to SP1 configuration (if using SP1)
    pub sp1_config_path: Option<String>,
    /// Path to Plonky2 configuration (if using Plonky2)
    pub plonky2_config_path: Option<String>,
}

impl Default for ProverConfig {
    fn default() -> Self {
        Self {
            use_placeholders: true,
            sp1_config_path: None,
            plonky2_config_path: None,
        }
    }
}

/// Main ZK prover service
/// 
/// This service coordinates STARK and SNARK proof generation
pub struct Prover {
    stark_prover: Box<dyn StarkProver>,
    snark_prover: Box<dyn SnarkProver>,
}

impl Prover {
    /// Create a new prover with the given configuration
    pub fn new(config: ProverConfig) -> Self {
        let stark_prover: Box<dyn StarkProver> = if config.use_placeholders {
            Box::new(crate::stark::PlaceholderStarkProver)
        } else {
            #[cfg(feature = "sp1")]
            {
                Box::new(crate::stark::Sp1StarkProver::new())
            }
            #[cfg(not(feature = "sp1"))]
            {
                Box::new(crate::stark::PlaceholderStarkProver)
            }
        };

        let snark_prover: Box<dyn SnarkProver> = if config.use_placeholders {
            Box::new(crate::snark::PlaceholderSnarkProver)
        } else {
            // TODO: Initialize Plonky2 or other SNARK prover based on config
            Box::new(crate::snark::PlaceholderSnarkProver)
        };

        Self {
            stark_prover,
            snark_prover,
        }
    }

    /// Generate a block proof (STARK + SNARK)
    /// 
    /// This generates a STARK proof for the block state transition,
    /// then wraps it in a SNARK for compact on-chain verification
    pub async fn prove_block(
        &self,
        block: &Block,
        prev_state: &State,
        new_state: &State,
    ) -> Result<BlockProof, ProverError> {
        // Calculate state roots
        let prev_state_root = self.compute_state_root(prev_state)?;
        let new_state_root = self.compute_state_root(new_state)?;
        let withdrawals_root = self.compute_withdrawals_root(block)?;

        // Serialize block data for proof generation
        let block_data = bincode::serialize(block)
            .map_err(|e| ProverError::Serialization(format!("Failed to serialize block: {}", e)))?;

        // Generate STARK proof
        let stark_proof = self.stark_prover.prove_block_transition(
            &prev_state_root,
            &new_state_root,
            &withdrawals_root,
            &block_data,
        ).await?;

        // Wrap STARK proof in SNARK
        let public_inputs = bincode::serialize(&(prev_state_root, new_state_root, withdrawals_root))
            .map_err(|e| ProverError::Serialization(format!("Failed to serialize public inputs: {}", e)))?;
        
        let snark_proof = self.snark_prover.wrap_stark_in_snark(&stark_proof, &public_inputs).await?;

        Ok(BlockProof {
            prev_state_root,
            new_state_root,
            withdrawals_root,
            zk_proof: snark_proof,
        })
    }

    /// Generate a withdrawal proof
    /// 
    /// This generates a Merkle proof for inclusion in withdrawals_root
    /// and a ZK proof for withdrawal validity
    pub async fn prove_withdrawal(
        &self,
        withdrawal: &Withdraw,
        user: Address,
        withdrawals_root: &[u8; 32],
        merkle_proof: Vec<[u8; 32]>,
        secret: &[u8; 32],
    ) -> Result<WithdrawalProof, ProverError> {
        // Generate nullifier
        let nullifier = generate_nullifier_from_withdrawal(
            user,
            withdrawal.asset_id,
            withdrawal.amount,
            withdrawal.chain_id,
            secret,
        );

        // Verify Merkle proof
        let leaf = hash_withdrawal(
            user,
            withdrawal.asset_id,
            withdrawal.amount,
            withdrawal.chain_id,
        );
        
        if !verify_merkle_proof(&leaf, &merkle_proof, withdrawals_root) {
            return Err(ProverError::InvalidWithdrawalsRoot("Merkle proof verification failed".to_string()));
        }

        // Generate ZK proof for withdrawal validity
        // For now, use placeholder (will be replaced with actual proof generation)
        let zk_proof = b"WITHDRAWAL_PROOF_PLACEHOLDER".to_vec();

        Ok(WithdrawalProof {
            merkle_proof: merkle_proof.iter().flat_map(|p| p.iter().copied()).collect(),
            nullifier,
            zk_proof,
        })
    }

    /// Compute state root from state
    fn compute_state_root(&self, state: &State) -> Result<[u8; 32], ProverError> {
        // TODO: Implement proper state root computation
        // For now, use a placeholder hash
        let state_bytes = bincode::serialize(state)
            .map_err(|e| ProverError::Serialization(format!("Failed to serialize state: {}", e)))?;
        
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&state_bytes);
        Ok(hasher.finalize().into())
    }

    /// Compute withdrawals root from block
    fn compute_withdrawals_root(&self, block: &Block) -> Result<[u8; 32], ProverError> {
        let mut tree = MerkleTree::new();
        
        // Extract withdrawals from block transactions
        for tx in &block.transactions {
            if let zkclear_types::TxPayload::Withdraw(w) = &tx.payload {
                let leaf = hash_withdrawal(
                    tx.from,
                    w.asset_id,
                    w.amount,
                    w.chain_id,
                );
                tree.add_leaf(leaf);
            }
        }

        tree.root()
    }

    /// Generate Merkle proof for a withdrawal
    pub fn generate_withdrawal_merkle_proof(
        &self,
        block: &Block,
        withdrawal_index: usize,
    ) -> Result<(Vec<[u8; 32]>, [u8; 32]), ProverError> {
        let mut tree = MerkleTree::new();
        let mut target_index = None;

        // Build tree and find withdrawal index
        let mut current_index = 0;
        for tx in &block.transactions {
            if let zkclear_types::TxPayload::Withdraw(w) = &tx.payload {
                let leaf = hash_withdrawal(
                    tx.from,
                    w.asset_id,
                    w.amount,
                    w.chain_id,
                );
                tree.add_leaf(leaf);
                
                if current_index == withdrawal_index {
                    target_index = Some(tree.leaves.len() - 1);
                }
                current_index += 1;
            }
        }

        let root = tree.root()?;
        let proof = if let Some(idx) = target_index {
            tree.proof(idx)?
        } else {
            return Err(ProverError::InvalidWithdrawalsRoot(
                format!("Withdrawal index {} not found", withdrawal_index)
            ));
        };

        Ok((proof, root))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zkclear_types::{Tx, TxPayload, Withdraw};

    #[tokio::test]
    async fn test_prove_block_placeholder() {
        let config = ProverConfig::default();
        let prover = Prover::new(config);

        let block = Block {
            id: 0,
            transactions: vec![],
            timestamp: 1000,
            state_root: [0u8; 32],
            withdrawals_root: [0u8; 32],
            block_proof: vec![],
        };

        let prev_state = State::new();
        let new_state = State::new();

        let proof = prover.prove_block(&block, &prev_state, &new_state).await;
        assert!(proof.is_ok());
    }
}

