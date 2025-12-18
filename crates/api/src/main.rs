use std::sync::Arc;
use tokio::net::TcpListener;
use zkclear_api::{create_router, ApiState};
use zkclear_sequencer::Sequencer;
use zkclear_storage::InMemoryStorage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sequencer = Sequencer::new();
    let storage = Arc::new(InMemoryStorage::new());
    
    let api_state = Arc::new(ApiState {
        sequencer: Arc::new(sequencer),
        storage: Some(storage as Arc<dyn zkclear_storage::Storage>),
    });

    let app = create_router(api_state);

    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("ZKClear API server listening on http://0.0.0.0:8080");
    
    axum::serve(listener, app).await?;
    
    Ok(())
}


