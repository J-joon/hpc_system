mod services;

use std::sync::Arc;
use hpc_logic::Hub;
use hpc_core::contracts::{StorageBackend, TransportBackend};
use hpc_core::domain::ControlRequest;
use hpc_storage_sqlite::SqliteStorage;
use hpc_transport_http::HttpTransport;
use hpc_ns_batch::{BATCH_NS_ID, BATCH_GEN_ID, get_batch_namespace_spec, get_batch_generator_spec};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Initializing HPC Node (Lean-aligned)...");

    // 1. Initialize Infrastructure (Plugins)
    let storage: Arc<dyn StorageBackend> = Arc::new(SqliteStorage::new("sqlite::memory:").await?);
    let transport: Arc<dyn TransportBackend> = Arc::new(HttpTransport::new("http://localhost:8080"));

    // 2. Initialize Logic
    let mut hub = Hub::new(storage, transport);

    // 3. Setup Registry (Registration)
    // Register Batch Namespace
    hub.process_control(ControlRequest::RegisterNamespace(
        BATCH_NS_ID.to_string(), 
        get_batch_namespace_spec()
    ));

    // Register Batch Generator
    hub.process_control(ControlRequest::RegisterGenerator(
        BATCH_GEN_ID.to_string(),
        get_batch_generator_spec()
    ));

    let hub = Arc::new(hub);

    // 4. Start Services
    println!("--- Event Loop Start ---");
    services::simulate_incoming_batch_job(hub.clone()).await;
    println!("--- Event Loop End ---");

    Ok(())
}
