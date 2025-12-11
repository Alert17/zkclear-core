use zkclear_state::State;
use zkclear_stf::apply_block;
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

fn main() {
    let mut state = State::new();

    let maker = addr(1);
    let taker = addr(2);

    let usdc: AssetId = 0;
    let btc: AssetId = 1;

    let dummy_hash = [0u8; 32];

    let txs = vec![
        // maker gets USDC
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
            signature: vec![],
        },
        // taker gets USDC
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
            signature: vec![],
        },
        // maker gets BTC
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
            signature: vec![],
        },
        // maker creates public deal: sell 1000 BTC units for 100 USDC per unit
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
            signature: vec![],
        },
        // taker accepts this deal
        Tx {
            id: 5,
            from: taker,
            nonce: 1,
            kind: TxKind::AcceptDeal,
            payload: TxPayload::AcceptDeal(AcceptDeal {
                deal_id: 42,
            }),
            signature: vec![],
        },
        // maker withdraws часть USDC
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
            signature: vec![],
        },
    ];

    match apply_block(&mut state, &txs) {
        Ok(()) => {
            println!("Block applied");

            for (id, acc) in &state.accounts {
                println!("Account {id}: owner={:?}", acc.owner);
                for b in &acc.balances {
                    println!("  asset={} amount={}", b.asset_id, b.amount);
                }
            }

            for (id, deal) in &state.deals {
                println!("Deal {id}: status={:?}", deal.status);
            }
        }
        Err(e) => {
            println!("Block failed: {e:?}");
        }
    }
}
