mod services;

use std::sync::Arc;
use hpc_logic::Hub;
use hpc_core::contracts::{StorageBackend, TransportBackend};
use hpc_core::domain::ControlRequest;
use hpc_storage_sqlite::SqliteStorage;
use hpc_transport_http::{HttpTransport, run_hub_server};
use hpc_ns_batch::{BATCH_NS_ID, BATCH_GEN_ID, get_batch_namespace_spec, get_batch_generator_spec};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Initializing HPC Hub (Persistent)...");

    // 1. Initialize Infrastructure (SQLite file)
    let db_path = "hpc_hub.db";
    let storage: Arc<dyn StorageBackend> = Arc::new(SqliteStorage::new(db_path).await?);
    let transport: Arc<dyn TransportBackend> = Arc::new(HttpTransport::new("http://localhost:8080"));

    // 2. Initialize Logic
    let mut hub = Hub::new(storage, transport);

    // 3. Setup Registry (Initial bootstrap)
    hub.process_control(ControlRequest::RegisterNamespace(
        BATCH_NS_ID.to_string(), 
        get_batch_namespace_spec()
    ));

    hub.process_control(ControlRequest::RegisterGenerator(
        BATCH_GEN_ID.to_string(),
        get_batch_generator_spec()
    ));

    // 4. Run as Persistent Server
    println!("Starting Hub API server on port 8080...");
    run_hub_server(hub, 8080).await?;

    Ok(())
}
