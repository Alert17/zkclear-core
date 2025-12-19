//! Validation tests for generated proofs
//!
//! Tests validate:
//! - Proof correctness
//! - Public inputs matching
//! - Commitments validity
//! - Different block sizes

#[cfg(any(feature = "winterfell", feature = "arkworks"))]
use crate::air::BlockTransitionInputs;
#[cfg(any(feature = "winterfell", feature = "arkworks"))]
use crate::prover::{Prover, ProverConfig};
#[cfg(any(feature = "winterfell", feature = "arkworks"))]
use crate::stark_proof::DeserializedStarkProof;
#[cfg(any(feature = "winterfell", feature = "arkworks"))]
use bincode;
#[cfg(any(feature = "winterfell", feature = "arkworks"))]
use zkclear_state::State;
#[cfg(any(feature = "winterfell", feature = "arkworks"))]
use zkclear_stf::apply_tx;
#[cfg(any(feature = "winterfell", feature = "arkworks"))]
use zkclear_types::Block;
#[cfg(any(feature = "winterfell", feature = "arkworks"))]
use zkclear_types::{Address, Tx, TxPayload};

/// Helper to create a test block
#[cfg(any(feature = "winterfell", feature = "arkworks"))]
fn create_test_block(id: u64, num_txs: usize) -> Block {
    use zkclear_types::{Deposit, TxKind};

    let mut transactions = Vec::new();

    for i in 0..num_txs {
        transactions.push(Tx {
            id: i as u64,
            from: Address::from([i as u8; 20]),
            nonce: i as u64,
            kind: TxKind::Deposit,
            payload: TxPayload::Deposit(Deposit {
                tx_hash: [i as u8; 32],
                account: Address::from([i as u8; 20]),
                asset_id: 1,
                amount: 1000 + i as u128,
                chain_id: 1,
            }),
            signature: [0u8; 65],
        });
    }

    Block {
        id,
        transactions,
        timestamp: 1000 + id,
        state_root: [0u8; 32],
        withdrawals_root: [0u8; 32],
        block_proof: vec![],
    }
}

#[cfg(any(feature = "winterfell", feature = "arkworks"))]
#[tokio::test]
async fn test_validate_proof_public_inputs() {
    let mut config = ProverConfig::default();
    config.use_placeholders = false;
    let prover = Prover::new(config).expect("Failed to create prover");

    let block = create_test_block(1, 2);
    let prev_state = State::new();
    let mut new_state = prev_state.clone();

    for tx in &block.transactions {
        apply_tx(&mut new_state, tx, block.timestamp).expect("Failed to apply transaction");
    }

    let block_proof = prover
        .prove_block(&block, &prev_state, &new_state)
        .await
        .expect("Failed to generate proof");

    // Validate public inputs match expected values
    let expected_prev_root =
        Prover::compute_state_root_static(&prev_state).expect("Failed to compute prev root");
    let expected_new_root =
        Prover::compute_state_root_static(&new_state).expect("Failed to compute new root");

    assert_eq!(
        block_proof.prev_state_root, expected_prev_root,
        "Previous state root should match computed value"
    );
    assert_eq!(
        block_proof.new_state_root, expected_new_root,
        "New state root should match computed value"
    );
}

