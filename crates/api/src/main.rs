use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::time::{interval, Duration};
use zkclear_api::{create_router, ApiState};
use zkclear_sequencer::Sequencer;
use zkclear_sequencer::SequencerError;
use zkclear_prover::{Prover, ProverConfig};
#[cfg(not(feature = "rocksdb"))]
use zkclear_storage::InMemoryStorage;
#[cfg(feature = "rocksdb")]
use zkclear_storage::RocksDBStorage;
use zkclear_watcher::{Watcher, WatcherConfig};

fn get_block_interval_seconds() -> u64 {
    std::env::var("BLOCK_INTERVAL_SEC")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(zkclear_sequencer::config::DEFAULT_BLOCK_INTERVAL_SECONDS)
}

fn get_storage_path() -> PathBuf {
    std::env::var("STORAGE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./data"))
}

fn init_storage() -> Result<Arc<dyn zkclear_storage::Storage>, Box<dyn std::error::Error>> {
    #[cfg(feature = "rocksdb")]
    {
        let path = get_storage_path();
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("Failed to create storage directory: {}", e))?;

        println!("Initializing RocksDB storage at: {}", path.display());
        let storage = RocksDBStorage::open(&path)
            .map_err(|e| format!("Failed to open RocksDB storage: {:?}", e))?;

        Ok(Arc::new(storage))
    }

    #[cfg(not(feature = "rocksdb"))]
    {
        println!("Using InMemoryStorage (RocksDB not enabled)");
        Ok(Arc::new(InMemoryStorage::new()))
    }
}

async fn block_production_task(sequencer: Arc<Sequencer>) {
    let interval_secs = get_block_interval_seconds();
    let mut interval_timer = interval(Duration::from_secs(interval_secs));

    println!(
        "Block production task started (interval: {}s)",
        interval_secs
    );

    loop {
        interval_timer.tick().await;

        if !sequencer.has_pending_txs() {
            continue;
        }

        // Build and execute block with proof generation enabled
        match sequencer.build_and_execute_block_with_proof(true) {
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
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    // Initialize storage
    let storage = init_storage()?;
    let storage_trait: Arc<dyn zkclear_storage::Storage> = storage.clone();

    // Initialize prover (optional - will use placeholders if not configured)
    let prover_config = ProverConfig {
        use_placeholders: std::env::var("USE_PLACEHOLDER_PROVER")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false),
        groth16_keys_dir: std::env::var("GROTH16_KEYS_DIR")
            .ok()
            .map(std::path::PathBuf::from),
        force_regenerate_keys: std::env::var("FORCE_REGENERATE_KEYS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false),
        ..Default::default()
    };

    let prover = match Prover::new(prover_config) {
        Ok(p) => {
            println!("Prover initialized successfully");
            Some(Arc::new(p))
        }
        Err(e) => {
            eprintln!("Warning: Failed to initialize prover: {:?}. Continuing without proof generation.", e);
            None
        }
    };

    // Initialize sequencer with storage (will load state from storage if available)
    println!("Initializing sequencer with storage...");
    let mut sequencer = Sequencer::with_storage_arc(storage.clone())
        .map_err(|e| format!("Failed to initialize sequencer with storage: {:?}", e))?;
    
    // Set prover if available
    if let Some(ref prover) = prover {
        sequencer = sequencer.with_prover(Arc::clone(prover));
        println!("Prover attached to sequencer");
    }
    
    let sequencer = Arc::new(sequencer);

    println!("Sequencer initialized with storage");
    println!("Current block ID: {}", sequencer.get_current_block_id());

    let api_state = Arc::new(ApiState {
        sequencer: sequencer.clone(),
        storage: Some(storage_trait),
    });

    let app = create_router(api_state);

    let watcher_config = WatcherConfig::default();
    let watcher = Watcher::new(sequencer.clone(), watcher_config);

    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("ZKClear API server listening on http://0.0.0.0:8080");

    let server_handle = tokio::spawn(async move { axum::serve(listener, app).await });

    let block_production_handle = tokio::spawn(block_production_task(sequencer.clone()));
    let watcher_handle = tokio::spawn(async move {
        if let Err(e) = watcher.start().await {
            eprintln!("Watcher error: {}", e);
        }
    });

    tokio::select! {
        result = server_handle => {
            result??;
        }
        _ = block_production_handle => {
            eprintln!("Block production task stopped unexpectedly");
        }
        _ = watcher_handle => {
            eprintln!("Watcher task stopped unexpectedly");
        }
    }

    Ok(())
}
