mod services;

use std::sync::Arc;
use clap::{Parser, Subcommand};
use hpc_logic::Hub;
use hpc_core::contracts::{StorageBackend, TransportBackend};
use hpc_core::domain::{ControlRequest, Poke, Cursor, Event, Payload, SysContext};
use hpc_storage_sqlite::SqliteStorage;
use hpc_transport_http::{HttpTransport, run_hub_server, register_with_hub, pull_from_hub, ingest_event_to_hub};
use hpc_ns_batch::{BATCH_NS_ID, get_batch_namespace_spec, get_batch_generator_spec, BatchJob};
use hpc_ns_simulation::{SIM_NS_ID, get_simulation_namespace_spec, get_simulation_generator_spec, SimulationJob};
use hpc_executor_relay::RelayExecutor;
use hpc_resource_monitor::{
    get_monitor_namespace_spec, get_monitor_generator_spec, MONITOR_NS_ID, MONITOR_GEN_ID
};
use axum::{Router, routing::post, Json, extract::State, http::StatusCode};
use std::collections::HashMap;

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
        #[arg(long, default_value = "http://localhost:8080")]
        hub_url: String,
        #[arg(short, long, default_value_t = 8081)]
        port: u16,
        #[arg(short, long, default_value = "batch")]
        ns: String,
    },
    /// Submits a job to the hub
    Submit {
        #[arg(long, default_value = "http://localhost:8080")]
        hub_url: String,
        #[arg(long, default_value = "batch")]
        ns: String,
    },
    /// Starts a hardware resource monitor
    Monitor {
        #[arg(long, default_value = "http://localhost:8080")]
        hub_url: String,
        #[arg(short, long, default_value = "node-001")]
        node_id: String,
        #[arg(short, long, default_value_t = 5)]
        interval: u64,
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
                    println!("[Relay] Detected new job: {}", job_id);
                    
                    let hub_url = state.hub_url.clone();
                    let executor = state.executor.clone();
                    let gen_id = state.gen_id.clone();
                    
                    if gen_id.contains("batch") {
                        tokio::spawn(async move {
                            println!("[Relay] Starting batch job: {}", job_id);
                            let _ = ingest_event_to_hub(&hub_url, &Event {
                                kind: "JobStarted".to_string(),
                                payload: Payload {
                                    ctx: SysContext { job_id: Some(job_id.clone()), ..Default::default() },
                                    spec: Default::default(),
                                },
                            }).await;

                            // Mock command execution
                            let _ = executor.run_command("sleep 2 && echo 'Batch job finished'").await;

                            let _ = ingest_event_to_hub(&hub_url, &Event {
                                kind: "JobFinished".to_string(),
                                payload: Payload {
                                    ctx: SysContext { job_id: Some(job_id.clone()), ..Default::default() },
                                    spec: Default::default(),
                                },
                            }).await;
                        });
                    } else if gen_id.contains("simulation") {
                        tokio::spawn(async move {
                            println!("[Relay] Starting simulation job: {}", job_id);
                            let _ = ingest_event_to_hub(&hub_url, &Event {
                                kind: "SimulationStarted".to_string(),
                                payload: Payload {
                                    ctx: SysContext { job_id: Some(job_id.clone()), ..Default::default() },
                                    spec: Default::default(),
                                },
                            }).await;

                            // 1. Start Env
                            println!("[Relay] [Simulation] Starting environment server...");
                            // We simulate port discovery: environment writes its port to a file
                            let _ = executor.run_command("echo 5432 > .env_port").await;
                            let port = "5432"; // Mocked discovery

                            let mut port_spec = HashMap::new();
                            port_spec.insert("port".to_string(), port.to_string());
                            let _ = ingest_event_to_hub(&hub_url, &Event {
                                kind: "PortAllocated".to_string(),
                                payload: Payload {
                                    ctx: SysContext { job_id: Some(job_id.clone()), ..Default::default() },
                                    spec: port_spec,
                                },
                            }).await;

                            // 2. Start Policy with discovered port
                            println!("[Relay] [Simulation] Starting policy module with port {}...", port);
                            let _ = executor.run_command(&format!("sleep 1 && echo 'Policy connected to {}'", port)).await;

                            let _ = ingest_event_to_hub(&hub_url, &Event {
                                kind: "SimulationFinished".to_string(),
                                payload: Payload {
                                    ctx: SysContext { job_id: Some(job_id.clone()), ..Default::default() },
                                    spec: Default::default(),
                                },
                            }).await;
                        });
                    }
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
            let mut hub = Hub::new(storage, transport);
            
            // Register known delta functions for the demo
            hub.delta_fns.insert("batch".to_string(), hpc_ns_batch::batch_delta_fn);
            hub.delta_fns.insert("monitor".to_string(), hpc_resource_monitor::monitor_delta_fn);
            hub.delta_fns.insert("simulation".to_string(), hpc_ns_simulation::simulation_delta_fn);

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
            } else if ns == "simulation" {
                register_with_hub(&hub_url, ControlRequest::RegisterNamespace(ns.clone(), get_simulation_namespace_spec())).await?;
                let mut gen_spec = get_simulation_generator_spec();
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
        Commands::Submit { hub_url, ns } => {
            println!("Simulating job submission for namespace '{}' to {}...", ns, hub_url);
            let client = reqwest::Client::new();
            
            let (kind, job_id, ns_id, spec) = if ns == "batch" {
                let job = BatchJob { command: "echo 'hello'".to_string(), cpu: 1, memory: 128 };
                ("SubmitJob", "batch-001", BATCH_NS_ID, job.to_spec())
            } else {
                let job = SimulationJob { env_cmd: "./env.sh".to_string(), policy_cmd: "./policy.sh".to_string() };
                ("SubmitSimulation", "sim-001", SIM_NS_ID, job.to_spec())
            };

            let event = Event {
                kind: kind.to_string(),
                payload: Payload {
                    ctx: SysContext {
                        job_id: Some(job_id.into()),
                        ns_id: Some(ns_id.to_string()),
                        priority: Some(1),
                        ..Default::default()
                    },
                    spec,
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
        Commands::Monitor { hub_url, node_id, interval } => {
            println!("Registering monitor namespace with Hub at {}...", hub_url);
            
            register_with_hub(&hub_url, ControlRequest::RegisterNamespace(
                MONITOR_NS_ID.to_string(), 
                get_monitor_namespace_spec()
            )).await?;
            
            register_with_hub(&hub_url, ControlRequest::RegisterGenerator(
                MONITOR_GEN_ID.to_string(), 
                get_monitor_generator_spec()
            )).await?;

            println!("Starting resource monitor for node {} reporting to {} every {}s...", node_id, hub_url, interval);
            let mut monitor = hpc_resource_monitor::ResourceMonitor::new(hub_url, node_id, interval);
            monitor.run().await?;
        }
    }

    Ok(())
}
