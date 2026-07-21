use crate::model::*;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;

const LEGACY_METRICS: &[&str] = &[
    "money",
    "energy",
    "influence",
    "days_remaining",
    "progress",
    "documentation",
    "community_support",
    "public_attention",
    "integrity",
    "departmental_backlog",
    "officer_integrity",
    "election_pressure",
    "corruption_pressure",
];

#[derive(Debug, Clone)]
pub struct PackRegistry {
    packs: HashMap<String, Arc<GamePack>>,
    default_id: String,
}

impl PackRegistry {
    pub fn load(
        packs_path: Option<&Path>,
        single_pack_path: &Path,
        requested_default_id: Option<&str>,
    ) -> Result<Self, PackLoadError> {
        let paths = if let Some(directory) = packs_path {
            collect_pack_paths(directory)?
        } else {
            vec![single_pack_path.to_path_buf()]
        };
        if paths.is_empty() {
            return Err(PackLoadError::NoPacks);
        }

        let mut packs = HashMap::new();
        for path in paths {
            let contents = fs::read_to_string(&path).map_err(|source| PackLoadError::Read {
                path: path.clone(),
                source,
            })?;
            let pack = parse_pack(&path, &contents)?;
            validate_pack(&pack).map_err(|errors| PackLoadError::Validation {
                path: path.clone(),
                errors,
            })?;
            let id = pack.id.clone();
            if packs.insert(id.clone(), Arc::new(pack)).is_some() {
                return Err(PackLoadError::DuplicatePack(id));
            }
        }

        let default_id = requested_default_id.map(str::to_owned).unwrap_or_else(|| {
            if packs.contains_key("civic-drainage-v1") {
                "civic-drainage-v1".to_string()
            } else {
                let mut ids: Vec<_> = packs.keys().cloned().collect();
                ids.sort();
                ids[0].clone()
            }
        });
        if !packs.contains_key(&default_id) {
            return Err(PackLoadError::DefaultNotFound(default_id));
        }
        Ok(Self { packs, default_id })
    }

    pub fn get(&self, id: &str) -> Option<Arc<GamePack>> {
        self.packs.get(id).cloned()
    }

    pub fn default_pack(&self) -> Arc<GamePack> {
        self.packs[&self.default_id].clone()
    }

    pub fn default_id(&self) -> &str {
        &self.default_id
    }

    pub fn summaries(&self) -> Vec<ScenarioSummary> {
        let mut summaries: Vec<_> = self
            .packs
            .values()
            .map(|pack| ScenarioSummary::from(pack.as_ref()))
            .collect();
        summaries.sort_by(|left, right| left.title.cmp(&right.title));
        summaries
    }

    pub fn len(&self) -> usize {
        self.packs.len()
    }

    pub fn insert(&mut self, pack: GamePack) -> Result<(), Vec<String>> {
        validate_pack(&pack)?;
        if let Some(existing) = self.packs.get(&pack.id) {
            if serde_json::to_value(existing.as_ref()).ok() == serde_json::to_value(&pack).ok() {
                return Ok(());
            }
            return Err(vec![format!(
                "game-pack id '{}' is already registered with different content",
                pack.id
            )]);
        }
        self.packs.insert(pack.id.clone(), Arc::new(pack));
        Ok(())
    }

    pub fn all(&self) -> Vec<Arc<GamePack>> {
        self.packs.values().cloned().collect()
    }
}

fn parse_pack(path: &Path, contents: &str) -> Result<GamePack, PackLoadError> {
    let extension = path.extension().and_then(|value| value.to_str());
    match extension {
        Some("yaml" | "yml") => {
            serde_yaml::from_str(contents).map_err(|source| PackLoadError::Parse {
                path: path.to_path_buf(),
                details: source.to_string(),
            })
        }
        _ => serde_json::from_str(contents).map_err(|source| PackLoadError::Parse {
            path: path.to_path_buf(),
            details: source.to_string(),
        }),
    }
}

fn collect_pack_paths(root: &Path) -> Result<Vec<PathBuf>, PackLoadError> {
    fn visit(path: &Path, output: &mut Vec<PathBuf>) -> Result<(), PackLoadError> {
        let entries = fs::read_dir(path).map_err(|source| PackLoadError::ReadDirectory {
            path: path.to_path_buf(),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| PackLoadError::ReadDirectory {
                path: path.to_path_buf(),
                source,
            })?;
            let child = entry.path();
            if child.is_dir() {
                visit(&child, output)?;
            } else if child.file_name().is_some_and(|name| {
                name == "game.json" || name == "game.yaml" || name == "game.yml"
            }) {
                output.push(child);
            }
        }
        Ok(())
    }

    let mut paths = Vec::new();
    visit(root, &mut paths)?;
    paths.sort();
    Ok(paths)
}

