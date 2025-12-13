use zkclear_sequencer::Sequencer;
use zkclear_types::{
    Address,
    AssetId,
    Deposit,
    Withdraw,
    CreateDeal,
    AcceptDeal,
    DealVisibility,
    Tx,
    TxKind,
    TxPayload,
};

fn addr(byte: u8) -> Address {
    [byte; 20]
}

fn format_address(addr: &Address) -> String {
    format!("0x{}", hex::encode(addr))
}

fn main() {
    let sequencer = Sequencer::new();

    let maker = addr(1);
    let taker = addr(2);

    let usdc: AssetId = 0;
    let btc: AssetId = 1;

    let dummy_hash = [0u8; 32];

    let txs = vec![
        Tx {
            id: 1,
            from: maker,
            nonce: 0,
            kind: TxKind::Deposit,
            payload: TxPayload::Deposit(Deposit {
                tx_hash: dummy_hash,
                account: maker,
                asset_id: usdc,
                amount: 1_000_000,  
            }),
            signature: [0u8; 65],
        },
        Tx {
            id: 2,
            from: taker,
            nonce: 0,
            kind: TxKind::Deposit,
            payload: TxPayload::Deposit(Deposit {
                tx_hash: dummy_hash,
                account: taker,
                asset_id: usdc,
                amount: 1_000_000,
            }),
            signature: [0u8; 65],
        },
        Tx {
            id: 3,
            from: maker,
            nonce: 1,
            kind: TxKind::Deposit,
            payload: TxPayload::Deposit(Deposit {
                tx_hash: dummy_hash,
                account: maker,
                asset_id: btc,
                amount: 10_000,
            }),
            signature: [0u8; 65],
        },
        Tx {
            id: 4,
            from: maker,
            nonce: 2,
            kind: TxKind::CreateDeal,
            payload: TxPayload::CreateDeal(CreateDeal {
                deal_id: 42,
                visibility: DealVisibility::Public,
                taker: None,
                asset_base: btc,
                asset_quote: usdc,
                amount_base: 1_000,
                price_quote_per_base: 100,
                expires_at: None,
                external_ref: None,
            }),
            signature: [0u8; 65],
        },
        Tx {
            id: 5,
            from: taker,
            nonce: 1,
            kind: TxKind::AcceptDeal,
            payload: TxPayload::AcceptDeal(AcceptDeal {
                deal_id: 42,
                amount: None,
            }),
            signature: [0u8; 65],
        },
        Tx {
            id: 6,
            from: maker,
            nonce: 3,
            kind: TxKind::Withdraw,
            payload: TxPayload::Withdraw(Withdraw {
                asset_id: usdc,
                amount: 50_000,
                to: maker,
            }),
            signature: [0u8; 65],
        },
    ];

    println!("Submitting {} transactions to sequencer...", txs.len());
    for tx in txs {
        sequencer.submit_tx_with_validation(tx, false).expect("Failed to submit transaction");
    }

    println!("Queue length: {} transactions", sequencer.queue_length());
    println!("\nExecuting blocks...");
    while sequencer.has_pending_txs() {
        match sequencer.build_and_execute_block() {
            Ok(block) => {
                println!(
                    "✓ Block {} executed successfully with {} transactions",
                    block.id,
                    block.transactions.len()
                );
            }
            Err(e) => {
                println!("✗ Block execution failed: {e:?}");
                break;
            }
        }
    }

    println!("\nFinal state:");
    println!("Current block ID: {}", sequencer.get_current_block_id());

    let state_handle = sequencer.get_state();
    let state = state_handle.lock().unwrap();

    for (id, acc) in &state.accounts {
        println!("Account {id}: owner={}", format_address(&acc.owner));
        for b in &acc.balances {
            println!("  asset={} amount={}", b.asset_id, b.amount);
        }
    }

    for (id, deal) in &state.deals {
        println!("Deal {id}: status={:?}", deal.status);
    }
}
