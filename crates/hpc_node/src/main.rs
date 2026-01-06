mod services;

use std::sync::Arc;
use clap::{Parser, Subcommand};
use hpc_logic::Hub;
use hpc_core::contracts::{StorageBackend, TransportBackend};
use hpc_core::domain::{ControlRequest, Poke, Cursor};
use hpc_storage_sqlite::SqliteStorage;
use hpc_transport_http::{HttpTransport, run_hub_server, register_with_hub, pull_from_hub};
use hpc_ns_batch::{BATCH_NS_ID, get_batch_namespace_spec, get_batch_generator_spec};
use hpc_executor_relay::RelayExecutor;
use axum::{Router, routing::post, Json, extract::State, http::StatusCode};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Runs the node as a Hub
    Hub {
        #[arg(short, long, default_value_t = 8080)]
        port: u16,
    },
    /// Runs the node as a Relay
    Relay {
        #[arg(short, long, default_value = "http://localhost:8080")]
        hub_url: String,
        #[arg(short, long, default_value_t = 8081)]
        port: u16,
        #[arg(short, long, default_value = "batch")]
        ns: String,
    },
    /// Submits a job to the hub
    Submit {
        #[arg(short, long, default_value = "http://localhost:8080")]
        hub_url: String,
    },
}

struct RelayState {
    hub_url: String,
    gen_id: String,
    cursor: Arc<tokio::sync::Mutex<Cursor>>,
    executor: Arc<RelayExecutor>,
}

async fn relay_poke_handler(
    State(state): State<Arc<RelayState>>,
    Json(poke): Json<Poke>,
) -> StatusCode {
    println!("[Relay] Received poke for generator: {}", poke.gid);
    
    let mut cursor_lock = state.cursor.lock().await;
    match pull_from_hub(&state.hub_url, &state.gen_id, *cursor_lock).await {
        Ok(facts) => {
            println!("[Relay] Pulled facts, checking for jobs...");
            for (job_id, info) in facts.jobs {
                if info.phase == 1 {
                    println!("[Relay] Executing job: {}", job_id);
                    // In a real relay, we'd update our local cursor based on how many events we saw.
                    // For the demo, we just print execution.
                }
            }
            // Advance cursor (mock)
            *cursor_lock += 1; 
            StatusCode::OK
        }
        Err(e) => {
            eprintln!("[Relay] Failed to pull from hub: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Hub { port } => {
            println!("Initializing HPC Hub on port {}...", port);
            let storage: Arc<dyn StorageBackend> = Arc::new(SqliteStorage::new("hpc_hub.db").await?);
            let transport: Arc<dyn TransportBackend> = Arc::new(HttpTransport::new());
            let hub = Hub::new(storage, transport);
            run_hub_server(hub, port).await?;
        }
        Commands::Relay { hub_url, port, ns } => {
            println!("Initializing HPC Relay on port {} for namespace '{}'...", port, ns);
            let gen_id = format!("{}_generator", ns);
            let callback_url = format!("http://localhost:{}/poke", port);

            // 1. Register Namespace (Dynamic)
            if ns == "batch" {
                register_with_hub(&hub_url, ControlRequest::RegisterNamespace(ns.clone(), get_batch_namespace_spec())).await?;
                
                let mut gen_spec = get_batch_generator_spec();
                gen_spec.callback_url = Some(callback_url);
                register_with_hub(&hub_url, ControlRequest::RegisterGenerator(gen_id.clone(), gen_spec)).await?;
            }

            // 2. Start Poke Listener
            let state = Arc::new(RelayState {
                hub_url,
                gen_id,
                cursor: Arc::new(tokio::sync::Mutex::new(0)),
                executor: Arc::new(RelayExecutor::new()),
            });

            let app = Router::new()
                .route("/poke", post(relay_poke_handler))
                .with_state(state);

            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
            println!("Relay listening for pokes on 0.0.0.0:{}", port);
            axum::serve(listener, app).await?;
        }
        Commands::Submit { hub_url } => {
            println!("Simulating job submission to {}...", hub_url);
            let client = reqwest::Client::new();
            
            use hpc_ns_batch::BatchJob;
            use hpc_core::domain::{Event, Payload, SysContext};

            let batch_job = BatchJob {
                command: "echo 'Hello from Distributed HPC'".to_string(),
                cpu: 2,
                memory: 512,
            };

            let event = Event {
                kind: "SubmitJob".to_string(),
                payload: Payload {
                    ctx: SysContext {
                        job_id: Some("dist-job-001".into()),
                        ns_id: Some(BATCH_NS_ID.to_string()),
                        priority: Some(1),
                        ..Default::default()
                    },
                    spec: batch_job.to_spec(),
                },
            };

            let resp = client.post(format!("{}/ingest", hub_url))
                .json(&event)
                .send()
                .await?;
            
            println!("Hub Response: {:?}", resp.status());
            let pokes: Vec<Poke> = resp.json().await?;
            println!("Generated {} pokes", pokes.len());
        }
    }

    Ok(())
}
