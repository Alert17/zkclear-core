//! Performance and profiling tests for proof generation

#[cfg(any(feature = "winterfell", feature = "arkworks"))]
use crate::prover::{Prover, ProverConfig};
#[cfg(any(feature = "winterfell", feature = "arkworks"))]
use std::time::Instant;
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
async fn test_proof_generation_performance() {
    let mut config = ProverConfig::default();
    config.use_placeholders = false;
    let prover = Prover::new(config).expect("Failed to create prover");

    let block = create_test_block(1, 4);
    let mut prev_state = State::new();
    let mut new_state = prev_state.clone();

    for tx in &block.transactions {
        apply_tx(&mut new_state, tx, block.timestamp).expect("Failed to apply transaction");
    }

    // Measure proof generation time
    let start = Instant::now();
    let block_proof = prover
        .prove_block(&block, &prev_state, &new_state)
        .await
        .expect("Failed to generate proof");
    let duration = start.elapsed();

    println!("Proof generation took: {:?}", duration);
    println!("Proof size: {} bytes", block_proof.zk_proof.len());

    // Performance assertions (adjust thresholds based on requirements)
    assert!(
        duration.as_secs() < 60,
        "Proof generation should complete within 60 seconds"
    );
    assert!(
        !block_proof.zk_proof.is_empty(),
        "Proof should not be empty"
    );
}

#[cfg(any(feature = "winterfell", feature = "arkworks"))]
#[tokio::test]
async fn test_proof_size_scaling() {
    let mut config = ProverConfig::default();
    config.use_placeholders = false;
    let prover = Prover::new(config).expect("Failed to create prover");

    // Test with different block sizes
    let sizes = vec![1, 2, 4, 8];
    let mut proof_sizes = Vec::new();

    for size in sizes {
        let block = create_test_block(size as u64, size);
        let mut prev_state = State::new();
        let mut new_state = prev_state.clone();

        for tx in &block.transactions {
            apply_tx(&mut new_state, tx, block.timestamp).expect("Failed to apply transaction");
        }

        let block_proof = prover
            .prove_block(&block, &prev_state, &new_state)
            .await
            .expect(&format!("Failed to generate proof for size {}", size));

        proof_sizes.push((size, block_proof.zk_proof.len()));
        println!(
            "Block size {}: proof size {} bytes",
            size,
            block_proof.zk_proof.len()
        );
    }

    // Validate that proof sizes are reasonable
    for (size, proof_size) in &proof_sizes {
        assert!(
            *proof_size > 0,
            "Proof size should be positive for block size {}",
            size
        );
        assert!(
            *proof_size < 10_000_000,
            "Proof size should be reasonable (< 10MB) for block size {}",
            size
        );
    }
}

#[cfg(any(feature = "winterfell", feature = "arkworks"))]
#[tokio::test]
async fn test_proof_generation_time_scaling() {
    let mut config = ProverConfig::default();
    config.use_placeholders = false;
    let prover = Prover::new(config).expect("Failed to create prover");

    // Test with different block sizes
    let sizes = vec![1, 2, 4];
    let mut timings = Vec::new();

    for size in sizes {
        let block = create_test_block(size as u64, size);
        let mut prev_state = State::new();
        let mut new_state = prev_state.clone();

        for tx in &block.transactions {
            apply_tx(&mut new_state, tx, block.timestamp).expect("Failed to apply transaction");
        }

        let start = Instant::now();
        let _block_proof = prover
            .prove_block(&block, &prev_state, &new_state)
            .await
            .expect(&format!("Failed to generate proof for size {}", size));
        let duration = start.elapsed();

        timings.push((size, duration));
        println!("Block size {}: proof generation took {:?}", size, duration);
    }

    // Validate that timings are reasonable
    for (size, duration) in &timings {
        assert!(
            duration.as_secs() < 120,
            "Proof generation should complete within 120 seconds for block size {}",
            size
        );
    }
}

#[cfg(any(feature = "winterfell", feature = "arkworks"))]
#[tokio::test]
async fn test_verification_performance() {
    let mut config = ProverConfig::default();
    config.use_placeholders = false;
    let prover = Prover::new(config).expect("Failed to create prover");

    let block = create_test_block(1, 2);
    let mut prev_state = State::new();
    let mut new_state = prev_state.clone();

    for tx in &block.transactions {
        apply_tx(&mut new_state, tx, block.timestamp).expect("Failed to apply transaction");
    }

    // Generate proof
    let block_proof = prover
        .prove_block(&block, &prev_state, &new_state)
        .await
        .expect("Failed to generate proof");

    // Measure verification time
    #[cfg(feature = "arkworks")]
    {
        let public_inputs = bincode::serialize(&(
            block_proof.prev_state_root,
            block_proof.new_state_root,
            block_proof.withdrawals_root,
        ))
        .unwrap();

        let start = Instant::now();
        let verify_result = prover
            .verify_snark_proof(&block_proof.zk_proof, &public_inputs)
            .await
            .expect("Verification should succeed");
        let duration = start.elapsed();

        println!("Proof verification took: {:?}", duration);
        assert!(verify_result, "Proof should be valid");
        assert!(
            duration.as_secs() < 10,
            "Verification should complete within 10 seconds"
        );
    }
}
