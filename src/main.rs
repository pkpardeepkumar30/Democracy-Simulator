mod static_assets;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use civic_sim_server::{
    campaign::{
        apply_campaign_imports, Campaign, CampaignError, CampaignStore, CampaignSummary,
        CreateCampaignRequest,
    },
    engine::{self, apply_action, create_session, public_state},
    game_pack::PackRegistry,
    generator::{
        generate_pack, AbstractionCatalog, GenerateScenarioRequest, GeneratedPackStore,
        GenerationError,
    },
    model::*,
    store::{SessionStore, StoreUpdateError},
};
use rand::Rng;
use serde::Serialize;
use std::{collections::HashSet, env, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    packs: Arc<RwLock<PackRegistry>>,
    catalog: Arc<AbstractionCatalog>,
    generated_packs: GeneratedPackStore,
    campaigns: CampaignStore,
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

    let pack_path = PathBuf::from(
        env::var("GAME_PACK_PATH").unwrap_or_else(|_| "game-packs/drainage/game.json".to_string()),
    );
    let packs_path = env::var("GAME_PACKS_PATH")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            PathBuf::from("game-packs")
                .is_dir()
                .then(|| PathBuf::from("game-packs"))
        });
    let default_pack_id = env::var("DEFAULT_GAME_PACK_ID")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let mut packs = PackRegistry::load(
        packs_path.as_deref(),
        &pack_path,
        default_pack_id.as_deref(),
    )?;

    let catalog_path = PathBuf::from(
        env::var("ABSTRACTION_CATALOG_PATH")
            .unwrap_or_else(|_| "game-packs/abstractions.json".to_string()),
    );
    let pack_ids: HashSet<_> = packs.all().iter().map(|pack| pack.id.clone()).collect();
    let catalog = AbstractionCatalog::load(&catalog_path, &pack_ids)?;
    let generated_pack_path = env::var("GENERATED_PACK_STORE_PATH")
        .unwrap_or_else(|_| "/data/generated-packs.json".to_string());
    let generated_packs = GeneratedPackStore::load(generated_pack_path).await?;
    for pack in generated_packs.all().await {
        packs.insert(pack).map_err(|errors| {
            std::io::Error::other(format!("invalid generated pack store: {errors:?}"))
        })?;
    }

    let store_path =
        env::var("SESSION_STORE_PATH").unwrap_or_else(|_| "/data/sessions.json".to_string());
    let store = SessionStore::load(store_path).await?;
    let campaign_store_path =
        env::var("CAMPAIGN_STORE_PATH").unwrap_or_else(|_| "/data/campaigns.json".to_string());
    let campaigns = CampaignStore::load(campaign_store_path).await?;
    for session in store.all().await {
        if session.campaign_id.is_some() && session.status != SessionStatus::Active {
            if let Some(pack) = packs.get(&session.game_pack_id) {
                if let Err(error) = campaigns.record_completed(&session, &pack).await {
                    warn!(session = %session.id, ?error, "could not reconcile campaign history");
                }
            }
        }
    }
    info!(
        sessions = store.list_count().await,
        scenarios = packs.len(),
        default_game_pack = %packs.default_id(),
        "loaded game server"
    );

    let state = AppState {
        packs: Arc::new(RwLock::new(packs)),
        catalog: Arc::new(catalog),
        generated_packs,
        campaigns,
        store,
    };

    let app = Router::new()
        .route("/", get(static_assets::index))
        .route("/app.js", get(static_assets::app_js))
        .route("/city.bundle.js", get(static_assets::city_bundle_js))
        .route("/styles.css", get(static_assets::styles_css))
        .route("/manifest.webmanifest", get(static_assets::manifest))
        .route("/sw.js", get(static_assets::service_worker))
        .route("/robots.txt", get(static_assets::robots_txt))
        .route("/sitemap.xml", get(static_assets::sitemap))
        .route("/favicon.ico", get(static_assets::favicon))
        .route("/api/v1/health", get(health))
        .route("/api/v1/scenario", get(get_scenario))
        .route("/api/v1/scenarios", get(list_scenarios))
        .route("/api/v1/scenario-generator", get(get_generator_catalog))
        .route("/api/v1/scenarios/generate", post(generate_scenario))
        .route("/api/v1/scenarios/{id}", get(get_scenario_by_id))
        .route(
            "/api/v1/campaigns",
            get(list_campaigns).post(create_campaign),
        )
        .route("/api/v1/campaigns/{id}", get(get_campaign))
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
    let packs = state.packs.read().await;
    Json(serde_json::json!({
        "status": "ok",
        "game_pack": packs.default_id(),
        "scenarios": packs.len(),
        "version": env!("CARGO_PKG_VERSION"),
        "campaigns_supported": true,
        "sessions": state.store.list_count().await
    }))
}

