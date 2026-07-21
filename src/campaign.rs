use crate::{
    engine::{apply_inherited_value, state_value},
    model::{CampaignTransfer, GameEvent, GamePack, GameSession, SessionStatus, StateValues},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::{
    fs,
    sync::{Mutex, RwLock},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campaign {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub values: StateValues,
    #[serde(default)]
    pub missions: Vec<CampaignMissionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignMissionRecord {
    pub mission_key: String,
    pub session_id: String,
    pub attempt: u32,
    pub game_pack_id: String,
    pub game_pack_version: String,
    pub mission_title: String,
    pub citizen_id: String,
    pub citizen_name: String,
    pub status: SessionStatus,
    pub ending_id: Option<String>,
    pub turns: u32,
    pub events: Vec<GameEvent>,
    pub exported_changes: StateValues,
    pub campaign_values_after: StateValues,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignSummary {
    pub id: String,
    pub name: String,
    pub mission_count: usize,
    pub values: StateValues,
}

impl From<&Campaign> for CampaignSummary {
    fn from(value: &Campaign) -> Self {
        Self {
            id: value.id.clone(),
            name: value.name.clone(),
            mission_count: value.missions.len(),
            values: value.values.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateCampaignRequest {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct CampaignStore {
    path: PathBuf,
    campaigns: Arc<RwLock<HashMap<String, Campaign>>>,
    mutation_lock: Arc<Mutex<()>>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct CampaignFile {
    #[serde(default = "store_version")]
    format_version: u32,
    #[serde(default)]
    campaigns: HashMap<String, Campaign>,
}

fn store_version() -> u32 {
    1
}

impl CampaignStore {
    pub async fn load(path: impl Into<PathBuf>) -> std::io::Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let campaigns = match fs::read_to_string(&path).await {
            Ok(contents) => {
                serde_json::from_str::<CampaignFile>(&contents)
                    .map_err(std::io::Error::other)?
                    .campaigns
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => HashMap::new(),
            Err(error) => return Err(error),
        };
        Ok(Self {
            path,
            campaigns: Arc::new(RwLock::new(campaigns)),
            mutation_lock: Arc::new(Mutex::new(())),
        })
    }

    pub async fn list(&self) -> Vec<CampaignSummary> {
        let mut values: Vec<_> = self
            .campaigns
            .read()
            .await
            .values()
            .map(CampaignSummary::from)
            .collect();
        values.sort_by(|left, right| left.name.cmp(&right.name));
        values
    }

    pub async fn get(&self, id: &str) -> Option<Campaign> {
        self.campaigns.read().await.get(id).cloned()
    }

    pub async fn create(&self, id: String, name: String) -> Result<Campaign, CampaignError> {
        let _mutation = self.mutation_lock.lock().await;
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 80 {
            return Err(CampaignError::InvalidName);
        }
        let campaign = Campaign {
            id: id.clone(),
            name: name.to_string(),
            values: StateValues::new(),
            missions: Vec::new(),
        };
        self.campaigns.write().await.insert(id, campaign.clone());
        self.persist().await?;
        Ok(campaign)
    }

    pub async fn record_completed(
        &self,
        session: &GameSession,
        pack: &GamePack,
    ) -> Result<Campaign, CampaignError> {
        let _mutation = self.mutation_lock.lock().await;
        let campaign_id = session
            .campaign_id
            .as_deref()
            .ok_or(CampaignError::NotLinked)?;
        if session.status == SessionStatus::Active {
            return Err(CampaignError::SessionActive);
        }
        let mission_key = format!("{}:{}", session.id, session.campaign_attempt);
        let result = {
            let mut campaigns = self.campaigns.write().await;
            let campaign = campaigns
                .get_mut(campaign_id)
                .ok_or(CampaignError::NotFound)?;
            if campaign
                .missions
                .iter()
                .any(|record| record.mission_key == mission_key)
            {
                return Ok(campaign.clone());
            }
            let mut exported_changes = StateValues::new();
            for transfer in &pack.campaign.exports {
                let source = state_value(session, &transfer.source_id);
                let change = transfer_amount(source, transfer);
                if change != 0 {
                    *campaign
                        .values
                        .entry(transfer.target_id.clone())
                        .or_default() += change;
                    *exported_changes
                        .entry(transfer.target_id.clone())
                        .or_default() += change;
                }
            }
            let ending_effect = match session.status {
                SessionStatus::Won => &pack.campaign.won_effect,
                SessionStatus::Lost => &pack.campaign.lost_effect,
                SessionStatus::Active => unreachable!(),
            };
            for (id, change) in ending_effect {
                *campaign.values.entry(id.clone()).or_default() += change;
                *exported_changes.entry(id.clone()).or_default() += change;
            }
            campaign.missions.push(CampaignMissionRecord {
                mission_key,
                session_id: session.id.clone(),
                attempt: session.campaign_attempt,
                game_pack_id: session.game_pack_id.clone(),
                game_pack_version: session.game_pack_version.clone(),
                mission_title: session.mission_title.clone(),
                citizen_id: session.citizen_id.clone(),
                citizen_name: session.citizen_name.clone(),
                status: session.status.clone(),
                ending_id: session.ending_id.clone(),
                turns: session.turn,
                events: session.events.clone(),
                exported_changes,
                campaign_values_after: campaign.values.clone(),
            });
            campaign.clone()
        };
        self.persist().await?;
        Ok(result)
    }

    async fn persist(&self) -> std::io::Result<()> {
        let payload = serde_json::to_vec_pretty(&CampaignFile {
            format_version: 1,
            campaigns: self.campaigns.read().await.clone(),
        })
        .map_err(std::io::Error::other)?;
        let temporary = self.path.with_extension("tmp");
        fs::write(&temporary, payload).await?;
        fs::rename(temporary, &self.path).await
    }
}

pub fn apply_campaign_imports(campaign: &Campaign, session: &mut GameSession, pack: &GamePack) {
    session.campaign_id = Some(campaign.id.clone());
    for transfer in &pack.campaign.imports {
        let source = campaign
            .values
            .get(&transfer.source_id)
            .copied()
            .unwrap_or_default();
        let change = transfer_amount(source, transfer);
        if change != 0 {
            apply_inherited_value(session, pack, &transfer.target_id, change);
            *session
                .persistent_consequences
                .entry(format!("campaign_import_{}", transfer.target_id))
                .or_default() += change;
        }
    }
}

fn transfer_amount(source: i32, transfer: &CampaignTransfer) -> i32 {
    let amount = ((source - transfer.source_offset) as f64 * transfer.multiplier).round() as i32;
    amount.clamp(
        transfer.min.unwrap_or(i32::MIN),
        transfer.max.unwrap_or(i32::MAX),
    )
}

#[derive(Debug, thiserror::Error)]
pub enum CampaignError {
    #[error("campaign not found")]
    NotFound,
    #[error("session is not linked to a campaign")]
    NotLinked,
    #[error("active sessions cannot be recorded in campaign history")]
    SessionActive,
    #[error("campaign name must contain 1 to 80 characters")]
    InvalidName,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::create_session;

    #[test]
    fn campaign_values_modify_a_new_session_through_pack_rules() {
        let pack: GamePack =
            serde_json::from_str(include_str!("../game-packs/drainage/game.json")).unwrap();
        let mut session = create_session(&pack, "shopkeeper", 9, "session".into()).unwrap();
        let support_before = state_value(&session, "community_support");
        let campaign = Campaign {
            id: "campaign".into(),
            name: "Test".into(),
            values: [("civic_reputation".into(), 40)].into_iter().collect(),
            missions: Vec::new(),
        };
        apply_campaign_imports(&campaign, &mut session, &pack);
        assert_eq!(session.campaign_id.as_deref(), Some("campaign"));
        assert!(state_value(&session, "community_support") > support_before);
    }

    #[tokio::test]
    async fn completed_mission_is_recorded_once_and_survives_reload() {
        let pack: GamePack =
            serde_json::from_str(include_str!("../game-packs/drainage/game.json")).unwrap();
        let root = std::env::temp_dir().join(format!(
            "civic-sim-campaign-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = root.join("campaigns.json");
        let store = CampaignStore::load(&path).await.unwrap();
        let campaign = store
            .create("campaign".into(), "Long civic arc".into())
            .await
            .unwrap();
        let mut session = create_session(&pack, "shopkeeper", 9, "session".into()).unwrap();
        session.campaign_id = Some(campaign.id);
        session.status = SessionStatus::Won;
        session.metrics.integrity = 90;
        session.values.insert("integrity".into(), 90);
        store.record_completed(&session, &pack).await.unwrap();
        store.record_completed(&session, &pack).await.unwrap();
        let restored = CampaignStore::load(&path).await.unwrap();
        let campaign = restored.get("campaign").await.unwrap();
        assert_eq!(campaign.missions.len(), 1);
        assert!(campaign.values["civic_reputation"] > 0);
        std::fs::remove_dir_all(root).unwrap();
    }
}
