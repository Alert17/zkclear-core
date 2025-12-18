use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::time::{interval, Duration};
use zkclear_api::{create_router, ApiState};
use zkclear_sequencer::Sequencer;
use zkclear_sequencer::SequencerError;
use zkclear_storage::InMemoryStorage;

fn get_block_interval_seconds() -> u64 {
    std::env::var("BLOCK_INTERVAL_SEC")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5)
}

async fn block_production_task(sequencer: Arc<Sequencer>) {
    let interval_secs = get_block_interval_seconds();
    let mut interval_timer = interval(Duration::from_secs(interval_secs));
    
    println!("Block production task started (interval: {}s)", interval_secs);
    
    loop {
        interval_timer.tick().await;
        
        if !sequencer.has_pending_txs() {
            continue;
        }
        
        match sequencer.build_and_execute_block() {
            Ok(block) => {
                println!(
                    "Block {} created and executed: {} transactions, queue: {}",
                    block.id,
                    block.transactions.len(),
                    sequencer.queue_length()
                );
            }
            Err(SequencerError::NoTransactions) => {
                // Queue was empty between check and build - skip
            }
            Err(e) => {
                eprintln!("Failed to create/execute block: {:?}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = Arc::new(InMemoryStorage::new());
    let storage_trait: Arc<dyn zkclear_storage::Storage> = storage.clone();
    
    let sequencer = Arc::new(
        Sequencer::with_storage(InMemoryStorage::new())
            .map_err(|e| format!("Failed to initialize sequencer with storage: {:?}", e))?
    );
    
    println!("Sequencer initialized with storage");
    println!("Current block ID: {}", sequencer.get_current_block_id());
    
    let api_state = Arc::new(ApiState {
        sequencer: sequencer.clone(),
        storage: Some(storage_trait),
    });

    let app = create_router(api_state);

    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("ZKClear API server listening on http://0.0.0.0:8080");
    
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app).await
    });
    
    let block_production_handle = tokio::spawn(block_production_task(sequencer));
    
    tokio::select! {
        result = server_handle => {
            result??;
        }
        _ = block_production_handle => {
            eprintln!("Block production task stopped unexpectedly");
        }
    }
    
    Ok(())
}


