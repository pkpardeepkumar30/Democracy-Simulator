use crate::model::GameSession;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::{fs, sync::RwLock};

#[derive(Debug, Clone)]
pub struct SessionStore {
    path: PathBuf,
    sessions: Arc<RwLock<HashMap<String, GameSession>>>,
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

        let sessions = match fs::read_to_string(&path).await {
            Ok(contents) => serde_json::from_str::<StoreFile>(&contents)
                .unwrap_or_default()
                .sessions,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => HashMap::new(),
            Err(error) => return Err(error),
        };

        Ok(Self {
            path,
            sessions: Arc::new(RwLock::new(sessions)),
        })
    }

    pub async fn list_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    pub async fn get(&self, id: &str) -> Option<GameSession> {
        self.sessions.read().await.get(id).cloned()
    }

    pub async fn insert(&self, session: GameSession) -> std::io::Result<()> {
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

#[derive(Debug, thiserror::Error)]
pub enum StoreUpdateError {
    #[error("session not found")]
    NotFound,
    #[error(transparent)]
    Game(#[from] crate::engine::GameError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