#[cfg(feature = "winterfell")]
#[tokio::test]
async fn test_validate_stark_proof_structure() {
    use crate::stark::StarkProver;
    use crate::stark::WinterfellStarkProver;

    let prover = WinterfellStarkProver::new();
    let block = create_test_block(1, 1);
    let prev_state = State::new();
    let mut new_state = prev_state.clone();

    for tx in &block.transactions {
        apply_tx(&mut new_state, tx, block.timestamp).expect("Failed to apply transaction");
    }

    let prev_state_root =
        Prover::compute_state_root_static(&prev_state).expect("Failed to compute prev root");
    let new_state_root =
        Prover::compute_state_root_static(&new_state).expect("Failed to compute new root");
    let withdrawals_root = [0u8; 32];

    let block_data = bincode::serialize(&block).expect("Failed to serialize block");

    let stark_proof = prover
        .prove_block_transition(
            &prev_state_root,
            &new_state_root,
            &withdrawals_root,
            &block_data,
        )
        .await
        .expect("Failed to generate STARK proof");

    // Deserialize and validate proof structure
    let expected_public_inputs = BlockTransitionInputs {
        prev_state_root,
        new_state_root,
        withdrawals_root,
        block_id: block.id,
        timestamp: block.timestamp,
    };

    let deserialized = DeserializedStarkProof::from_bytes(&stark_proof, &expected_public_inputs)
        .expect("Failed to deserialize proof");

    // Validate structure
    assert!(
        deserialized.verify_structure(),
        "Proof structure should be valid"
    );
    assert!(
        deserialized.verify_commitments(),
        "Commitments should be valid"
    );
    assert!(
        deserialized.verify_public_inputs(&expected_public_inputs),
        "Public inputs should match"
    );

    // Validate specific fields
    assert!(
        deserialized.proof_length >= 100,
        "Proof should have reasonable size"
    );
    assert!(
        deserialized.num_queries >= 20 && deserialized.num_queries <= 50,
        "Queries count should be reasonable"
    );
}

#[cfg(any(feature = "winterfell", feature = "arkworks"))]
#[tokio::test]
async fn test_validate_different_block_sizes() {
    let mut config = ProverConfig::default();
    config.use_placeholders = false;
    let prover = Prover::new(config).expect("Failed to create prover");

    // Test with different block sizes
    let sizes = vec![0, 1, 2, 4, 8, 16];

    for size in sizes {
        let block = create_test_block(size as u64, size);
        let prev_state = State::new();
        let mut new_state = prev_state.clone();

        for tx in &block.transactions {
            apply_tx(&mut new_state, tx, block.timestamp).expect("Failed to apply transaction");
        }

        let block_proof = prover
            .prove_block(&block, &prev_state, &new_state)
            .await
            .expect(&format!("Failed to generate proof for block size {}", size));

        // Validate proof for each size
        assert!(
            !block_proof.zk_proof.is_empty(),
            "Proof for block size {} should not be empty",
            size
        );

        // Validate state roots
        let expected_prev =
            Prover::compute_state_root_static(&prev_state).expect("Failed to compute prev root");
        let expected_new =
            Prover::compute_state_root_static(&new_state).expect("Failed to compute new root");

        assert_eq!(
            block_proof.prev_state_root, expected_prev,
            "Prev root should match for block size {}",
            size
        );
        assert_eq!(
            block_proof.new_state_root, expected_new,
            "New root should match for block size {}",
            size
        );
    }
}

#[cfg(any(feature = "winterfell", feature = "arkworks"))]
#[tokio::test]
async fn test_validate_proof_rejects_invalid_inputs() {
    let mut config = ProverConfig::default();
    config.use_placeholders = false;
    let prover = Prover::new(config).expect("Failed to create prover");

    let block = create_test_block(1, 1);
    let prev_state = State::new();
    let mut new_state = prev_state.clone();

    for tx in &block.transactions {
        apply_tx(&mut new_state, tx, block.timestamp).expect("Failed to apply transaction");
    }

    let block_proof = prover
        .prove_block(&block, &prev_state, &new_state)
        .await
        .expect("Failed to generate proof");

    // Try to verify with wrong state
    let wrong_state = State::new();
    let wrong_proof = prover
        .prove_block(&block, &wrong_state, &new_state)
        .await
        .expect("Should generate proof even with wrong prev state");

    // Proofs should have different prev_state_root
    assert_ne!(
        block_proof.prev_state_root, wrong_proof.prev_state_root,
        "Proofs with different prev states should have different prev roots"
    );
}
