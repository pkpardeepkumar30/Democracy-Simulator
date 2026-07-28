use crate::{
    game_pack::validate_pack,
    model::{GamePack, GeneratedScenarioMetadata, Metrics, ResourceCost, Resources, StateValues},
};
use rand::{prelude::IndexedRandom, Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;
use tokio::{
    fs,
    sync::{Mutex, RwLock},
};

pub const REQUIRED_CATEGORIES: &[&str] = &[
    "world_region",
    "government_level",
    "political_system",
    "administrative_capacity",
    "corruption_structure",
    "rule_of_law",
    "media_environment",
    "civil_society_strength",
    "economic_condition",
    "inequality_level",
    "player_role",
    "objective_type",
];
pub const OPTIONAL_CATEGORIES: &[&str] = &["city_plan"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbstractionCatalog {
    pub schema_version: u32,
    pub categories: BTreeMap<String, Vec<AbstractionOption>>,
    pub difficulties: Vec<DifficultyDefinition>,
    pub modifiers: Vec<AbstractionOption>,
    #[serde(default)]
    pub constraints: Vec<SelectionConstraint>,
    pub templates: Vec<GenerationTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbstractionOption {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub effects: GenerationEffects,
    #[serde(default)]
    pub palette: Option<ThemePalette>,
    #[serde(default)]
    pub map_asset: Option<String>,
    #[serde(default)]
    pub compatible_world_regions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationEffects {
    #[serde(default = "one")]
    pub starting_resource_multiplier: f64,
    #[serde(default = "one")]
    pub action_cost_multiplier: f64,
    #[serde(default = "one")]
    pub event_chance_multiplier: f64,
    #[serde(default)]
    pub hidden_range_shift: i32,
    #[serde(default)]
    pub starting_value_deltas: StateValues,
}

impl Default for GenerationEffects {
    fn default() -> Self {
        Self {
            starting_resource_multiplier: 1.0,
            action_cost_multiplier: 1.0,
            event_chance_multiplier: 1.0,
            hidden_range_shift: 0,
            starting_value_deltas: StateValues::new(),
        }
    }
}

fn one() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemePalette {
    pub primary_color: String,
    pub accent_color: String,
    pub background_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyDefinition {
    pub id: String,
    pub label: String,
    pub description: String,
    #[serde(default)]
    pub effects: GenerationEffects,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationTemplate {
    pub pack_id: String,
    pub objective_types: Vec<String>,
    #[serde(default)]
    pub constraints: Vec<SelectionConstraint>,
    #[serde(default)]
    pub limits: GenerationLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationLimits {
    #[serde(default = "default_min_resource_multiplier")]
    pub min_starting_resource_multiplier: f64,
    #[serde(default = "default_max_cost_multiplier")]
    pub max_action_cost_multiplier: f64,
    #[serde(default = "default_max_event_multiplier")]
    pub max_event_chance_multiplier: f64,
}

impl Default for GenerationLimits {
    fn default() -> Self {
        Self {
            min_starting_resource_multiplier: default_min_resource_multiplier(),
            max_action_cost_multiplier: default_max_cost_multiplier(),
            max_event_chance_multiplier: default_max_event_multiplier(),
        }
    }
}

fn default_min_resource_multiplier() -> f64 {
    0.5
}

fn default_max_cost_multiplier() -> f64 {
    2.0
}

fn default_max_event_multiplier() -> f64 {
    2.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionConstraint {
    pub id: String,
    pub message: String,
    #[serde(default)]
    pub when: BTreeMap<String, Vec<String>>,
    pub require: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateScenarioRequest {
    #[serde(default)]
    pub seed: Option<u64>,
    #[serde(default)]
    pub selections: BTreeMap<String, String>,
    #[serde(default = "default_difficulty")]
    pub difficulty: String,
    #[serde(default)]
    pub modifiers: Vec<String>,
    #[serde(default = "default_randomize")]
    pub randomize_unspecified: bool,
}

impl Default for GenerateScenarioRequest {
    fn default() -> Self {
        Self {
            seed: None,
            selections: BTreeMap::new(),
            difficulty: default_difficulty(),
            modifiers: Vec::new(),
            randomize_unspecified: true,
        }
    }
}

fn default_difficulty() -> String {
    "standard".to_string()
}

fn default_randomize() -> bool {
    true
}

#[derive(Debug, Error)]
pub enum GenerationError {
    #[error("could not read abstraction catalog {path}: {source}")]
    ReadCatalog {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not parse abstraction catalog {path}: {source}")]
    ParseCatalog {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("abstraction catalog failed validation: {0:?}")]
    InvalidCatalog(Vec<String>),
    #[error("scenario generation request is invalid: {0:?}")]
    InvalidRequest(Vec<String>),
    #[error("no template supports objective type '{0}'")]
    NoTemplate(String),
    #[error("template game pack '{0}' is unavailable")]
    MissingTemplate(String),
    #[error("generated game pack failed validation: {0:?}")]
    InvalidPack(Vec<String>),
}

impl AbstractionCatalog {
    pub fn load(
        path: &Path,
        available_pack_ids: &HashSet<String>,
    ) -> Result<Self, GenerationError> {
        let contents =
            std::fs::read_to_string(path).map_err(|source| GenerationError::ReadCatalog {
                path: path.to_path_buf(),
                source,
            })?;
        let mut catalog: Self =
            serde_json::from_str(&contents).map_err(|source| GenerationError::ParseCatalog {
                path: path.to_path_buf(),
                source,
            })?;
        catalog
            .templates
            .retain(|template| available_pack_ids.contains(&template.pack_id));
        catalog.validate(available_pack_ids)?;
        Ok(catalog)
    }

    pub fn validate(&self, _available_pack_ids: &HashSet<String>) -> Result<(), GenerationError> {
        let mut errors = Vec::new();
        if self.schema_version != 1 {
            errors.push(format!(
                "unsupported schema_version {}",
                self.schema_version
            ));
        }
        for category in REQUIRED_CATEGORIES {
            match self.categories.get(*category) {
                Some(options) if !options.is_empty() => {
                    validate_option_ids(category, options, &mut errors)
                }
                _ => errors.push(format!("category '{category}' must contain options")),
            }
        }
        for category in OPTIONAL_CATEGORIES {
            if let Some(options) = self.categories.get(*category) {
                if options.is_empty() {
                    errors.push(format!("category '{category}' must contain options"));
                } else {
                    validate_option_ids(category, options, &mut errors);
                    if *category == "city_plan" {
                        for option in options {
                            for region in &option.compatible_world_regions {
                                if !self.categories["world_region"]
                                    .iter()
                                    .any(|candidate| &candidate.id == region)
                                {
                                    errors.push(format!(
                                        "city plan '{}' references unknown world region '{}'",
                                        option.id, region
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        validate_ids(
            "difficulty",
            self.difficulties.iter().map(|item| item.id.as_str()),
            &mut errors,
        );
        validate_option_ids("modifier", &self.modifiers, &mut errors);
        if !self.difficulties.iter().any(|value| value.id == "standard") {
            errors.push("difficulty 'standard' is required".to_string());
        }
        let objective_ids: HashSet<_> = self.categories["objective_type"]
            .iter()
            .map(|item| item.id.as_str())
            .collect();
        for template in &self.templates {
            for objective in &template.objective_types {
                if !objective_ids.contains(objective.as_str()) {
                    errors.push(format!(
                        "template '{}' references unknown objective '{objective}'",
                        template.pack_id
                    ));
                }
            }
            validate_constraints(
                &format!("template '{}'", template.pack_id),
                &template.constraints,
                &self.categories,
                &mut errors,
            );
            if !template.limits.min_starting_resource_multiplier.is_finite()
                || template.limits.min_starting_resource_multiplier <= 0.0
                || !template.limits.max_action_cost_multiplier.is_finite()
                || template.limits.max_action_cost_multiplier <= 0.0
                || !template.limits.max_event_chance_multiplier.is_finite()
                || template.limits.max_event_chance_multiplier <= 0.0
            {
                errors.push(format!(
                    "template '{}' has invalid generation limits",
                    template.pack_id
                ));
            }
        }
        validate_constraints("catalog", &self.constraints, &self.categories, &mut errors);
        validate_effects(
            self.categories
                .values()
                .flatten()
                .map(|item| (&item.id, &item.effects))
                .chain(self.modifiers.iter().map(|item| (&item.id, &item.effects)))
                .chain(
                    self.difficulties
                        .iter()
                        .map(|item| (&item.id, &item.effects)),
                ),
            &mut errors,
        );
        if errors.is_empty() {
            Ok(())
        } else {
            Err(GenerationError::InvalidCatalog(errors))
        }
    }
}

fn validate_constraints(
    owner: &str,
    constraints: &[SelectionConstraint],
    categories: &BTreeMap<String, Vec<AbstractionOption>>,
    errors: &mut Vec<String>,
) {
    validate_ids(
        &format!("{owner} constraint"),
        constraints.iter().map(|item| item.id.as_str()),
        errors,
    );
    for constraint in constraints {
        if constraint.require.is_empty() {
            errors.push(format!(
                "{owner} constraint '{}' must require at least one selection",
                constraint.id
            ));
        }
        for (category, ids) in constraint.when.iter().chain(&constraint.require) {
            let Some(options) = categories.get(category) else {
                errors.push(format!(
                    "{owner} constraint '{}' references unknown category '{category}'",
                    constraint.id
                ));
                continue;
            };
            if ids.is_empty() {
                errors.push(format!(
                    "{owner} constraint '{}' has no allowed values for '{category}'",
                    constraint.id
                ));
            }
            for id in ids {
                if !options.iter().any(|option| &option.id == id) {
                    errors.push(format!(
                        "{owner} constraint '{}' references unknown {category} option '{id}'",
                        constraint.id
                    ));
                }
            }
        }
    }
}

fn validate_option_ids(kind: &str, options: &[AbstractionOption], errors: &mut Vec<String>) {
    validate_ids(kind, options.iter().map(|item| item.id.as_str()), errors);
}

fn validate_ids<'a>(kind: &str, ids: impl Iterator<Item = &'a str>, errors: &mut Vec<String>) {
    let mut seen = HashSet::new();
    for id in ids {
        if id.trim().is_empty() || !seen.insert(id) {
            errors.push(format!("{kind} has an empty or duplicate id '{id}'"));
        }
    }
}

fn validate_effects<'a>(
    effects: impl Iterator<Item = (&'a String, &'a GenerationEffects)>,
    errors: &mut Vec<String>,
) {
    for (id, effect) in effects {
        for (name, value) in [
            (
                "starting_resource_multiplier",
                effect.starting_resource_multiplier,
            ),
            ("action_cost_multiplier", effect.action_cost_multiplier),
            ("event_chance_multiplier", effect.event_chance_multiplier),
        ] {
            if !value.is_finite() || value <= 0.0 {
                errors.push(format!("'{id}' has invalid {name}"));
            }
        }
    }
}

pub fn generate_pack<F>(
    catalog: &AbstractionCatalog,
    request: GenerateScenarioRequest,
    mut get_pack: F,
) -> Result<GamePack, GenerationError>
where
    F: FnMut(&str) -> Option<Arc<GamePack>>,
{
    let seed = request.seed.unwrap_or_else(|| rand::rng().random());
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut errors = Vec::new();
    let explicit_categories: HashSet<_> = request.selections.keys().cloned().collect();
    for key in request.selections.keys() {
        if !REQUIRED_CATEGORIES.contains(&key.as_str())
            && !OPTIONAL_CATEGORIES.contains(&key.as_str())
        {
            errors.push(format!("unknown category '{key}'"));
        }
    }
    let mut selected = BTreeMap::new();
    for category in REQUIRED_CATEGORIES {
        let options = &catalog.categories[*category];
        let requested = request.selections.get(*category);
        let option = match requested {
            Some(id) => options.iter().find(|item| &item.id == id).or_else(|| {
                errors.push(format!("unknown {category} option '{id}'"));
                None
            }),
            None if request.randomize_unspecified => options.choose(&mut rng),
            None => {
                errors.push(format!("selection '{category}' is required"));
                None
            }
        };
        if let Some(option) = option {
            selected.insert((*category).to_string(), option.id.clone());
        }
    }
    for category in OPTIONAL_CATEGORIES {
        let Some(options) = catalog.categories.get(*category) else {
            continue;
        };
        let requested = request.selections.get(*category);
        let option = match requested {
            Some(id) => options.iter().find(|item| &item.id == id).or_else(|| {
                errors.push(format!("unknown {category} option '{id}'"));
                None
            }),
            None if request.randomize_unspecified => {
                let region = selected.get("world_region");
                let compatible: Vec<_> = options
                    .iter()
                    .filter(|item| {
                        item.compatible_world_regions.is_empty()
                            || region.is_some_and(|region| {
                                item.compatible_world_regions.contains(region)
                            })
                    })
                    .collect();
                compatible.choose(&mut rng).copied()
            }
            None => None,
        };
        if let Some(option) = option {
            if !option.compatible_world_regions.is_empty()
                && selected
                    .get("world_region")
                    .is_some_and(|region| !option.compatible_world_regions.contains(region))
            {
                if explicit_categories.contains("world_region") {
                    errors.push(format!(
                        "{} is not compatible with the selected world region",
                        option.label
                    ));
                } else if let Some(region) = option.compatible_world_regions.first() {
                    selected.insert("world_region".to_string(), region.clone());
                }
            }
            selected.insert((*category).to_string(), option.id.clone());
        }
    }
    let difficulty = catalog
        .difficulties
        .iter()
        .find(|item| item.id == request.difficulty)
        .or_else(|| {
            errors.push(format!("unknown difficulty '{}'", request.difficulty));
            None
        });
    let mut selected_modifiers = Vec::new();
    let mut modifier_ids = request.modifiers;
    if modifier_ids.is_empty() && request.randomize_unspecified {
        let count = rng.random_range(1..=4).min(catalog.modifiers.len());
        let mut candidates = catalog.modifiers.clone();
        while selected_modifiers.len() < count {
            let index = rng.random_range(0..candidates.len());
            selected_modifiers.push(candidates.swap_remove(index));
        }
    } else {
        let mut seen = HashSet::new();
        modifier_ids.retain(|id| seen.insert(id.clone()));
        if modifier_ids.len() > 4 {
            errors.push("at most four modifiers may be selected".to_string());
        }
        for id in modifier_ids {
            match catalog.modifiers.iter().find(|item| item.id == id) {
                Some(item) => selected_modifiers.push(item.clone()),
                None => errors.push(format!("unknown modifier '{id}'")),
            }
        }
    }
    if !errors.is_empty() {
        return Err(GenerationError::InvalidRequest(errors));
    }

    let objective_id = &selected["objective_type"];
    let candidate_templates: Vec<_> = catalog
        .templates
        .iter()
        .filter(|template| template.objective_types.contains(objective_id))
        .collect();
    let template = candidate_templates
        .choose(&mut rng)
        .ok_or_else(|| GenerationError::NoTemplate(objective_id.clone()))?;
    enforce_constraints(
        catalog,
        template
            .constraints
            .iter()
            .chain(catalog.constraints.iter()),
        &explicit_categories,
        &mut selected,
        &mut rng,
        &mut errors,
    );
    if !errors.is_empty() {
        return Err(GenerationError::InvalidRequest(errors));
    }
    let mut pack = get_pack(&template.pack_id)
        .ok_or_else(|| GenerationError::MissingTemplate(template.pack_id.clone()))?
        .as_ref()
        .clone();
    let template_fingerprint =
        stable_hash(&serde_json::to_vec(&pack).expect("template game pack is serializable"));

    let difficulty = difficulty.expect("validated above");
    let mut combined = difficulty.effects.clone();
    for (category, id) in &selected {
        let option = catalog.categories[category]
            .iter()
            .find(|item| &item.id == id)
            .expect("validated above");
        combine_effects(&mut combined, &option.effects);
    }
    for modifier in &selected_modifiers {
        combine_effects(&mut combined, &modifier.effects);
    }
    combined.starting_resource_multiplier = combined
        .starting_resource_multiplier
        .max(template.limits.min_starting_resource_multiplier);
    combined.action_cost_multiplier = combined
        .action_cost_multiplier
        .min(template.limits.max_action_cost_multiplier);
    combined.event_chance_multiplier = combined
        .event_chance_multiplier
        .min(template.limits.max_event_chance_multiplier);
    apply_effects(&mut pack, &combined);

    apply_environment(&mut pack, catalog, &selected);
    if let Some(region) = selected_option(catalog, &selected, "world_region") {
        if let Some(palette) = &region.palette {
            pack.visual_theme.primary_color = palette.primary_color.clone();
            pack.visual_theme.accent_color = palette.accent_color.clone();
            pack.visual_theme.background_color = palette.background_color.clone();
        }
    }
    let city_plan = selected_option(catalog, &selected, "city_plan");
    pack.visual_theme.map_asset = city_plan.and_then(|option| option.map_asset.clone());
    if let Some(role) = selected_option(catalog, &selected, "player_role") {
        let profile_index = rng.random_range(0..pack.citizens.len());
        let mut profile = pack.citizens[profile_index].clone();
        profile.id = format!("generated_{}", role.id);
        profile.role = role.label.clone();
        profile.occupation = role.label.clone();
        profile.context = format!(
            "You enter this mission as a {}. Your starting resources and civic connections reflect a playable profile for this objective.",
            role.label.to_lowercase()
        );
        pack.citizens = vec![profile];
    }

    let modifiers: Vec<_> = selected_modifiers
        .iter()
        .map(|item| item.id.clone())
        .collect();
    pack.environment.modifiers = selected_modifiers
        .iter()
        .map(|item| item.label.clone())
        .collect();
    let identity = identity_hash(
        seed,
        &template.pack_id,
        &pack.version,
        template_fingerprint,
        &selected,
        &request.difficulty,
        &modifiers,
    );
    let region = selected_option(catalog, &selected, "world_region").expect("validated above");
    pack.id = format!("generated-{}-{identity:016x}", template.pack_id);
    if let Some(city_plan) = city_plan {
        pack.title = format!("{} — {}", pack.title, city_plan.label);
        pack.description = format!(
            "A generated {} scenario in a {} environment, rendered over the street geometry of {}. The civic scenario and institution locations are fictional. {}",
            difficulty.label.to_lowercase(),
            region.label,
            city_plan.label,
            pack.description
        );
    } else {
        pack.title = format!("{} — {}", pack.title, region.label);
        pack.description = format!(
            "A generated {} scenario in a {} environment. {}",
            difficulty.label.to_lowercase(),
            region.label,
            pack.description
        );
    }
    pack.version = format!("{}+generated.{identity:016x}", pack.version);
    pack.generated = Some(GeneratedScenarioMetadata {
        seed,
        template_pack_id: template.pack_id.clone(),
        difficulty: request.difficulty,
        selections: selected,
        modifiers,
    });
    validate_pack(&pack).map_err(GenerationError::InvalidPack)?;
    Ok(pack)
}

fn enforce_constraints<'a>(
    catalog: &AbstractionCatalog,
    constraints: impl Iterator<Item = &'a SelectionConstraint>,
    explicit_categories: &HashSet<String>,
    selected: &mut BTreeMap<String, String>,
    rng: &mut ChaCha8Rng,
    errors: &mut Vec<String>,
) {
    for constraint in constraints {
        let matches = constraint.when.iter().all(|(category, allowed)| {
            selected
                .get(category)
                .is_some_and(|value| allowed.contains(value))
        });
        if !matches {
            continue;
        }
        for (category, allowed) in &constraint.require {
            if selected
                .get(category)
                .is_some_and(|value| allowed.contains(value))
            {
                continue;
            }
            if explicit_categories.contains(category) {
                errors.push(format!("{} ({})", constraint.message, constraint.id));
                continue;
            }
            let candidates: Vec<_> = catalog.categories[category]
                .iter()
                .filter(|option| allowed.contains(&option.id))
                .collect();
            if let Some(replacement) = candidates.choose(rng) {
                selected.insert(category.clone(), replacement.id.clone());
            }
        }
    }
}

fn selected_option<'a>(
    catalog: &'a AbstractionCatalog,
    selected: &BTreeMap<String, String>,
    category: &str,
) -> Option<&'a AbstractionOption> {
    let id = selected.get(category)?;
    catalog
        .categories
        .get(category)?
        .iter()
        .find(|item| &item.id == id)
}

fn apply_environment(
    pack: &mut GamePack,
    catalog: &AbstractionCatalog,
    selected: &BTreeMap<String, String>,
) {
    let label = |category| {
        selected_option(catalog, selected, category)
            .map(|item| item.label.clone())
            .unwrap_or_default()
    };
    pack.environment.world_region = label("world_region");
    pack.environment.government_level = label("government_level");
    pack.environment.political_system = label("political_system");
    pack.environment.administrative_capacity = label("administrative_capacity");
    pack.environment.corruption_structure = label("corruption_structure");
    pack.environment.rule_of_law = label("rule_of_law");
    pack.environment.media_environment = label("media_environment");
    pack.environment.civil_society_strength = label("civil_society_strength");
    pack.environment.economic_condition = label("economic_condition");
    pack.environment.inequality_level = label("inequality_level");
}

fn combine_effects(target: &mut GenerationEffects, next: &GenerationEffects) {
    target.starting_resource_multiplier *= next.starting_resource_multiplier;
    target.action_cost_multiplier *= next.action_cost_multiplier;
    target.event_chance_multiplier *= next.event_chance_multiplier;
    target.hidden_range_shift += next.hidden_range_shift;
    for (id, delta) in &next.starting_value_deltas {
        *target.starting_value_deltas.entry(id.clone()).or_default() += delta;
    }
}

fn apply_effects(pack: &mut GamePack, effects: &GenerationEffects) {
    let resource_value_ids: HashSet<_> = pack
        .value_definitions
        .iter()
        .filter(|definition| definition.group == crate::model::ValueGroup::Resource)
        .map(|definition| definition.id.clone())
        .collect();
    for citizen in &mut pack.citizens {
        scale_resources(
            &mut citizen.starting_resources,
            effects.starting_resource_multiplier,
        );
        for (id, value) in &mut citizen.starting_values {
            if resource_value_ids.contains(id) {
                *value = scale(*value, effects.starting_resource_multiplier);
            }
        }
        for (id, delta) in &effects.starting_value_deltas {
            apply_starting_delta(
                &mut citizen.starting_resources,
                &mut citizen.starting_metrics,
                &mut citizen.starting_values,
                id,
                *delta,
            );
        }
        for definition in &pack.value_definitions {
            clamp_starting_value(
                &mut citizen.starting_resources,
                &mut citizen.starting_metrics,
                &mut citizen.starting_values,
                &definition.id,
                definition.min,
                definition.max,
            );
        }
    }
    for action in &mut pack.actions {
        scale_cost(&mut action.cost, effects.action_cost_multiplier);
    }
    for event in &mut pack.random_events {
        event.chance_per_turn =
            (event.chance_per_turn * effects.event_chance_multiplier).clamp(0.0, 1.0);
    }
    for hidden in &mut pack.hidden_variable_definitions {
        hidden.min += effects.hidden_range_shift;
        hidden.max += effects.hidden_range_shift;
    }
}

fn scale_resources(resources: &mut Resources, multiplier: f64) {
    resources.money = scale(resources.money, multiplier);
    resources.energy = scale(resources.energy, multiplier);
    resources.influence = scale(resources.influence, multiplier);
    resources.days_remaining = scale(resources.days_remaining, multiplier);
}

fn scale_cost(cost: &mut ResourceCost, multiplier: f64) {
    cost.money = scale(cost.money, multiplier);
    cost.energy = scale(cost.energy, multiplier);
    cost.influence = scale(cost.influence, multiplier);
    cost.days = scale(cost.days, multiplier);
    for value in cost.values.values_mut() {
        *value = scale(*value, multiplier);
    }
}

fn scale(value: i32, multiplier: f64) -> i32 {
    if value == 0 {
        0
    } else {
        let scaled = (value as f64 * multiplier).round() as i32;
        if scaled == 0 {
            value.signum()
        } else {
            scaled
        }
    }
}

fn clamp_starting_value(
    resources: &mut Resources,
    metrics: &mut Metrics,
    values: &mut StateValues,
    id: &str,
    min: i32,
    max: i32,
) {
    match id {
        "money" => resources.money = resources.money.clamp(min, max),
        "energy" => resources.energy = resources.energy.clamp(min, max),
        "influence" => resources.influence = resources.influence.clamp(min, max),
        "days_remaining" => resources.days_remaining = resources.days_remaining.clamp(min, max),
        "progress" => metrics.progress = metrics.progress.clamp(min, max),
        "documentation" => metrics.documentation = metrics.documentation.clamp(min, max),
        "community_support" => {
            metrics.community_support = metrics.community_support.clamp(min, max)
        }
        "public_attention" => metrics.public_attention = metrics.public_attention.clamp(min, max),
        "integrity" => metrics.integrity = metrics.integrity.clamp(min, max),
        _ => {
            if let Some(value) = values.get_mut(id) {
                *value = (*value).clamp(min, max);
            }
        }
    }
}

fn apply_starting_delta(
    resources: &mut Resources,
    metrics: &mut Metrics,
    values: &mut StateValues,
    id: &str,
    delta: i32,
) {
    match id {
        "money" => resources.money += delta,
        "energy" => resources.energy += delta,
        "influence" => resources.influence += delta,
        "days_remaining" => resources.days_remaining += delta,
        "progress" => metrics.progress += delta,
        "documentation" => metrics.documentation += delta,
        "community_support" => metrics.community_support += delta,
        "public_attention" => metrics.public_attention += delta,
        "integrity" => metrics.integrity += delta,
        _ => *values.entry(id.to_string()).or_default() += delta,
    }
}

fn identity_hash(
    seed: u64,
    template: &str,
    template_version: &str,
    template_fingerprint: u64,
    selections: &BTreeMap<String, String>,
    difficulty: &str,
    modifiers: &[String],
) -> u64 {
    let canonical = serde_json::to_vec(&(
        seed,
        template,
        template_version,
        template_fingerprint,
        selections,
        difficulty,
        modifiers,
    ))
    .expect("generation identity is serializable");
    stable_hash(&canonical)
}

fn stable_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

#[derive(Debug, Clone)]
pub struct GeneratedPackStore {
    path: PathBuf,
    packs: Arc<RwLock<HashMap<String, GamePack>>>,
    mutation_lock: Arc<Mutex<()>>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct GeneratedPackFile {
    #[serde(default = "default_store_version")]
    format_version: u32,
    #[serde(default)]
    packs: HashMap<String, GamePack>,
}

fn default_store_version() -> u32 {
    1
}

impl GeneratedPackStore {
    pub async fn load(path: impl Into<PathBuf>) -> std::io::Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let packs = match fs::read_to_string(&path).await {
            Ok(contents) => {
                serde_json::from_str::<GeneratedPackFile>(&contents)
                    .map_err(std::io::Error::other)?
                    .packs
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => HashMap::new(),
            Err(error) => return Err(error),
        };
        for pack in packs.values() {
            if pack.generated.is_none() || !pack.id.starts_with("generated-") {
                return Err(std::io::Error::other(format!(
                    "persisted pack '{}' is not a generated scenario",
                    pack.id
                )));
            }
            validate_pack(pack).map_err(|errors| {
                std::io::Error::other(format!("invalid persisted generated pack: {errors:?}"))
            })?;
        }
        Ok(Self {
            path,
            packs: Arc::new(RwLock::new(packs)),
            mutation_lock: Arc::new(Mutex::new(())),
        })
    }

    pub async fn all(&self) -> Vec<GamePack> {
        self.packs.read().await.values().cloned().collect()
    }

    pub async fn insert(&self, pack: GamePack) -> std::io::Result<()> {
        let _mutation = self.mutation_lock.lock().await;
        self.packs.write().await.insert(pack.id.clone(), pack);
        self.persist().await
    }

    async fn persist(&self) -> std::io::Result<()> {
        let payload = serde_json::to_vec_pretty(&GeneratedPackFile {
            format_version: 1,
            packs: self.packs.read().await.clone(),
        })
        .map_err(std::io::Error::other)?;
        let temporary = self.path.with_extension("tmp");
        fs::write(&temporary, payload).await?;
        fs::rename(temporary, &self.path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_pack::PackRegistry;

    fn fixtures() -> (AbstractionCatalog, PackRegistry) {
        let registry = PackRegistry::load(
            Some(Path::new("game-packs")),
            Path::new("game-packs/drainage/game.json"),
            Some("civic-drainage-v1"),
        )
        .unwrap();
        let ids = registry.all().iter().map(|pack| pack.id.clone()).collect();
        let catalog =
            AbstractionCatalog::load(Path::new("game-packs/abstractions.json"), &ids).unwrap();
        (catalog, registry)
    }

    #[test]
    fn same_request_has_same_identity_and_content() {
        let (catalog, registry) = fixtures();
        let request = GenerateScenarioRequest {
            seed: Some(42),
            ..Default::default()
        };
        let first = generate_pack(&catalog, request.clone(), |id| registry.get(id)).unwrap();
        let second = generate_pack(&catalog, request, |id| registry.get(id)).unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(
            serde_json::to_value(first).unwrap(),
            serde_json::to_value(second).unwrap()
        );
    }

    #[test]
    fn explicit_environment_changes_mechanics_and_metadata() {
        let (catalog, registry) = fixtures();
        let mut selections = BTreeMap::new();
        selections.insert("objective_type".into(), "infrastructure".into());
        selections.insert("administrative_capacity".into(), "weak".into());
        let generated = generate_pack(
            &catalog,
            GenerateScenarioRequest {
                seed: Some(7),
                selections,
                difficulty: "hard".into(),
                ..Default::default()
            },
            |id| registry.get(id),
        )
        .unwrap();
        assert_eq!(generated.environment.administrative_capacity, "Weak");
        assert_eq!(generated.generated.as_ref().unwrap().difficulty, "hard");
        assert_eq!(generated.citizens.len(), 1);
        assert!(
            generated.actions[0].cost.energy
                >= registry.get("civic-drainage-v1").unwrap().actions[0]
                    .cost
                    .energy
        );
    }

    #[test]
    fn explicit_city_plan_sets_map_and_repairs_unspecified_region() {
        let (catalog, registry) = fixtures();
        let selections = [
            ("objective_type".into(), "business_land".into()),
            ("city_plan".into(), "rio_de_janeiro_brazil".into()),
        ]
        .into_iter()
        .collect();
        let generated = generate_pack(
            &catalog,
            GenerateScenarioRequest {
                seed: Some(20260727),
                selections,
                ..Default::default()
            },
            |id| registry.get(id),
        )
        .unwrap();
        assert_eq!(
            generated.visual_theme.map_asset.as_deref(),
            Some("osm:rio-de-janeiro-brazil")
        );
        assert_eq!(
            generated.environment.world_region,
            "Latin American provincial city"
        );
        assert!(generated.title.contains("Rio de Janeiro, Brazil"));
        assert!(generated
            .description
            .contains("institution locations are fictional"));
    }

    #[test]
    fn explicit_incoherent_scale_is_rejected_but_random_scale_is_repaired() {
        let (catalog, registry) = fixtures();
        let incompatible = GenerateScenarioRequest {
            seed: Some(11),
            selections: [
                ("objective_type".into(), "accountability".into()),
                ("government_level".into(), "village".into()),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        };
        let error = generate_pack(&catalog, incompatible, |id| registry.get(id)).unwrap_err();
        assert!(error.to_string().contains("ministerial_scale"));

        let repaired = generate_pack(
            &catalog,
            GenerateScenarioRequest {
                seed: Some(11),
                selections: [("objective_type".into(), "accountability".into())]
                    .into_iter()
                    .collect(),
                ..Default::default()
            },
            |id| registry.get(id),
        )
        .unwrap();
        let level = &repaired.generated.as_ref().unwrap().selections["government_level"];
        assert!(["province_state", "federal_national", "supranational"].contains(&level.as_str()));
    }
}
