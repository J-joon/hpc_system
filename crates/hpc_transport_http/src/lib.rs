use async_trait::async_trait;
use hpc_core::contracts::{TransportBackend, DynResult};
use hpc_core::domain::{Poke, Event, Cursor, GenId, ControlRequest};
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
    pub base_url: String,
}

impl HttpTransport {
    pub fn new(url: &str) -> Self {
        Self { base_url: url.to_string() }
    }
}

#[async_trait]
impl TransportBackend for HttpTransport {
    async fn send_poke(&self, poke: &Poke) -> DynResult<()> {
        println!("[Transport] Poking generator: {}", poke.gid);
        Ok(())
    }
}

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
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Ingest failed").into_response(),
    }
}

pub async fn pull_handler(
    State(state): State<Arc<HubState>>,
    Query(params): Query<PullParams>,
) -> impl IntoResponse {
    let hub = state.hub.read().await;
    match hub.handle_pull(&params.gid, params.cursor).await {
        Ok(facts) => (StatusCode::OK, Json(facts)).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Pull failed").into_response(),
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
