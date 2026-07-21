use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

pub type StateValues = BTreeMap<String, i32>;

fn default_schema_version() -> u32 {
    1
}

fn default_integrity() -> i32 {
    100
}

fn default_event_chance() -> f64 {
    0.0
}

fn default_once() -> bool {
    true
}

fn default_minimum() -> i32 {
    0
}

fn default_maximum() -> i32 {
    100
}

fn default_legacy_game_pack_id() -> String {
    "civic-drainage-v1".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePack {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub description: String,
    pub version: String,
    #[serde(default)]
    pub generated: Option<GeneratedScenarioMetadata>,
    #[serde(default)]
    pub campaign: CampaignRules,
    #[serde(default)]
    pub environment: EnvironmentDefinition,
    pub mission: MissionDefinition,
    #[serde(default)]
    pub value_definitions: Vec<ValueDefinition>,
    #[serde(default)]
    pub hidden_variable_definitions: Vec<HiddenVariableDefinition>,
    #[serde(default)]
    pub institutions: Vec<InstitutionDefinition>,
    #[serde(default)]
    pub stakeholders: Vec<StakeholderDefinition>,
    #[serde(default)]
    pub barriers: Vec<ProceduralBarrierDefinition>,
    #[serde(default)]
    pub random_events: Vec<RandomEventDefinition>,
    #[serde(default)]
    pub endings: Vec<EndingDefinition>,
    #[serde(default)]
    pub visual_theme: VisualTheme,
    pub citizens: Vec<CitizenProfile>,
    pub actions: Vec<ActionDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CampaignRules {
    #[serde(default)]
    pub imports: Vec<CampaignTransfer>,
    #[serde(default)]
    pub exports: Vec<CampaignTransfer>,
    #[serde(default)]
    pub won_effect: StateValues,
    #[serde(default)]
    pub lost_effect: StateValues,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignTransfer {
    pub source_id: String,
    pub target_id: String,
    #[serde(default = "default_transfer_multiplier")]
    pub multiplier: f64,
    #[serde(default)]
    pub source_offset: i32,
    #[serde(default)]
    pub min: Option<i32>,
    #[serde(default)]
    pub max: Option<i32>,
}

fn default_transfer_multiplier() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GeneratedScenarioMetadata {
    pub seed: u64,
    pub template_pack_id: String,
    pub difficulty: String,
    #[serde(default)]
    pub selections: BTreeMap<String, String>,
    #[serde(default)]
    pub modifiers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnvironmentDefinition {
    #[serde(default)]
    pub world_region: String,
    #[serde(default)]
    pub government_level: String,
    #[serde(default)]
    pub political_system: String,
    #[serde(default)]
    pub administrative_capacity: String,
    #[serde(default)]
    pub corruption_structure: String,
    #[serde(default)]
    pub rule_of_law: String,
    #[serde(default)]
    pub media_environment: String,
    #[serde(default)]
    pub civil_society_strength: String,
    #[serde(default)]
    pub economic_condition: String,
    #[serde(default)]
    pub inequality_level: String,
    #[serde(default)]
    pub government_level_scope: String,
    #[serde(default)]
    pub modifiers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionDefinition {
    pub title: String,
    pub objective: String,
    pub starting_status: String,
    #[serde(default = "default_win_progress")]
    pub win_progress: i32,
    #[serde(default)]
    pub objective_type: String,
    #[serde(default)]
    pub time_horizon: String,
}

fn default_win_progress() -> i32 {
    100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueDefinition {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub group: ValueGroup,
    #[serde(default = "default_minimum")]
    pub min: i32,
    #[serde(default = "default_maximum")]
    pub max: i32,
    #[serde(default)]
    pub format: ValueFormat,
    #[serde(default)]
    pub hidden_from_hud: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValueGroup {
    Resource,
    #[default]
    Metric,
    Relationship,
    Skill,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValueFormat {
    #[default]
    Number,
    Percent,
    Money,
    Days,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiddenVariableDefinition {
    pub id: String,
    pub min: i32,
    pub max: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstitutionDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub institution_type: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeholderDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub archetype: String,
    #[serde(default)]
    pub institution_id: Option<String>,
    #[serde(default)]
    pub interests: Vec<String>,
    #[serde(default)]
    pub fears: Vec<String>,
    #[serde(default)]
    pub public_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProceduralBarrierDefinition {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub barrier_type: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VisualTheme {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub layout: String,
    #[serde(default)]
    pub primary_color: String,
    #[serde(default)]
    pub accent_color: String,
    #[serde(default)]
    pub background_color: String,
    #[serde(default)]
    pub map_asset: Option<String>,
    #[serde(default)]
    pub locations: Vec<VisualLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualLocation {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub institution_id: Option<String>,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitizenProfile {
    pub id: String,
    pub name: String,
    pub occupation: String,
    pub context: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub starting_resources: Resources,
    #[serde(default)]
    pub starting_metrics: Metrics,
    #[serde(default)]
    pub starting_values: StateValues,
    #[serde(default)]
    pub skills: StateValues,
    #[serde(default)]
    pub relationships: StateValues,
    #[serde(default)]
    pub modifiers: HashMap<String, f64>,
    #[serde(default)]
    pub visual: Option<PlayerVisual>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerVisual {
    #[serde(default)]
    pub sprite: Option<String>,
    #[serde(default)]
    pub start_location_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Resources {
    #[serde(default)]
    pub money: i32,
    #[serde(default)]
    pub energy: i32,
    #[serde(default)]
    pub influence: i32,
    #[serde(default)]
    pub days_remaining: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceCost {
    #[serde(default)]
    pub money: i32,
    #[serde(default)]
    pub energy: i32,
    #[serde(default)]
    pub influence: i32,
    #[serde(default)]
    pub days: i32,
    #[serde(default)]
    pub values: StateValues,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDefinition {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub action_type: String,
    #[serde(default)]
    pub location_id: Option<String>,
    #[serde(default)]
    pub ethical_tags: Vec<String>,
    #[serde(default)]
    pub cost: ResourceCost,
    #[serde(default)]
    pub guaranteed_effect: StateDelta,
    #[serde(default)]
    pub requirements: Vec<Requirement>,
    #[serde(default)]
    pub max_uses: Option<u32>,
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
    #[serde(default)]
    pub values: StateValues,
    #[serde(default)]
    pub consequences: StateValues,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    pub metric: String,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
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
    #[serde(default)]
    pub visual_event: Option<VisualEvent>,
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
pub struct RandomEventDefinition {
    pub id: String,
    pub title: String,
    pub message: String,
    #[serde(default = "default_event_chance")]
    pub chance_per_turn: f64,
    #[serde(default = "default_once")]
    pub once: bool,
    #[serde(default)]
    pub conditions: Vec<Requirement>,
    #[serde(default)]
    pub effect: StateDelta,
    #[serde(default)]
    pub visual_event: Option<VisualEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndingDefinition {
    pub id: String,
    pub title: String,
    pub message: String,
    pub status: SessionStatus,
    #[serde(default)]
    pub conditions: Vec<Requirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualEvent {
    #[serde(default)]
    pub focus_location_id: Option<String>,
    #[serde(default)]
    pub animation: String,
    #[serde(default)]
    pub effect: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HiddenContext {
    #[serde(default)]
    pub departmental_backlog: i32,
    #[serde(default)]
    pub officer_integrity: i32,
    #[serde(default)]
    pub election_pressure: i32,
    #[serde(default)]
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
    #[serde(default = "default_legacy_game_pack_id")]
    pub game_pack_id: String,
    #[serde(default)]
    pub game_pack_version: String,
    #[serde(default)]
    pub campaign_id: Option<String>,
    #[serde(default)]
    pub campaign_attempt: u32,
    pub citizen_id: String,
    pub citizen_name: String,
    pub citizen_context: String,
    pub mission_title: String,
    pub objective: String,
    pub current_status: String,
    #[serde(default)]
    pub resources: Resources,
    #[serde(default)]
    pub metrics: Metrics,
    #[serde(default)]
    pub values: StateValues,
    #[serde(default)]
    pub hidden: HiddenContext,
    #[serde(default)]
    pub hidden_values: StateValues,
    #[serde(default)]
    pub persistent_consequences: StateValues,
    #[serde(default)]
    pub triggered_random_events: HashSet<String>,
    #[serde(default)]
    pub player_modifiers: HashMap<String, f64>,
    pub status: SessionStatus,
    pub turn: u32,
    pub seed: u64,
    pub events: Vec<GameEvent>,
    #[serde(default)]
    pub action_results: HashMap<String, ActionResponse>,
    #[serde(default)]
    pub ending_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEvent {
    pub turn: u32,
    #[serde(default = "default_action_event_kind")]
    pub kind: String,
    pub action_id: String,
    pub action_title: String,
    pub outcome_id: String,
    pub message: String,
    pub progress_change: i32,
    pub resources_after: Resources,
    #[serde(default)]
    pub value_changes: StateValues,
    #[serde(default)]
    pub visual_event: Option<VisualEvent>,
}

fn default_action_event_kind() -> String {
    "action".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioSummary {
    pub id: String,
    pub title: String,
    pub description: String,
    pub version: String,
    pub objective_type: String,
    pub world_region: String,
    pub role_count: usize,
    pub visual_theme: VisualTheme,
    pub generated: bool,
}

impl From<&GamePack> for ScenarioSummary {
    fn from(value: &GamePack) -> Self {
        Self {
            id: value.id.clone(),
            title: value.title.clone(),
            description: value.description.clone(),
            version: value.version.clone(),
            objective_type: value.mission.objective_type.clone(),
            world_region: value.environment.world_region.clone(),
            role_count: value.citizens.len(),
            visual_theme: value.visual_theme.clone(),
            generated: value.generated.is_some(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicScenario {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub description: String,
    pub version: String,
    pub generated: Option<GeneratedScenarioMetadata>,
    pub environment: EnvironmentDefinition,
    pub mission: MissionDefinition,
    pub value_definitions: Vec<ValueDefinition>,
    pub institutions: Vec<InstitutionDefinition>,
    pub stakeholders: Vec<StakeholderDefinition>,
    pub barriers: Vec<ProceduralBarrierDefinition>,
    pub visual_theme: VisualTheme,
    pub citizens: Vec<CitizenProfile>,
}

impl From<&GamePack> for PublicScenario {
    fn from(value: &GamePack) -> Self {
        Self {
            schema_version: value.schema_version,
            id: value.id.clone(),
            title: value.title.clone(),
            description: value.description.clone(),
            version: value.version.clone(),
            generated: value.generated.clone(),
            environment: value.environment.clone(),
            mission: value.mission.clone(),
            value_definitions: value.value_definitions.clone(),
            institutions: value.institutions.clone(),
            stakeholders: value.stakeholders.clone(),
            barriers: value.barriers.clone(),
            visual_theme: value.visual_theme.clone(),
            citizens: value.citizens.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableAction {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub action_type: String,
    #[serde(default)]
    pub location_id: Option<String>,
    pub cost: ResourceCost,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicIndicator {
    pub id: String,
    pub label: String,
    pub description: String,
    pub group: ValueGroup,
    pub value: i32,
    pub min: i32,
    pub max: i32,
    pub format: ValueFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicGameState {
    pub id: String,
    #[serde(default = "default_legacy_game_pack_id")]
    pub game_pack_id: String,
    #[serde(default)]
    pub game_pack_version: String,
    #[serde(default)]
    pub campaign_id: Option<String>,
    pub citizen_id: String,
    pub citizen_name: String,
    pub citizen_context: String,
    pub mission_title: String,
    pub objective: String,
    pub current_status: String,
    pub resources: Resources,
    pub metrics: Metrics,
    #[serde(default)]
    pub values: StateValues,
    #[serde(default)]
    pub indicators: Vec<PublicIndicator>,
    #[serde(default)]
    pub persistent_consequences: StateValues,
    pub status: SessionStatus,
    #[serde(default)]
    pub ending_id: Option<String>,
    pub turn: u32,
    pub events: Vec<GameEvent>,
    pub available_actions: Vec<AvailableAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    #[serde(default)]
    pub scenario_id: Option<String>,
    #[serde(default)]
    pub profile_id: Option<String>,
    #[serde(default)]
    pub citizen_id: String,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub campaign_id: Option<String>,
}

impl CreateSessionRequest {
    pub fn requested_profile_id(&self) -> &str {
        self.profile_id.as_deref().unwrap_or(&self.citizen_id)
    }
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
    #[serde(default)]
    pub value_changes: StateValues,
    #[serde(default)]
    pub visual_event: Option<VisualEvent>,
    pub state: PublicGameState,
}