async fn list_campaigns(State(state): State<AppState>) -> Json<Vec<CampaignSummary>> {
    Json(state.campaigns.list().await)
}

async fn create_campaign(
    State(state): State<AppState>,
    Json(request): Json<CreateCampaignRequest>,
) -> Result<(StatusCode, Json<Campaign>), ApiError> {
    let campaign = state
        .campaigns
        .create(Uuid::new_v4().to_string(), request.name)
        .await?;
    Ok((StatusCode::CREATED, Json(campaign)))
}

async fn get_campaign(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Campaign>, ApiError> {
    state
        .campaigns
        .get(&id)
        .await
        .map(Json)
        .ok_or(ApiError::Campaign(CampaignError::NotFound))
}

async fn get_scenario(State(state): State<AppState>) -> Json<PublicScenario> {
    Json(PublicScenario::from(
        state.packs.read().await.default_pack().as_ref(),
    ))
}

async fn list_scenarios(State(state): State<AppState>) -> Json<Vec<ScenarioSummary>> {
    Json(state.packs.read().await.summaries())
}

async fn get_generator_catalog(State(state): State<AppState>) -> Json<AbstractionCatalog> {
    Json(state.catalog.as_ref().clone())
}

async fn generate_scenario(
    State(state): State<AppState>,
    Json(request): Json<GenerateScenarioRequest>,
) -> Result<(StatusCode, Json<PublicScenario>), ApiError> {
    let pack = {
        let packs = state.packs.read().await;
        generate_pack(&state.catalog, request, |id| packs.get(id))?
    };
    state.generated_packs.insert(pack.clone()).await?;
    state
        .packs
        .write()
        .await
        .insert(pack.clone())
        .map_err(GenerationError::InvalidPack)?;
    Ok((StatusCode::CREATED, Json(PublicScenario::from(&pack))))
}

async fn get_scenario_by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PublicScenario>, ApiError> {
    let pack = state
        .packs
        .read()
        .await
        .get(&id)
        .ok_or(ApiError::ScenarioNotFound)?;
    Ok(Json(PublicScenario::from(pack.as_ref())))
}

async fn create_game_session(
    State(state): State<AppState>,
    Json(request): Json<CreateSessionRequest>,
) -> Result<(StatusCode, Json<PublicGameState>), ApiError> {
    let pack = match request.scenario_id.as_deref() {
        Some(id) => state
            .packs
            .read()
            .await
            .get(id)
            .ok_or(ApiError::ScenarioNotFound)?,
        None => state.packs.read().await.default_pack(),
    };
    let profile_id = request.requested_profile_id();
    let seed: u64 = rand::rng().random();
    let id = Uuid::new_v4().to_string();
    let mut session = create_session(&pack, profile_id, seed, id)?;
    if let Some(campaign_id) = request.campaign_id.as_deref() {
        let campaign = state
            .campaigns
            .get(campaign_id)
            .await
            .ok_or(ApiError::Campaign(CampaignError::NotFound))?;
        apply_campaign_imports(&campaign, &mut session, &pack);
    }
    let public = public_state(&session, &pack);
    state.store.insert(session).await?;
    Ok((StatusCode::CREATED, Json(public)))
}

async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PublicGameState>, ApiError> {
    let session = state.store.get(&id).await.ok_or(ApiError::NotFound)?;
    let pack = state
        .packs
        .read()
        .await
        .get(&session.game_pack_id)
        .ok_or(ApiError::ScenarioNotFound)?;
    Ok(Json(public_state(&session, &pack)))
}

async fn take_action(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<ActionRequest>,
) -> Result<Json<ActionResponse>, ApiError> {
    let session = state.store.get(&id).await.ok_or(ApiError::NotFound)?;
    let pack = state
        .packs
        .read()
        .await
        .get(&session.game_pack_id)
        .ok_or(ApiError::ScenarioNotFound)?;
    let action_pack = pack.clone();
    let response = state
        .store
        .update(&id, move |session| {
            apply_action(session, &action_pack, &request)
        })
        .await?;
    if response.state.status != SessionStatus::Active && response.state.campaign_id.is_some() {
        let completed = state.store.get(&id).await.ok_or(ApiError::NotFound)?;
        state.campaigns.record_completed(&completed, &pack).await?;
    }
    Ok(Json(response))
}

async fn reset_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PublicGameState>, ApiError> {
    let old = state.store.get(&id).await.ok_or(ApiError::NotFound)?;
    let pack = state
        .packs
        .read()
        .await
        .get(&old.game_pack_id)
        .ok_or(ApiError::ScenarioNotFound)?;
    let seed: u64 = rand::rng().random();
    let mut reset = create_session(&pack, &old.citizen_id, seed, id)?;
    reset.campaign_attempt = old.campaign_attempt.saturating_add(1);
    if let Some(campaign_id) = old.campaign_id.as_deref() {
        let campaign = state
            .campaigns
            .get(campaign_id)
            .await
            .ok_or(ApiError::Campaign(CampaignError::NotFound))?;
        apply_campaign_imports(&campaign, &mut reset, &pack);
    }
    let public = public_state(&reset, &pack);
    state.store.insert(reset).await?;
    Ok(Json(public))
}

#[derive(Debug)]
enum ApiError {
    NotFound,
    ScenarioNotFound,
    Game(engine::GameError),
    Store(StoreUpdateError),
    Io(std::io::Error),
    Generation(GenerationError),
    Campaign(CampaignError),
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
            StoreUpdateError::Game(error) => Self::Game(error),
            error @ StoreUpdateError::Io(_) => Self::Store(error),
        }
    }
}

