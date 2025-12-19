use crate::error::ProverError;

/// STARK proof generator trait
///
/// This trait allows for different STARK implementations (Winterfell, etc.)
#[async_trait::async_trait]
pub trait StarkProver: Send + Sync {
    /// Generate a STARK proof for a block state transition
    async fn prove_block_transition(
        &self,
        prev_state_root: &[u8; 32],
        new_state_root: &[u8; 32],
        withdrawals_root: &[u8; 32],
        block_data: &[u8],
    ) -> Result<Vec<u8>, ProverError>;

    /// Verify a STARK proof
    async fn verify_stark_proof(
        &self,
        proof: &[u8],
        public_inputs: &[u8],
    ) -> Result<bool, ProverError>;
}

/// Placeholder STARK prover implementation
///
/// This is a placeholder implementation used when:
/// - `use_placeholders=true` in ProverConfig (for testing)
/// - `winterfell` feature is not enabled
///
/// In production, use `WinterfellStarkProver` by enabling the `winterfell` feature
/// and setting `use_placeholders=false`.
pub struct PlaceholderStarkProver;

#[async_trait::async_trait]
impl StarkProver for PlaceholderStarkProver {
    async fn prove_block_transition(
        &self,
        _prev_state_root: &[u8; 32],
        _new_state_root: &[u8; 32],
        _withdrawals_root: &[u8; 32],
        _block_data: &[u8],
    ) -> Result<Vec<u8>, ProverError> {
        // Placeholder implementation: returns a dummy proof
        // This is intentional for testing/development when real proof generation is not needed
        Ok(b"STARK_PROOF_PLACEHOLDER".to_vec())
    }

    async fn verify_stark_proof(
        &self,
        _proof: &[u8],
        _public_inputs: &[u8],
    ) -> Result<bool, ProverError> {
        // Placeholder implementation: always returns true
        // This is intentional for testing/development when real proof verification is not needed
        Ok(true)
    }
}

/// Winterfell-based STARK prover
///
/// This uses Winterfell (from Polygon Zero) for generating STARK proofs
/// Winterfell is a reliable, well-maintained STARK library available on crates.io
#[cfg(feature = "winterfell")]
pub struct WinterfellStarkProver {
    prover: std::sync::Mutex<crate::air::BlockTransitionProver>,
    verifier: crate::air::BlockTransitionVerifier,
}

#[cfg(feature = "winterfell")]
impl WinterfellStarkProver {
    pub fn new() -> Self {
        use winterfell::ProofOptions;

        // Create proof options with reasonable defaults
        // These can be customized based on security/performance requirements
        // Note: fri_max_remainder_size must be one less than a power of two (e.g., 3, 7, 15, 31)
        let options = ProofOptions::new(
            28, // num_queries
            4,  // blowup_factor
            0,  // grinding_factor
            winterfell::FieldExtension::None,
            8,                                  // fri_folding_factor
            3, // fri_max_remainder_size (must be 2^n - 1, e.g., 3 = 2^2 - 1)
            winterfell::BatchingMethod::Linear, // constraint_batching
            winterfell::BatchingMethod::Linear, // query_batching
        );

        Self {
            prover: std::sync::Mutex::new(crate::air::BlockTransitionProver::new(options.clone())),
            verifier: crate::air::BlockTransitionVerifier::new(options),
        }
    }
}

#[cfg(feature = "winterfell")]
#[async_trait::async_trait]
impl StarkProver for WinterfellStarkProver {
    async fn prove_block_transition(
        &self,
        prev_state_root: &[u8; 32],
        new_state_root: &[u8; 32],
        withdrawals_root: &[u8; 32],
        block_data: &[u8],
    ) -> Result<Vec<u8>, ProverError> {
        use crate::air::{BlockTransitionInputs, BlockTransitionPrivateInputs};
        use zkclear_types::Block;

        // Deserialize block to extract metadata
        let block: Block = bincode::deserialize(block_data).map_err(|e| {
            ProverError::Serialization(format!("Failed to deserialize block: {}", e))
        })?;

        // Create public inputs
        let public_inputs = BlockTransitionInputs {
            prev_state_root: *prev_state_root,
            new_state_root: *new_state_root,
            withdrawals_root: *withdrawals_root,
            block_id: block.id,
            timestamp: block.timestamp,
        };

        // Create private inputs
        let private_inputs = BlockTransitionPrivateInputs {
            transactions: block_data.to_vec(),
        };

        // Generate proof using Winterfell
        // Use Mutex for interior mutability since prove requires &mut self
        let mut prover = self.prover.lock().map_err(|e| {
            ProverError::StarkProof(format!("Failed to acquire prover lock: {}", e))
        })?;
        let (proof, trace_info) = prover.prove(public_inputs, private_inputs)?;

        // Serialize proof and trace_info together
        // This allows proper verification later
        use bincode;

        #[derive(serde::Serialize, serde::Deserialize)]
        struct StarkProofWithTraceInfo {
            proof_bytes: Vec<u8>,
            trace_width: usize,
            trace_length: usize,
            version: u8,
        }

        let proof_bytes = proof.to_bytes();
        let wrapper = StarkProofWithTraceInfo {
            proof_bytes,
            trace_width: trace_info.main_trace_width(),
            trace_length: trace_info.length(),
            version: 1, // Version 1 for proof with trace_info
        };

        let serialized = bincode::serialize(&wrapper).map_err(|e| {
            ProverError::Serialization(format!("Failed to serialize proof with trace_info: {}", e))
        })?;

        Ok(serialized)
    }

    async fn verify_stark_proof(
        &self,
        proof: &[u8],
        public_inputs: &[u8],
    ) -> Result<bool, ProverError> {
        use crate::air::BlockTransitionInputs;
        use winterfell::Proof;

        // Deserialize proof and public inputs
        let proof = Proof::from_bytes(proof).map_err(|e| {
            ProverError::Serialization(format!("Failed to deserialize Winterfell proof: {}", e))
        })?;

        let public_inputs: BlockTransitionInputs =
            bincode::deserialize(public_inputs).map_err(|e| {
                ProverError::Serialization(format!("Failed to deserialize public inputs: {}", e))
            })?;

        // Verify proof using Winterfell
        self.verifier.verify(&proof, &public_inputs)?;

        Ok(true)
    }
}
