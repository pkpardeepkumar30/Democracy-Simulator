use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePack {
    pub id: String,
    pub title: String,
    pub description: String,
    pub version: String,
    pub mission: MissionDefinition,
    pub citizens: Vec<CitizenProfile>,
    pub actions: Vec<ActionDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionDefinition {
    pub title: String,
    pub objective: String,
    pub starting_status: String,
    pub win_progress: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitizenProfile {
    pub id: String,
    pub name: String,
    pub occupation: String,
    pub context: String,
    pub starting_resources: Resources,
    #[serde(default)]
    pub starting_metrics: Metrics,
    #[serde(default)]
    pub modifiers: HashMap<String, f64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Resources {
    pub money: i32,
    pub energy: i32,
    pub influence: i32,
    pub days_remaining: i32,
}

impl Resources {
    pub fn can_afford(&self, cost: &ResourceCost) -> bool {
        self.money >= cost.money
            && self.energy >= cost.energy
            && self.influence >= cost.influence
            && self.days_remaining > cost.days
    }

    pub fn apply_cost(&mut self, cost: &ResourceCost) {
        self.money -= cost.money;
        self.energy -= cost.energy;
        self.influence -= cost.influence;
        self.days_remaining -= cost.days;
    }

    pub fn apply_delta(&mut self, delta: &ResourceDelta) {
        self.money += delta.money;
        self.energy += delta.energy;
        self.influence += delta.influence;
        self.days_remaining += delta.days_remaining;
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct ResourceCost {
    #[serde(default)]
    pub money: i32,
    #[serde(default)]
    pub energy: i32,
    #[serde(default)]
    pub influence: i32,
    #[serde(default)]
    pub days: i32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct ResourceDelta {
    #[serde(default)]
    pub money: i32,
    #[serde(default)]
    pub energy: i32,
    #[serde(default)]
    pub influence: i32,
    #[serde(default)]
    pub days_remaining: i32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Metrics {
    #[serde(default)]
    pub progress: i32,
    #[serde(default)]
    pub documentation: i32,
    #[serde(default)]
    pub community_support: i32,
    #[serde(default)]
    pub public_attention: i32,
    #[serde(default = "default_integrity")]
    pub integrity: i32,
}

fn default_integrity() -> i32 {
    100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDefinition {
    pub id: String,
    pub title: String,
    pub description: String,
    pub cost: ResourceCost,
    #[serde(default)]
    pub guaranteed_effect: StateDelta,
    #[serde(default)]
    pub requirements: Vec<Requirement>,
    pub outcomes: Vec<OutcomeDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StateDelta {
    #[serde(default)]
    pub resources: ResourceDelta,
    #[serde(default)]
    pub progress: i32,
    #[serde(default)]
    pub documentation: i32,
    #[serde(default)]
    pub community_support: i32,
    #[serde(default)]
    pub public_attention: i32,
    #[serde(default)]
    pub integrity: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    pub metric: String,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeDefinition {
    pub id: String,
    pub message: String,
    pub base_weight: f64,
    #[serde(default)]
    pub progress_min: i32,
    #[serde(default)]
    pub progress_max: i32,
    #[serde(default)]
    pub effect: StateDelta,
    #[serde(default)]
    pub conditions: Vec<WeightCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightCondition {
    pub metric: String,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    pub multiplier: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiddenContext {
    pub departmental_backlog: i32,
    pub officer_integrity: i32,
    pub election_pressure: i32,
    pub corruption_pressure: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Won,
    Lost,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSession {
    pub id: String,
    pub game_pack_id: String,
    pub citizen_id: String,
    pub citizen_name: String,
    pub citizen_context: String,
    pub mission_title: String,
    pub objective: String,
    pub current_status: String,
    pub resources: Resources,
    pub metrics: Metrics,
    pub hidden: HiddenContext,
    pub status: SessionStatus,
    pub turn: u32,
    pub seed: u64,
    pub events: Vec<GameEvent>,
    #[serde(default)]
    pub action_results: HashMap<String, ActionResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEvent {
    pub turn: u32,
    pub action_id: String,
    pub action_title: String,
    pub outcome_id: String,
    pub message: String,
    pub progress_change: i32,
    pub resources_after: Resources,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicScenario {
    pub id: String,
    pub title: String,
    pub description: String,
    pub version: String,
    pub mission: MissionDefinition,
    pub citizens: Vec<CitizenProfile>,
}

impl From<&GamePack> for PublicScenario {
    fn from(value: &GamePack) -> Self {
        Self {
            id: value.id.clone(),
            title: value.title.clone(),
            description: value.description.clone(),
            version: value.version.clone(),
            mission: value.mission.clone(),
            citizens: value.citizens.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableAction {
    pub id: String,
    pub title: String,
    pub description: String,
    pub cost: ResourceCost,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicGameState {
    pub id: String,
    pub citizen_id: String,
    pub citizen_name: String,
    pub citizen_context: String,
    pub mission_title: String,
    pub objective: String,
    pub current_status: String,
    pub resources: Resources,
    pub metrics: Metrics,
    pub status: SessionStatus,
    pub turn: u32,
    pub events: Vec<GameEvent>,
    pub available_actions: Vec<AvailableAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub citizen_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRequest {
    pub action_id: String,
    pub client_action_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResponse {
    pub outcome_id: String,
    pub message: String,
    pub progress_change: i32,
    pub state: PublicGameState,
}