impl From<std::io::Error> for ApiError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<GenerationError> for ApiError {
    fn from(value: GenerationError) -> Self {
        Self::Generation(value)
    }
}

impl From<CampaignError> for ApiError {
    fn from(value: CampaignError) -> Self {
        Self::Campaign(value)
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
            ApiError::ScenarioNotFound => (StatusCode::NOT_FOUND, "scenario not found".to_string()),
            ApiError::Game(error) => (StatusCode::BAD_REQUEST, error.to_string()),
            ApiError::Store(error) => {
                error!(?error, "session store error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "could not save the game".to_string(),
                )
            }
            ApiError::Io(error) => {
                error!(?error, "I/O error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "server storage error".to_string(),
                )
            }
            ApiError::Generation(error) => (StatusCode::BAD_REQUEST, error.to_string()),
            ApiError::Campaign(CampaignError::NotFound) => {
                (StatusCode::NOT_FOUND, "campaign not found".to_string())
            }
            ApiError::Campaign(CampaignError::Io(error)) => {
                error!(?error, "campaign store error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "could not save campaign history".to_string(),
                )
            }
            ApiError::Campaign(error) => (StatusCode::BAD_REQUEST, error.to_string()),
        };
        (status, Json(ErrorBody { error: message })).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejected_actions_are_client_errors_not_store_failures() {
        let error: ApiError = StoreUpdateError::Game(engine::GameError::ActionExhausted).into();
        assert_eq!(error.into_response().status(), StatusCode::BAD_REQUEST);
    }
}
