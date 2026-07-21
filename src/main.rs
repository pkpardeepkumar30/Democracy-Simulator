mod engine;
mod model;
mod static_assets;
mod store;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use engine::{apply_action, create_session, public_state};
use model::*;
use rand::Rng;
use serde::Serialize;
use std::{env, net::SocketAddr, sync::Arc};
use store::{SessionStore, StoreUpdateError};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    pack: Arc<GamePack>,
    store: SessionStore,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "civic_sim_server=info,tower_http=info".into()),
        )
        .init();

    let pack_path = env::var("GAME_PACK_PATH")
        .unwrap_or_else(|_| "game-packs/drainage/game.json".to_string());
    let pack_contents = tokio::fs::read_to_string(&pack_path).await?;
    let pack: GamePack = serde_json::from_str(&pack_contents)?;

    let store_path = env::var("SESSION_STORE_PATH")
        .unwrap_or_else(|_| "/data/sessions.json".to_string());
    let store = SessionStore::load(store_path).await?;
    info!(sessions = store.list_count().await, game_pack = %pack.id, "loaded game server");

    let state = AppState {
        pack: Arc::new(pack),
        store,
    };

    let app = Router::new()
        .route("/", get(static_assets::index))
        .route("/app.js", get(static_assets::app_js))
        .route("/styles.css", get(static_assets::styles_css))
        .route("/manifest.webmanifest", get(static_assets::manifest))
        .route("/sw.js", get(static_assets::service_worker))
        .route("/favicon.ico", get(static_assets::favicon))
        .route("/api/v1/health", get(health))
        .route("/api/v1/scenario", get(get_scenario))
        .route("/api/v1/sessions", post(create_game_session))
        .route("/api/v1/sessions/{id}", get(get_session))
        .route("/api/v1/sessions/{id}/actions", post(take_action))
        .route("/api/v1/sessions/{id}/reset", post(reset_session))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8080);
    let address = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(address).await?;
    info!(%address, "server listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "game_pack": state.pack.id.clone(),
        "version": env!("CARGO_PKG_VERSION"),
        "sessions": state.store.list_count().await
    }))
}

async fn get_scenario(State(state): State<AppState>) -> Json<PublicScenario> {
    Json(PublicScenario::from(state.pack.as_ref()))
}

async fn create_game_session(
    State(state): State<AppState>,
    Json(request): Json<CreateSessionRequest>,
) -> Result<(StatusCode, Json<PublicGameState>), ApiError> {
    let seed: u64 = rand::rng().random();
    let id = Uuid::new_v4().to_string();
    let session = create_session(&state.pack, &request.citizen_id, seed, id)?;
    let public = public_state(&session, &state.pack);
    state.store.insert(session).await?;
    Ok((StatusCode::CREATED, Json(public)))
}

async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PublicGameState>, ApiError> {
    let session = state.store.get(&id).await.ok_or(ApiError::NotFound)?;
    Ok(Json(public_state(&session, &state.pack)))
}

async fn take_action(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<ActionRequest>,
) -> Result<Json<ActionResponse>, ApiError> {
    let pack = state.pack.clone();
    let response = state
        .store
        .update(&id, move |session| apply_action(session, &pack, &request))
        .await?;
    Ok(Json(response))
}

async fn reset_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PublicGameState>, ApiError> {
    let old = state.store.get(&id).await.ok_or(ApiError::NotFound)?;
    let seed: u64 = rand::rng().random();
    let reset = create_session(&state.pack, &old.citizen_id, seed, id)?;
    let public = public_state(&reset, &state.pack);
    state.store.insert(reset).await?;
    Ok(Json(public))
}

#[derive(Debug)]
enum ApiError {
    NotFound,
    Game(engine::GameError),
    Store(StoreUpdateError),
    Io(std::io::Error),
}

impl From<engine::GameError> for ApiError {
    fn from(value: engine::GameError) -> Self {
        Self::Game(value)
    }
}

impl From<StoreUpdateError> for ApiError {
    fn from(value: StoreUpdateError) -> Self {
        match value {
            StoreUpdateError::NotFound => Self::NotFound,
            other => Self::Store(other),
        }
    }
}

impl From<std::io::Error> for ApiError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            ApiError::NotFound => (StatusCode::NOT_FOUND, "session not found".to_string()),
            ApiError::Game(error) => (StatusCode::BAD_REQUEST, error.to_string()),
            ApiError::Store(error) => {
                error!(?error, "session store error");
                (StatusCode::INTERNAL_SERVER_ERROR, "could not save the game".to_string())
            }
            ApiError::Io(error) => {
                error!(?error, "I/O error");
                (StatusCode::INTERNAL_SERVER_ERROR, "server storage error".to_string())
            }
        };
        (status, Json(ErrorBody { error: message })).into_response()
    }
}
