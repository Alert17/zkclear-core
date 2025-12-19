//! End-to-end integration tests for ZK proof flow
//!
//! Tests the complete flow: block creation → proof generation → verification

#[cfg(any(feature = "winterfell", feature = "arkworks"))]
use crate::prover::{Prover, ProverConfig};
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
async fn test_e2e_block_creation_to_proof_generation() {
    // Create prover
    let mut config = ProverConfig::default();
    config.use_placeholders = false; // Use real provers
    let prover = Prover::new(config).expect("Failed to create prover");

    // Create block with transactions
    let block = create_test_block(1, 3);
    let prev_state = State::new();
    let mut new_state = prev_state.clone();

    // Apply transactions
    for tx in &block.transactions {
        apply_tx(&mut new_state, tx, block.timestamp).expect("Failed to apply transaction");
    }

    // Generate proof
    let result = prover.prove_block(&block, &prev_state, &new_state).await;
    assert!(result.is_ok(), "Proof generation should succeed");

    let block_proof = result.unwrap();

    // Validate proof structure
    assert_ne!(
        block_proof.prev_state_root, block_proof.new_state_root,
        "State roots should be different after transactions"
    );
    assert!(
        !block_proof.zk_proof.is_empty(),
        "ZK proof should not be empty"
    );

    // Verify proof
    #[cfg(feature = "arkworks")]
    {
        let verify_result = prover
            .verify_snark_proof(
                &block_proof.zk_proof,
                &bincode::serialize(&(
                    block_proof.prev_state_root,
                    block_proof.new_state_root,
                    block_proof.withdrawals_root,
                ))
                .unwrap(),
            )
            .await;

        assert!(
            verify_result.is_ok(),
            "SNARK proof verification should succeed"
        );
        assert!(verify_result.unwrap(), "SNARK proof should be valid");
    }
}

#[cfg(any(feature = "winterfell", feature = "arkworks"))]
#[tokio::test]
async fn test_e2e_multiple_blocks_sequential() {
    let mut config = ProverConfig::default();
    config.use_placeholders = false;
    let prover = Prover::new(config).expect("Failed to create prover");

    let mut state = State::new();

    // Process multiple blocks sequentially
    for block_id in 1..=5 {
        let block = create_test_block(block_id, 2);
        let prev_state = state.clone();

        // Apply transactions
        for tx in &block.transactions {
            apply_tx(&mut state, tx, block.timestamp).expect("Failed to apply transaction");
        }

        // Generate proof
        let block_proof = prover
            .prove_block(&block, &prev_state, &state)
            .await
            .expect("Failed to generate proof");

        // Validate each block proof
        assert!(
            !block_proof.zk_proof.is_empty(),
            "Block {} proof should not be empty",
            block_id
        );

        // State root should change after transactions
        if !block.transactions.is_empty() {
            assert_ne!(
                block_proof.prev_state_root, block_proof.new_state_root,
                "Block {} state root should change",
                block_id
            );
        }
    }
}

#[cfg(any(feature = "winterfell", feature = "arkworks"))]
#[tokio::test]
async fn test_e2e_proof_consistency() {
    let mut config = ProverConfig::default();
    config.use_placeholders = false;
    let prover = Prover::new(config).expect("Failed to create prover");

    let block = create_test_block(1, 2);
    let prev_state = State::new();
    let mut new_state = prev_state.clone();

    // Apply transactions
    for tx in &block.transactions {
        apply_tx(&mut new_state, tx, block.timestamp).expect("Failed to apply transaction");
    }

    // Generate proof twice - should be consistent
    let proof1 = prover
        .prove_block(&block, &prev_state, &new_state)
        .await
        .expect("Failed to generate proof 1");

    let proof2 = prover
        .prove_block(&block, &prev_state, &new_state)
        .await
        .expect("Failed to generate proof 2");

    // Public inputs should match
    assert_eq!(proof1.prev_state_root, proof2.prev_state_root);
    assert_eq!(proof1.new_state_root, proof2.new_state_root);
    assert_eq!(proof1.withdrawals_root, proof2.withdrawals_root);

    // Proofs might differ (non-deterministic), but should both verify
    #[cfg(feature = "arkworks")]
    {
        let public_inputs = bincode::serialize(&(
            proof1.prev_state_root,
            proof1.new_state_root,
            proof1.withdrawals_root,
        ))
        .unwrap();

        let verify1 = prover
            .verify_snark_proof(&proof1.zk_proof, &public_inputs)
            .await
            .expect("Verification 1 should succeed");
        assert!(verify1, "Proof 1 should be valid");

        let verify2 = prover
            .verify_snark_proof(&proof2.zk_proof, &public_inputs)
            .await
            .expect("Verification 2 should succeed");
        assert!(verify2, "Proof 2 should be valid");
    }
}