#[derive(Debug, Error)]
pub enum PackLoadError {
    #[error("no game packs were found")]
    NoPacks,
    #[error("could not read game-pack directory {path}: {source}")]
    ReadDirectory {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not read game pack {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not parse game pack {path}: {details}")]
    Parse { path: PathBuf, details: String },
    #[error("game pack {path} failed validation: {errors:?}")]
    Validation { path: PathBuf, errors: Vec<String> },
    #[error("duplicate game-pack id: {0}")]
    DuplicatePack(String),
    #[error("default game-pack id was not found: {0}")]
    DefaultNotFound(String),
}

pub fn validate_pack(pack: &GamePack) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if pack.id.trim().is_empty() {
        errors.push("pack id must not be empty".to_string());
    }
    if pack.citizens.is_empty() {
        errors.push("at least one player profile is required".to_string());
    }
    if pack.actions.is_empty() {
        errors.push("at least one action is required".to_string());
    }
    if pack.schema_version > 2 {
        errors.push(format!(
            "unsupported schema_version {}; maximum is 2",
            pack.schema_version
        ));
    }

    unique_ids(
        "player profile",
        pack.citizens.iter().map(|value| value.id.as_str()),
        &mut errors,
    );
    unique_ids(
        "action",
        pack.actions.iter().map(|value| value.id.as_str()),
        &mut errors,
    );
    unique_ids(
        "value",
        pack.value_definitions.iter().map(|value| value.id.as_str()),
        &mut errors,
    );
    unique_ids(
        "hidden variable",
        pack.hidden_variable_definitions
            .iter()
            .map(|value| value.id.as_str()),
        &mut errors,
    );
    unique_ids(
        "institution",
        pack.institutions.iter().map(|value| value.id.as_str()),
        &mut errors,
    );
    unique_ids(
        "stakeholder",
        pack.stakeholders.iter().map(|value| value.id.as_str()),
        &mut errors,
    );
    unique_ids(
        "barrier",
        pack.barriers.iter().map(|value| value.id.as_str()),
        &mut errors,
    );
    unique_ids(
        "random event",
        pack.random_events.iter().map(|value| value.id.as_str()),
        &mut errors,
    );
    unique_ids(
        "ending",
        pack.endings.iter().map(|value| value.id.as_str()),
        &mut errors,
    );
    unique_ids(
        "visual location",
        pack.visual_theme
            .locations
            .iter()
            .map(|value| value.id.as_str()),
        &mut errors,
    );

    let institution_ids: HashSet<_> = pack
        .institutions
        .iter()
        .map(|value| value.id.as_str())
        .collect();
    let location_ids: HashSet<_> = pack
        .visual_theme
        .locations
        .iter()
        .map(|value| value.id.as_str())
        .collect();
    for stakeholder in &pack.stakeholders {
        if let Some(id) = stakeholder.institution_id.as_deref() {
            if !institution_ids.contains(id) {
                errors.push(format!(
                    "stakeholder '{}' references unknown institution '{}'",
                    stakeholder.id, id
                ));
            }
        }
    }
    for location in &pack.visual_theme.locations {
        if let Some(id) = location.institution_id.as_deref() {
            if !institution_ids.contains(id) {
                errors.push(format!(
                    "visual location '{}' references unknown institution '{}'",
                    location.id, id
                ));
            }
        }
    }

    let mut known_metrics: HashSet<String> = LEGACY_METRICS
        .iter()
        .map(|value| (*value).to_string())
        .collect();
    known_metrics.extend(pack.value_definitions.iter().map(|value| value.id.clone()));
    known_metrics.extend(
        pack.hidden_variable_definitions
            .iter()
            .map(|value| value.id.clone()),
    );
    for citizen in &pack.citizens {
        known_metrics.extend(citizen.starting_values.keys().cloned());
        known_metrics.extend(citizen.skills.keys().cloned());
        known_metrics.extend(citizen.relationships.keys().cloned());
        if let Some(start) = citizen
            .visual
            .as_ref()
            .and_then(|visual| visual.start_location_id.as_deref())
        {
            if !location_ids.contains(start) {
                errors.push(format!(
                    "player profile '{}' references unknown start location '{}'",
                    citizen.id, start
                ));
            }
        }
    }
    for action in &pack.actions {
        if action.max_uses == Some(0) {
            errors.push(format!(
                "action '{}' max_uses must be at least one",
                action.id
            ));
        }
        if action.max_uses.is_some_and(|limit| limit > 1) {
            errors.push(format!(
                "action '{}' cannot set max_uses above one; model changed circumstances as a distinct state-gated action",
                action.id
            ));
        }
        known_metrics.extend(action.cost.values.keys().cloned());
        collect_delta_ids(&action.guaranteed_effect, &mut known_metrics);
        for outcome in &action.outcomes {
            collect_delta_ids(&outcome.effect, &mut known_metrics);
        }
    }
    for event in &pack.random_events {
        collect_delta_ids(&event.effect, &mut known_metrics);
    }

    for (kind, transfers) in [
        ("campaign import", &pack.campaign.imports),
        ("campaign export", &pack.campaign.exports),
    ] {
        for transfer in transfers {
            let session_metric = if kind == "campaign import" {
                &transfer.target_id
            } else {
                &transfer.source_id
            };
            if !known_metrics.contains(session_metric) {
                errors.push(format!(
                    "{kind} references unknown session metric '{session_metric}'"
                ));
            }
            if !transfer.multiplier.is_finite() {
                errors.push(format!("{kind} has a non-finite multiplier"));
            }
            if transfer
                .min
                .zip(transfer.max)
                .is_some_and(|(min, max)| min > max)
            {
                errors.push(format!("{kind} has min greater than max"));
            }
        }
    }

    for definition in &pack.value_definitions {
        if definition.min > definition.max {
            errors.push(format!(
                "value '{}' has min greater than max",
                definition.id
            ));
        }
    }
    for definition in &pack.hidden_variable_definitions {
        if definition.min > definition.max {
            errors.push(format!(
                "hidden variable '{}' has min greater than max",
                definition.id
            ));
        }
    }

    for action in &pack.actions {
        if let Some(location_id) = action.location_id.as_deref() {
            if !location_ids.is_empty() && !location_ids.contains(location_id) {
                errors.push(format!(
                    "action '{}' references unknown visual location '{}'",
                    action.id, location_id
                ));
            }
        }
        if action.outcomes.is_empty() {
            errors.push(format!("action '{}' has no outcomes", action.id));
        }
        unique_ids(
            &format!("outcome in action '{}'", action.id),
            action.outcomes.iter().map(|value| value.id.as_str()),
            &mut errors,
        );
        validate_requirements(
            &format!("action '{}'", action.id),
            &action.requirements,
            &known_metrics,
            &mut errors,
        );
        for outcome in &action.outcomes {
            if !outcome.base_weight.is_finite() || outcome.base_weight <= 0.0 {
                errors.push(format!(
                    "outcome '{}.{}' must have a finite positive base_weight",
                    action.id, outcome.id
                ));
            }
            if outcome.progress_min > outcome.progress_max {
                errors.push(format!(
                    "outcome '{}.{}' has progress_min greater than progress_max",
                    action.id, outcome.id
                ));
            }
            for condition in &outcome.conditions {
                if !known_metrics.contains(&condition.metric) {
                    errors.push(format!(
                        "outcome '{}.{}' references unknown metric '{}'",
                        action.id, outcome.id, condition.metric
                    ));
                }
                if !condition.multiplier.is_finite() || condition.multiplier < 0.0 {
                    errors.push(format!(
                        "outcome '{}.{}' has an invalid multiplier",
                        action.id, outcome.id
                    ));
                }
            }
            validate_visual_event(
                &format!("outcome '{}.{}'", action.id, outcome.id),
                outcome.visual_event.as_ref(),
                &location_ids,
                &mut errors,
            );
        }
    }

    for event in &pack.random_events {
        if !(0.0..=1.0).contains(&event.chance_per_turn) || !event.chance_per_turn.is_finite() {
            errors.push(format!(
                "random event '{}' chance_per_turn must be between 0 and 1",
                event.id
            ));
        }
        validate_requirements(
            &format!("random event '{}'", event.id),
            &event.conditions,
            &known_metrics,
            &mut errors,
        );
        validate_visual_event(
            &format!("random event '{}'", event.id),
            event.visual_event.as_ref(),
            &location_ids,
            &mut errors,
        );
    }
    for ending in &pack.endings {
        if ending.status == SessionStatus::Active {
            errors.push(format!("ending '{}' cannot have active status", ending.id));
        }
        if ending.conditions.is_empty() {
            errors.push(format!("ending '{}' must have conditions", ending.id));
        }
        validate_requirements(
            &format!("ending '{}'", ending.id),
            &ending.conditions,
            &known_metrics,
            &mut errors,
        );
        let legacy_generated_pack = pack.generated.is_some() && pack.version.starts_with("1.0.0+");
        if pack.schema_version >= 2 && !legacy_generated_pack && ending.status == SessionStatus::Won
        {
            let factors: HashSet<_> = ending
                .conditions
                .iter()
                .filter(|condition| condition.metric != "progress")
                .map(|condition| condition.metric.as_str())
                .collect();
            if factors.len() < 2 {
                errors.push(format!(
                    "winning ending '{}' must require at least two non-progress factors",
                    ending.id
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn unique_ids<'a>(kind: &str, ids: impl Iterator<Item = &'a str>, errors: &mut Vec<String>) {
    let mut seen = HashSet::new();
    for id in ids {
        if id.trim().is_empty() {
            errors.push(format!("{kind} id must not be empty"));
        } else if !seen.insert(id) {
            errors.push(format!("duplicate {kind} id '{id}'"));
        }
    }
}

fn collect_delta_ids(delta: &StateDelta, output: &mut HashSet<String>) {
    output.extend(delta.values.keys().cloned());
    output.extend(delta.consequences.keys().cloned());
}

fn validate_requirements(
    owner: &str,
    requirements: &[Requirement],
    known_metrics: &HashSet<String>,
    errors: &mut Vec<String>,
) {
    for requirement in requirements {
        if !known_metrics.contains(&requirement.metric) {
            errors.push(format!(
                "{owner} references unknown metric '{}'",
                requirement.metric
            ));
        }
        if requirement.min.is_some_and(|value| !value.is_finite())
            || requirement.max.is_some_and(|value| !value.is_finite())
        {
            errors.push(format!("{owner} has a non-finite requirement"));
        }
        if requirement
            .min
            .zip(requirement.max)
            .is_some_and(|(min, max)| min > max)
        {
            errors.push(format!(
                "{owner} has a requirement with min greater than max"
            ));
        }
    }
}

fn validate_visual_event(
    owner: &str,
    visual_event: Option<&VisualEvent>,
    location_ids: &HashSet<&str>,
    errors: &mut Vec<String>,
) {
    if let Some(location_id) = visual_event.and_then(|event| event.focus_location_id.as_deref()) {
        if !location_ids.contains(location_id) {
            errors.push(format!(
                "{owner} references unknown visual location '{location_id}'"
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_drainage_pack_is_valid() {
        let pack: GamePack =
            serde_json::from_str(include_str!("../game-packs/drainage/game.json")).unwrap();
        assert_eq!(validate_pack(&pack), Ok(()));
    }

    #[test]
    fn validator_rejects_unknown_metrics() {
        let mut pack: GamePack =
            serde_json::from_str(include_str!("../game-packs/drainage/game.json")).unwrap();
        pack.actions[0].requirements.push(Requirement {
            metric: "not_a_real_metric".into(),
            min: Some(1.0),
            max: None,
            message: "invalid".into(),
        });
        let errors = validate_pack(&pack).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.contains("not_a_real_metric")));
    }

    #[test]
    fn validator_rejects_progress_only_schema_v2_victory() {
        let mut pack: GamePack =
            serde_json::from_str(include_str!("../game-packs/drainage/game.json")).unwrap();
        let winning = pack
            .endings
            .iter_mut()
            .find(|ending| ending.status == SessionStatus::Won)
            .unwrap();
        winning.conditions = vec![Requirement {
            metric: "progress".into(),
            min: Some(100.0),
            max: None,
            message: String::new(),
        }];
        let errors = validate_pack(&pack).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.contains("at least two non-progress factors")));
    }

    #[test]
    fn validator_rejects_repeatable_actions() {
        let mut pack: GamePack =
            serde_json::from_str(include_str!("../game-packs/drainage/game.json")).unwrap();
        pack.actions[0].max_uses = Some(2);
        let errors = validate_pack(&pack).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.contains("model changed circumstances as a distinct")));
    }

    #[test]
    fn starter_library_loads_all_three_valid_packs() {
        let registry = PackRegistry::load(
            Some(Path::new("game-packs")),
            Path::new("game-packs/drainage/game.json"),
            Some("civic-drainage-v1"),
        )
        .unwrap();

        assert_eq!(registry.len(), 3);
        assert!(registry.get("civic-drainage-v1").is_some());
        assert!(registry.get("examination-scandal-v1").is_some());
        assert!(registry.get("factory-ground-v1").is_some());
    }

    #[test]
    fn yaml_pack_loads_through_the_same_registry() {
        let pack: GamePack =
            serde_json::from_str(include_str!("../game-packs/drainage/game.json")).unwrap();
        let root = std::env::temp_dir().join(format!(
            "civic-sim-yaml-pack-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let pack_directory = root.join("drainage");
        std::fs::create_dir_all(&pack_directory).unwrap();
        std::fs::write(
            pack_directory.join("game.yaml"),
            serde_yaml::to_string(&pack).unwrap(),
        )
        .unwrap();
        let registry = PackRegistry::load(
            Some(&root),
            Path::new("unused.json"),
            Some("civic-drainage-v1"),
        )
        .unwrap();
        assert_eq!(registry.len(), 1);
        assert!(registry.get("civic-drainage-v1").is_some());
        std::fs::remove_dir_all(root).unwrap();
    }
}
