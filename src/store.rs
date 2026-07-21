use crate::model::GameSession;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::{
    fs,
    sync::{Mutex, RwLock},
};

#[derive(Debug, Clone)]
pub struct SessionStore {
    path: PathBuf,
    sessions: Arc<RwLock<HashMap<String, GameSession>>>,
    mutation_lock: Arc<Mutex<()>>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct StoreFile {
    sessions: HashMap<String, GameSession>,
}

impl SessionStore {
    pub async fn load(path: impl Into<PathBuf>) -> std::io::Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut sessions = match fs::read_to_string(&path).await {
            Ok(contents) => {
                serde_json::from_str::<StoreFile>(&contents)
                    .map_err(std::io::Error::other)?
                    .sessions
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => HashMap::new(),
            Err(error) => return Err(error),
        };
        normalize_legacy_sessions(&mut sessions);

        Ok(Self {
            path,
            sessions: Arc::new(RwLock::new(sessions)),
            mutation_lock: Arc::new(Mutex::new(())),
        })
    }

    pub async fn list_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    pub async fn get(&self, id: &str) -> Option<GameSession> {
        self.sessions.read().await.get(id).cloned()
    }

    pub async fn all(&self) -> Vec<GameSession> {
        self.sessions.read().await.values().cloned().collect()
    }

    pub async fn insert(&self, session: GameSession) -> std::io::Result<()> {
        let _mutation = self.mutation_lock.lock().await;
        self.sessions
            .write()
            .await
            .insert(session.id.clone(), session);
        self.persist().await
    }

    pub async fn update<F, T>(&self, id: &str, operation: F) -> Result<T, StoreUpdateError>
    where
        F: FnOnce(&mut GameSession) -> Result<T, crate::engine::GameError>,
    {
        let _mutation = self.mutation_lock.lock().await;
        let result = {
            let mut guard = self.sessions.write().await;
            let session = guard.get_mut(id).ok_or(StoreUpdateError::NotFound)?;
            operation(session).map_err(StoreUpdateError::Game)?
        };
        self.persist().await.map_err(StoreUpdateError::Io)?;
        Ok(result)
    }

    async fn persist(&self) -> std::io::Result<()> {
        let snapshot = self.sessions.read().await.clone();
        let payload = serde_json::to_vec_pretty(&StoreFile { sessions: snapshot })
            .map_err(std::io::Error::other)?;
        let temporary = self.path.with_extension("tmp");
        fs::write(&temporary, payload).await?;
        fs::rename(temporary, &self.path).await?;
        Ok(())
    }
}

fn normalize_legacy_sessions(sessions: &mut HashMap<String, GameSession>) {
    for session in sessions.values_mut() {
        for response in session.action_results.values_mut() {
            if response.state.game_pack_id.is_empty() {
                response
                    .state
                    .game_pack_id
                    .clone_from(&session.game_pack_id);
            }
            if response.state.game_pack_version.is_empty() {
                response
                    .state
                    .game_pack_version
                    .clone_from(&session.game_pack_version);
            }
            if response.state.campaign_id.is_none() {
                response.state.campaign_id.clone_from(&session.campaign_id);
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StoreUpdateError {
    #[error("session not found")]
    NotFound,
    #[error(transparent)]
    Game(#[from] crate::engine::GameError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        engine::{apply_action, create_session},
        model::{ActionRequest, GamePack},
    };

    fn temporary_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "civic-sim-{label}-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[tokio::test]
    async fn corrupt_store_is_reported_instead_of_silently_discarded() {
        let path = temporary_path("corrupt-store");
        fs::write(&path, b"{not-json").await.unwrap();
        assert!(SessionStore::load(&path).await.is_err());
        fs::remove_file(path).await.unwrap();
    }

    #[tokio::test]
    async fn concurrent_inserts_survive_reload() {
        let path = temporary_path("concurrent-store");
        let store = SessionStore::load(&path).await.unwrap();
        let pack: GamePack =
            serde_json::from_str(include_str!("../game-packs/drainage/game.json")).unwrap();
        let mut tasks = Vec::new();
        for index in 0..12 {
            let store = store.clone();
            let session =
                create_session(&pack, "shopkeeper", index, format!("session-{index}")).unwrap();
            tasks.push(tokio::spawn(async move { store.insert(session).await }));
        }
        for task in tasks {
            task.await.unwrap().unwrap();
        }
        let restored = SessionStore::load(&path).await.unwrap();
        assert_eq!(restored.list_count().await, 12);
        fs::remove_file(path).await.unwrap();
    }

    #[tokio::test]
    async fn legacy_cached_public_state_still_loads() {
        let path = temporary_path("legacy-action-type");
        let pack: GamePack =
            serde_json::from_str(include_str!("../game-packs/drainage/game.json")).unwrap();
        let mut session = create_session(&pack, "shopkeeper", 55, "legacy-session".into()).unwrap();
        apply_action(
            &mut session,
            &pack,
            &ActionRequest {
                action_id: "file_complaint".into(),
                client_action_id: "legacy-action".into(),
            },
        )
        .unwrap();

        let mut payload = serde_json::to_value(StoreFile {
            sessions: HashMap::from([(session.id.clone(), session)]),
        })
        .unwrap();
        let state = payload["sessions"]["legacy-session"]["action_results"]["legacy-action"]
            ["state"]
            .as_object_mut()
            .unwrap();
        for field in [
            "game_pack_id",
            "game_pack_version",
            "campaign_id",
            "values",
            "indicators",
            "persistent_consequences",
            "ending_id",
        ] {
            state.remove(field);
        }
        let actions = state["available_actions"].as_array_mut().unwrap();
        for action in actions {
            action.as_object_mut().unwrap().remove("action_type");
            action.as_object_mut().unwrap().remove("location_id");
        }
        fs::write(&path, serde_json::to_vec_pretty(&payload).unwrap())
            .await
            .unwrap();

        let restored = SessionStore::load(&path).await.unwrap();
        let session = restored.get("legacy-session").await.unwrap();
        let cached = session.action_results.get("legacy-action").unwrap();
        assert_eq!(cached.state.game_pack_id, "civic-drainage-v1");
        assert!(cached
            .state
            .available_actions
            .iter()
            .all(|action| action.action_type.is_empty()));
        fs::remove_file(path).await.unwrap();
    }
}
