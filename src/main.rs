mod game_events;
mod infrai_client;

use axum::{body::Bytes, extract::{Path, State}, http::{HeaderMap, StatusCode}, routing::{get, post}, Json, Router};
use game_events::{route, Decision, GameEvent};
use hmac::{Hmac, Mac};
use infrai_client::{ApiError, InfraiClient};
use serde_json::{json, Value};
use sha2::Sha256;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
struct AppState {
    api: InfraiClient,
    secret: Vec<u8>,
    seen: Arc<Mutex<HashMap<String, Decision>>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = AppState {
        api: InfraiClient::new(std::env::var("INFRAI_API_KEY")?),
        secret: std::env::var("WEBHOOK_SECRET")?.into_bytes(),
        seen: Arc::new(Mutex::new(HashMap::new())),
    };
    let app = Router::new()
        .route("/events", post(receive))
        .route("/deliveries/:id", get(deliveries))
        .route("/redrive/:queue", post(redrive))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn receive(State(state): State<AppState>, headers: HeaderMap, body: Bytes)
    -> Result<Json<Decision>, (StatusCode, Json<Value>)> {
    let signature = headers.get("x-game-signature").and_then(|v| v.to_str().ok())
        .and_then(|v| hex::decode(v).ok()).ok_or_else(|| bad_request("missing signature"))?;
    let mut mac = Hmac::<Sha256>::new_from_slice(&state.secret)
        .map_err(|_| bad_request("invalid secret"))?;
    mac.update(&body);
    mac.verify_slice(&signature).map_err(|_| bad_request("signature mismatch"))?;
    let event: GameEvent = serde_json::from_slice(&body).map_err(|_| bad_request("invalid event"))?;
    let mut seen = state.seen.lock().await;
    let decision = seen.entry(event.event_id.clone()).or_insert_with(|| route(&event)).clone();
    Ok(Json(decision))
}

async fn deliveries(State(state): State<AppState>, Path(id): Path<String>)
    -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    state.api.deliveries(&id).await.map(Json).map_err(api_response)
}

async fn redrive(State(state): State<AppState>, Path(queue): Path<String>)
    -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    state.api.redrive(&queue).await.map(Json).map_err(api_response)
}

fn bad_request(message: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": message })))
}

fn api_response(error: ApiError) -> (StatusCode, Json<Value>) {
    match error {
        ApiError::Rejected { status, detail } if status.is_client_error() =>
            (status, Json(json!({ "error": detail }))),
        other => (StatusCode::BAD_GATEWAY, Json(json!({ "error": other.to_string() }))),
    }
}
