use async_trait::async_trait;
use hpc_core::contracts::{TransportBackend, DynResult};
use hpc_core::domain::{Poke, Event, Cursor, GenId, ControlRequest, Facts};
use hpc_logic::Hub;
use std::sync::Arc;
use tokio::sync::RwLock;
use axum::{
    routing::{get, post},
    extract::{State, Query},
    Json, Router,
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;

pub struct HttpTransport {
    pub client: reqwest::Client,
}

impl HttpTransport {
    pub fn new() -> Self {
        Self { client: reqwest::Client::new() }
    }
}

#[async_trait]
impl TransportBackend for HttpTransport {
    async fn send_poke(&self, poke: &Poke) -> DynResult<()> {
        println!("[Transport] Mock poke for generator: {}", poke.gid);
        Ok(())
    }

    async fn send_poke_to_url(&self, poke: &Poke, url: &str) -> DynResult<()> {
        println!("[Transport] Sending HTTP poke to {}", url);
        self.client.post(url)
            .json(poke)
            .send()
            .await?;
        Ok(())
    }
}

// --- Hub Server Implementation ---

pub struct HubState {
    pub hub: RwLock<Hub>,
}

#[derive(Deserialize)]
pub struct PullParams {
    pub gid: GenId,
    pub cursor: Cursor,
}

pub async fn ingest_handler(
    State(state): State<Arc<HubState>>,
    Json(event): Json<Event>,
) -> impl IntoResponse {
    let hub = state.hub.read().await;
    match hub.ingest_event(event).await {
        Ok(pokes) => (StatusCode::OK, Json(pokes)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Ingest failed: {}", e)).into_response(),
    }
}

pub async fn pull_handler(
    State(state): State<Arc<HubState>>,
    Query(params): Query<PullParams>,
) -> impl IntoResponse {
    let hub = state.hub.read().await;
    match hub.handle_pull(&params.gid, params.cursor).await {
        Ok(facts) => (StatusCode::OK, Json(facts)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Pull failed: {}", e)).into_response(),
    }
}

pub async fn control_handler(
    State(state): State<Arc<HubState>>,
    Json(req): Json<ControlRequest>,
) -> impl IntoResponse {
    let mut hub = state.hub.write().await;
    hub.process_control(req);
    StatusCode::OK
}

pub fn create_hub_router(hub: Hub) -> Router {
    let state = Arc::new(HubState {
        hub: RwLock::new(hub),
    });

    Router::new()
        .route("/ingest", post(ingest_handler))
        .route("/pull", get(pull_handler))
        .route("/control", post(control_handler))
        .with_state(state)
}

pub async fn run_hub_server(hub: Hub, port: u16) -> DynResult<()> {
    let app = create_hub_router(hub);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("Hub server listening on 0.0.0.0:{}", port);
    axum::serve(listener, app).await?;
    Ok(())
}

// --- Relay Client Helpers ---

pub async fn register_with_hub(hub_url: &str, req: ControlRequest) -> DynResult<()> {
    let client = reqwest::Client::new();
    client.post(format!("{}/control", hub_url))
        .json(&req)
        .send()
        .await?;
    Ok(())
}

pub async fn pull_from_hub(hub_url: &str, gid: &str, cursor: u64) -> DynResult<Facts> {
    let client = reqwest::Client::new();
    let facts = client.get(format!("{}/pull", hub_url))
        .query(&[("gid", gid), ("cursor", &cursor.to_string())])
        .send()
        .await?
        .json::<Facts>()
        .await?;
    Ok(facts)
}

pub async fn ingest_event_to_hub(hub_url: &str, event: &Event) -> DynResult<Vec<Poke>> {
    let client = reqwest::Client::new();
    let pokes = client.post(format!("{}/ingest", hub_url))
        .json(event)
        .send()
        .await?
        .json::<Vec<Poke>>()
        .await?;
    Ok(pokes)
}
