use crate::model::*;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::{BTreeSet, HashMap, HashSet};
use thiserror::Error;

const LEGACY_VALUE_IDS: &[&str] = &[
    "money",
    "energy",
    "influence",
    "days_remaining",
    "progress",
    "documentation",
    "community_support",
    "public_attention",
    "integrity",
];

#[derive(Debug, Error)]
pub enum GameError {
    #[error("player profile not found")]
    CitizenNotFound,
    #[error("action not found")]
    ActionNotFound,
    #[error("session is already finished")]
    SessionFinished,
    #[error("insufficient resources: {0}")]
    InsufficientResources(String),
    #[error("action unavailable: {0}")]
    RequirementNotMet(String),
    #[error("action unavailable: this opportunity has already been used")]
    ActionExhausted,
    #[error("game pack contains no outcomes for this action")]
    NoOutcomes,
}

pub fn create_session(
    pack: &GamePack,
    citizen_id: &str,
    seed: u64,
    id: String,
) -> Result<GameSession, GameError> {
    let citizen = pack
        .citizens
        .iter()
        .find(|candidate| candidate.id == citizen_id)
        .ok_or(GameError::CitizenNotFound)?;

    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let hidden = HiddenContext {
        departmental_backlog: rng.random_range(30..=90),
        officer_integrity: rng.random_range(25..=95),
        election_pressure: rng.random_range(10..=90),
        corruption_pressure: rng.random_range(10..=85),
    };
    let mut hidden_values = StateValues::new();
    for definition in &pack.hidden_variable_definitions {
        hidden_values.insert(
            definition.id.clone(),
            rng.random_range(definition.min..=definition.max),
        );
    }

    let mut values = citizen.starting_values.clone();
    insert_legacy_values(
        &mut values,
        &citizen.starting_resources,
        &citizen.starting_metrics,
    );
    for (id, value) in &citizen.skills {
        values.entry(id.clone()).or_insert(*value);
    }
    for (id, value) in &citizen.relationships {
        values.entry(id.clone()).or_insert(*value);
    }

    Ok(GameSession {
        id,
        game_pack_id: pack.id.clone(),
        game_pack_version: pack.version.clone(),
        campaign_id: None,
        campaign_attempt: 0,
        citizen_id: citizen.id.clone(),
        citizen_name: citizen.name.clone(),
        citizen_context: citizen.context.clone(),
        mission_title: pack.mission.title.clone(),
        objective: pack.mission.objective.clone(),
        current_status: pack.mission.starting_status.clone(),
        resources: citizen.starting_resources,
        metrics: citizen.starting_metrics,
        values,
        hidden,
        hidden_values,
        persistent_consequences: StateValues::new(),
        triggered_random_events: HashSet::new(),
        player_modifiers: citizen.modifiers.clone(),
        status: SessionStatus::Active,
        ending_id: None,
        turn: 0,
        seed,
        events: Vec::new(),
        action_results: HashMap::new(),
    })
}

pub fn public_state(session: &GameSession, pack: &GamePack) -> PublicGameState {
    let available_actions = if session.status == SessionStatus::Active {
        pack.actions
            .iter()
            .filter(|action| ensure_available(session, action).is_ok())
            .map(action_as_available)
            .collect()
    } else {
        Vec::new()
    };
    let values = public_values(session);
    let definitions = if pack.value_definitions.is_empty() {
        legacy_value_definitions()
    } else {
        pack.value_definitions.clone()
    };
    let indicators = definitions
        .into_iter()
        .filter(|definition| !definition.hidden_from_hud)
        .map(|definition| PublicIndicator {
            value: metric_value(session, &definition.id) as i32,
            id: definition.id,
            label: definition.label,
            description: definition.description,
            group: definition.group,
            min: definition.min,
            max: definition.max,
            format: definition.format,
        })
        .collect();

    PublicGameState {
        id: session.id.clone(),
        game_pack_id: session.game_pack_id.clone(),
        game_pack_version: if session.game_pack_version.is_empty() {
            pack.version.clone()
        } else {
            session.game_pack_version.clone()
        },
        campaign_id: session.campaign_id.clone(),
        citizen_id: session.citizen_id.clone(),
        citizen_name: session.citizen_name.clone(),
        citizen_context: session.citizen_context.clone(),
        mission_title: session.mission_title.clone(),
        objective: session.objective.clone(),
        current_status: session.current_status.clone(),
        resources: session.resources,
        metrics: session.metrics,
        values,
        indicators,
        persistent_consequences: session.persistent_consequences.clone(),
        status: session.status.clone(),
        ending_id: session.ending_id.clone(),
        turn: session.turn,
        events: session.events.clone(),
        available_actions,
    }
}

pub fn state_value(session: &GameSession, id: &str) -> i32 {
    value_i32(session, id)
}

pub fn apply_inherited_value(session: &mut GameSession, pack: &GamePack, id: &str, delta: i32) {
    adjust_value(session, id, delta);
    clamp_session(session, pack);
}

pub fn apply_action(
    session: &mut GameSession,
    pack: &GamePack,
    request: &ActionRequest,
) -> Result<ActionResponse, GameError> {
    if let Some(previous) = session.action_results.get(&request.client_action_id) {
        return Ok(previous.clone());
    }
    if session.status != SessionStatus::Active {
        return Err(GameError::SessionFinished);
    }

    let action = pack
        .actions
        .iter()
        .find(|candidate| candidate.id == request.action_id)
        .ok_or(GameError::ActionNotFound)?;
    ensure_available(session, action)?;

    let values_before = public_values(session);
    let progress_before = value_i32(session, "progress");
    apply_cost(session, &action.cost);
    apply_delta(session, &action.guaranteed_effect);

    let mut rng = ChaCha8Rng::seed_from_u64(
        session.seed ^ ((session.turn as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15)),
    );
    let outcome = choose_outcome(session, action, &mut rng)?;
    let random_progress = if outcome.progress_max > outcome.progress_min {
        rng.random_range(outcome.progress_min..=outcome.progress_max)
    } else {
        outcome.progress_min
    };

    apply_delta(session, &outcome.effect);
    if random_progress != 0 {
        adjust_value(session, "progress", random_progress);
    }
    clamp_session(session, pack);
    session.turn += 1;
    session.current_status = outcome.message.clone();

    let progress_change = value_i32(session, "progress") - progress_before;
    let action_value_changes = value_changes(&values_before, &public_values(session));
    let action_event = GameEvent {
        turn: session.turn,
        kind: "action".to_string(),
        action_id: action.id.clone(),
        action_title: action.title.clone(),
        outcome_id: outcome.id.clone(),
        message: outcome.message.clone(),
        progress_change,
        resources_after: session.resources,
        value_changes: action_value_changes,
        visual_event: outcome.visual_event.clone(),
    };
    session.events.push(action_event);

    if let Some(world_event) = trigger_random_event(session, pack, &mut rng) {
        session.current_status = world_event.message.clone();
        session.events.push(world_event);
    }
    resolve_end_state(session, pack);

    let response = ActionResponse {
        outcome_id: outcome.id.clone(),
        message: outcome.message.clone(),
        progress_change,
        value_changes: value_changes(&values_before, &public_values(session)),
        visual_event: outcome.visual_event.clone(),
        state: public_state(session, pack),
    };
    session
        .action_results
        .insert(request.client_action_id.clone(), response.clone());
    Ok(response)
}

fn action_as_available(action: &ActionDefinition) -> AvailableAction {
    AvailableAction {
        id: action.id.clone(),
        title: action.title.clone(),
        description: action.description.clone(),
        action_type: action.action_type.clone(),
        location_id: action.location_id.clone(),
        cost: action.cost.clone(),
        enabled: true,
        disabled_reason: None,
    }
}

fn ensure_available(session: &GameSession, action: &ActionDefinition) -> Result<(), GameError> {
    let use_limit = action.max_uses.unwrap_or(1);
    if session
        .events
        .iter()
        .filter(|event| event.kind == "action" && event.action_id == action.id)
        .count()
        >= use_limit as usize
    {
        return Err(GameError::ActionExhausted);
    }
    let mut missing = Vec::new();
    for (id, required) in cost_values(&action.cost) {
        if value_i32(session, &id) < required {
            missing.push(format!("{required} {id}"));
        }
    }
    if value_i32(session, "days_remaining") <= action.cost.days {
        missing.push(format!("{} days", action.cost.days));
    }
    if !missing.is_empty() {
        return Err(GameError::InsufficientResources(format!(
            "requires {}",
            missing.join(", ")
        )));
    }
    for requirement in &action.requirements {
        if !condition_matches(session, requirement) {
            return Err(GameError::RequirementNotMet(requirement.message.clone()));
        }
    }
    Ok(())
}

fn choose_outcome<'a>(
    session: &GameSession,
    action: &'a ActionDefinition,
    rng: &mut ChaCha8Rng,
) -> Result<&'a OutcomeDefinition, GameError> {
    if action.outcomes.is_empty() {
        return Err(GameError::NoOutcomes);
    }
    let action_modifier = session
        .player_modifiers
        .get(&format!("action:{}", action.action_type))
        .copied()
        .unwrap_or(1.0);
    let weights: Vec<f64> = action
        .outcomes
        .iter()
        .map(|outcome| {
            let conditional = outcome.conditions.iter().fold(1.0, |acc, condition| {
                let value = metric_value(session, &condition.metric);
                let matches = condition.min.is_none_or(|min| value >= min)
                    && condition.max.is_none_or(|max| value <= max);
                if matches {
                    acc * condition.multiplier
                } else {
                    acc
                }
            });
            let outcome_modifier = session
                .player_modifiers
                .get(&format!("outcome:{}", outcome.id))
                .copied()
                .unwrap_or(1.0);
            (outcome.base_weight * conditional * action_modifier * outcome_modifier).max(0.001)
        })
        .collect();
    let total: f64 = weights.iter().sum();
    let mut roll = rng.random_range(0.0..total);
    for (index, weight) in weights.iter().enumerate() {
        if roll < *weight {
            return Ok(&action.outcomes[index]);
        }
        roll -= weight;
    }
    Ok(action.outcomes.last().expect("checked non-empty"))
}

fn trigger_random_event(
    session: &mut GameSession,
    pack: &GamePack,
    rng: &mut ChaCha8Rng,
) -> Option<GameEvent> {
    for event in &pack.random_events {
        if event.once && session.triggered_random_events.contains(&event.id) {
            continue;
        }
        if !event
            .conditions
            .iter()
            .all(|condition| condition_matches(session, condition))
        {
            continue;
        }
        if rng.random::<f64>() >= event.chance_per_turn {
            continue;
        }

        let before = public_values(session);
        let progress_before = value_i32(session, "progress");
        apply_delta(session, &event.effect);
        clamp_session(session, pack);
        session.triggered_random_events.insert(event.id.clone());
        return Some(GameEvent {
            turn: session.turn,
            kind: "random_event".to_string(),
            action_id: event.id.clone(),
            action_title: event.title.clone(),
            outcome_id: event.id.clone(),
            message: event.message.clone(),
            progress_change: value_i32(session, "progress") - progress_before,
            resources_after: session.resources,
            value_changes: value_changes(&before, &public_values(session)),
            visual_event: event.visual_event.clone(),
        });
    }
    None
}

fn condition_matches(session: &GameSession, condition: &Requirement) -> bool {
    let value = metric_value(session, &condition.metric);
    condition.min.is_none_or(|min| value >= min) && condition.max.is_none_or(|max| value <= max)
}

fn metric_value(session: &GameSession, metric: &str) -> f64 {
    if let Some(value) = session.values.get(metric) {
        return *value as f64;
    }
    if let Some(value) = session.hidden_values.get(metric) {
        return *value as f64;
    }
    if let Some(value) = session.persistent_consequences.get(metric) {
        return *value as f64;
    }
    match metric {
        "progress" => session.metrics.progress as f64,
        "documentation" => session.metrics.documentation as f64,
        "community_support" => session.metrics.community_support as f64,
        "public_attention" => session.metrics.public_attention as f64,
        "integrity" => session.metrics.integrity as f64,
        "money" => session.resources.money as f64,
        "energy" => session.resources.energy as f64,
        "influence" => session.resources.influence as f64,
        "days_remaining" => session.resources.days_remaining as f64,
        "departmental_backlog" => session.hidden.departmental_backlog as f64,
        "officer_integrity" => session.hidden.officer_integrity as f64,
        "election_pressure" => session.hidden.election_pressure as f64,
        "corruption_pressure" => session.hidden.corruption_pressure as f64,
        _ => 0.0,
    }
}

fn value_i32(session: &GameSession, id: &str) -> i32 {
    metric_value(session, id) as i32
}

fn apply_cost(session: &mut GameSession, cost: &ResourceCost) {
    adjust_value(session, "money", -cost.money);
    adjust_value(session, "energy", -cost.energy);
    adjust_value(session, "influence", -cost.influence);
    adjust_value(session, "days_remaining", -cost.days);
    for (id, value) in &cost.values {
        adjust_value(session, id, -*value);
    }
}

fn cost_values(cost: &ResourceCost) -> Vec<(String, i32)> {
    let mut values = Vec::new();
    if cost.money > 0 {
        values.push(("money".to_string(), cost.money));
    }
    if cost.energy > 0 {
        values.push(("energy".to_string(), cost.energy));
    }
    if cost.influence > 0 {
        values.push(("influence".to_string(), cost.influence));
    }
    values.extend(cost.values.iter().map(|(id, value)| (id.clone(), *value)));
    values
}

fn apply_delta(session: &mut GameSession, delta: &StateDelta) {
    adjust_value(session, "money", delta.resources.money);
    adjust_value(session, "energy", delta.resources.energy);
    adjust_value(session, "influence", delta.resources.influence);
    adjust_value(session, "days_remaining", delta.resources.days_remaining);
    adjust_value(session, "progress", delta.progress);
    adjust_value(session, "documentation", delta.documentation);
    adjust_value(session, "community_support", delta.community_support);
    adjust_value(session, "public_attention", delta.public_attention);
    adjust_value(session, "integrity", delta.integrity);
    for (id, value) in &delta.values {
        adjust_value(session, id, *value);
    }
    for (id, value) in &delta.consequences {
        *session
            .persistent_consequences
            .entry(id.clone())
            .or_default() += *value;
    }
}

fn adjust_value(session: &mut GameSession, id: &str, delta: i32) {
    if delta == 0 {
        return;
    }
    let current = value_i32(session, id);
    set_value(session, id, current + delta);
}

fn set_value(session: &mut GameSession, id: &str, value: i32) {
    session.values.insert(id.to_string(), value);
    match id {
        "money" => session.resources.money = value,
        "energy" => session.resources.energy = value,
        "influence" => session.resources.influence = value,
        "days_remaining" => session.resources.days_remaining = value,
        "progress" => session.metrics.progress = value,
        "documentation" => session.metrics.documentation = value,
        "community_support" => session.metrics.community_support = value,
        "public_attention" => session.metrics.public_attention = value,
        "integrity" => session.metrics.integrity = value,
        _ => {}
    }
}

fn clamp_session(session: &mut GameSession, pack: &GamePack) {
    let definitions = if pack.value_definitions.is_empty() {
        legacy_value_definitions()
    } else {
        pack.value_definitions.clone()
    };
    for definition in definitions {
        let value = value_i32(session, &definition.id).clamp(definition.min, definition.max);
        set_value(session, &definition.id, value);
    }
    set_value(
        session,
        "progress",
        value_i32(session, "progress").clamp(0, 100),
    );
    set_value(
        session,
        "documentation",
        value_i32(session, "documentation").clamp(0, 100),
    );
    set_value(
        session,
        "community_support",
        value_i32(session, "community_support").clamp(0, 100),
    );
    set_value(
        session,
        "public_attention",
        value_i32(session, "public_attention").clamp(0, 100),
    );
    set_value(
        session,
        "integrity",
        value_i32(session, "integrity").clamp(0, 100),
    );
    set_value(
        session,
        "energy",
        value_i32(session, "energy").clamp(0, 100),
    );
    set_value(
        session,
        "influence",
        value_i32(session, "influence").clamp(0, 100),
    );
    set_value(
        session,
        "days_remaining",
        value_i32(session, "days_remaining").max(0),
    );
}

fn resolve_end_state(session: &mut GameSession, pack: &GamePack) {
    for ending in &pack.endings {
        if ending.conditions.is_empty()
            || !ending
                .conditions
                .iter()
                .all(|condition| condition_matches(session, condition))
        {
            continue;
        }
        session.status = ending.status.clone();
        session.ending_id = Some(ending.id.clone());
        session.current_status = ending.message.clone();
        return;
    }

    if pack.endings.is_empty() && value_i32(session, "progress") >= pack.mission.win_progress {
        session.status = SessionStatus::Won;
        session.ending_id = Some("completed".to_string());
        session.current_status = format!("{} completed successfully.", pack.mission.title);
        return;
    }
    if value_i32(session, "days_remaining") <= 0 {
        session.status = SessionStatus::Lost;
        session.ending_id = Some("deadline_expired".to_string());
        session.current_status =
            "The deadline expired before the objective was completed.".to_string();
    } else if value_i32(session, "energy") <= 0 {
        session.status = SessionStatus::Lost;
        session.ending_id = Some("exhausted".to_string());
        session.current_status = "The player exhausted their capacity to continue.".to_string();
    } else if value_i32(session, "money") < 0 {
        session.status = SessionStatus::Lost;
        session.ending_id = Some("insolvent".to_string());
        session.current_status = "The campaign can no longer be financed.".to_string();
    } else if !pack
        .actions
        .iter()
        .any(|action| ensure_available(session, action).is_ok())
    {
        session.status = SessionStatus::Lost;
        session.ending_id = Some("no_available_actions".to_string());
        session.current_status =
            "No affordable or procedurally available action remains.".to_string();
    }
}

fn insert_legacy_values(values: &mut StateValues, resources: &Resources, metrics: &Metrics) {
    let pairs = [
        ("money", resources.money),
        ("energy", resources.energy),
        ("influence", resources.influence),
        ("days_remaining", resources.days_remaining),
        ("progress", metrics.progress),
        ("documentation", metrics.documentation),
        ("community_support", metrics.community_support),
        ("public_attention", metrics.public_attention),
        ("integrity", metrics.integrity),
    ];
    for (id, value) in pairs {
        values.entry(id.to_string()).or_insert(value);
    }
}

fn public_values(session: &GameSession) -> StateValues {
    let mut values = session.values.clone();
    insert_legacy_values(&mut values, &session.resources, &session.metrics);
    values
}

fn value_changes(before: &StateValues, after: &StateValues) -> StateValues {
    let ids: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    ids.into_iter()
        .filter_map(|id| {
            let change = after.get(id).copied().unwrap_or_default()
                - before.get(id).copied().unwrap_or_default();
            (change != 0).then(|| (id.clone(), change))
        })
        .collect()
}

fn legacy_value_definitions() -> Vec<ValueDefinition> {
    LEGACY_VALUE_IDS
        .iter()
        .map(|id| {
            let (label, group, min, max, format) = match *id {
                "money" => (
                    "Money",
                    ValueGroup::Resource,
                    -1_000_000,
                    1_000_000,
                    ValueFormat::Money,
                ),
                "energy" => ("Energy", ValueGroup::Resource, 0, 100, ValueFormat::Percent),
                "influence" => (
                    "Influence",
                    ValueGroup::Resource,
                    0,
                    100,
                    ValueFormat::Percent,
                ),
                "days_remaining" => (
                    "Days remaining",
                    ValueGroup::Resource,
                    0,
                    10_000,
                    ValueFormat::Days,
                ),
                "progress" => (
                    "Mission progress",
                    ValueGroup::Metric,
                    0,
                    100,
                    ValueFormat::Percent,
                ),
                "documentation" => (
                    "Documentation",
                    ValueGroup::Metric,
                    0,
                    100,
                    ValueFormat::Percent,
                ),
                "community_support" => (
                    "Community support",
                    ValueGroup::Metric,
                    0,
                    100,
                    ValueFormat::Percent,
                ),
                "public_attention" => (
                    "Public attention",
                    ValueGroup::Metric,
                    0,
                    100,
                    ValueFormat::Percent,
                ),
                "integrity" => (
                    "Integrity",
                    ValueGroup::Metric,
                    0,
                    100,
                    ValueFormat::Percent,
                ),
                _ => unreachable!(),
            };
            ValueDefinition {
                id: (*id).to_string(),
                label: label.to_string(),
                description: String::new(),
                group,
                min,
                max,
                format,
                hidden_from_hud: false,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_pack() -> GamePack {
        serde_json::from_str(include_str!("../game-packs/drainage/game.json")).unwrap()
    }

    #[test]
    fn same_seed_and_action_produce_same_outcome() {
        let pack = sample_pack();
        let mut first = create_session(&pack, "shopkeeper", 42, "a".into()).unwrap();
        let mut second = create_session(&pack, "shopkeeper", 42, "b".into()).unwrap();
        let request = ActionRequest {
            action_id: "file_complaint".into(),
            client_action_id: "click-1".into(),
        };

        let first_result = apply_action(&mut first, &pack, &request).unwrap();
        let second_result = apply_action(&mut second, &pack, &request).unwrap();

        assert_eq!(first_result.outcome_id, second_result.outcome_id);
        assert_eq!(first_result.progress_change, second_result.progress_change);
    }

    #[test]
    fn repeated_client_action_is_idempotent() {
        let pack = sample_pack();
        let mut session = create_session(&pack, "shopkeeper", 55, "a".into()).unwrap();
        let request = ActionRequest {
            action_id: "file_complaint".into(),
            client_action_id: "same-click".into(),
        };

        let first = apply_action(&mut session, &pack, &request).unwrap();
        let resources_after_first = session.resources;
        let second = apply_action(&mut session, &pack, &request).unwrap();

        assert_eq!(first.outcome_id, second.outcome_id);
        assert_eq!(session.resources.money, resources_after_first.money);
        assert_eq!(session.turn, 1);
    }

    #[test]
    fn a_consumed_action_cannot_be_spammed_for_more_progress() {
        let mut pack = sample_pack();
        pack.actions[0].max_uses = Some(1);
        let mut session = create_session(&pack, "shopkeeper", 55, "limited".into()).unwrap();
        apply_action(
            &mut session,
            &pack,
            &ActionRequest {
                action_id: "file_complaint".into(),
                client_action_id: "first".into(),
            },
        )
        .unwrap();
        let progress = session.metrics.progress;
        let turn = session.turn;

        let repeated = apply_action(
            &mut session,
            &pack,
            &ActionRequest {
                action_id: "file_complaint".into(),
                client_action_id: "second".into(),
            },
        );

        assert!(matches!(repeated, Err(GameError::ActionExhausted)));
        assert_eq!(session.metrics.progress, progress);
        assert_eq!(session.turn, turn);
        assert!(!public_state(&session, &pack)
            .available_actions
            .iter()
            .any(|action| action.id == "file_complaint"));
    }

    #[test]
    fn action_response_reports_visible_factor_changes() {
        let pack = sample_pack();
        let mut session = create_session(&pack, "shopkeeper", 42, "changes".into()).unwrap();
        let response = apply_action(
            &mut session,
            &pack,
            &ActionRequest {
                action_id: "file_complaint".into(),
                client_action_id: "factor-feedback".into(),
            },
        )
        .unwrap();

        assert!(response.value_changes["documentation"] >= 8);
        assert_eq!(response.value_changes["institutional_pressure"], 5);
    }

    #[test]
    fn unavailable_actions_are_hidden_until_state_unlocks_them() {
        let mut pack = sample_pack();
        pack.value_definitions.push(ValueDefinition {
            id: "whistleblower_packet_available".into(),
            label: "Whistleblower packet".into(),
            description: String::new(),
            group: ValueGroup::Metric,
            min: 0,
            max: 1,
            format: ValueFormat::Number,
            hidden_from_hud: true,
        });
        pack.actions[1].id = "verify_whistleblower_evidence".into();
        pack.actions[1].requirements = vec![Requirement {
            metric: "whistleblower_packet_available".into(),
            min: Some(1.0),
            max: None,
            message: "No new packet is available.".into(),
        }];
        pack.random_events = vec![RandomEventDefinition {
            id: "whistleblower_packet".into(),
            title: "Whistleblower packet arrives".into(),
            message: "A new packet is available for verification.".into(),
            chance_per_turn: 1.0,
            once: true,
            conditions: Vec::new(),
            effect: StateDelta {
                values: [("whistleblower_packet_available".into(), 1)]
                    .into_iter()
                    .collect(),
                ..Default::default()
            },
            visual_event: None,
        }];
        let mut session = create_session(&pack, "shopkeeper", 7, "unlock".into()).unwrap();

        assert!(!public_state(&session, &pack)
            .available_actions
            .iter()
            .any(|action| action.id == "verify_whistleblower_evidence"));
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        let event = trigger_random_event(&mut session, &pack, &mut rng).unwrap();
        assert_eq!(event.action_id, "whistleblower_packet");
        assert!(public_state(&session, &pack)
            .available_actions
            .iter()
            .any(|action| action.id == "verify_whistleblower_evidence"));
    }

    #[test]
    fn verification_advances_through_distinct_evidence_stages() {
        let pack: GamePack =
            serde_json::from_str(include_str!("../game-packs/examination-scandal/game.json"))
                .unwrap();
        let mut session = create_session(&pack, "exam_candidate", 19, "stages".into()).unwrap();
        let stages = [
            ("verify_leak", 1),
            ("corroborate_leak", 2),
            ("confirm_chain_of_custody", 3),
            ("prepare_admissible_record", 4),
        ];

        for (index, (action_id, expected_stage)) in stages.iter().enumerate() {
            assert!(public_state(&session, &pack)
                .available_actions
                .iter()
                .any(|action| action.id == *action_id));
            apply_action(
                &mut session,
                &pack,
                &ActionRequest {
                    action_id: (*action_id).into(),
                    client_action_id: format!("stage-{index}"),
                },
            )
            .unwrap();
            assert_eq!(session.values["evidence_stage"], *expected_stage);
            assert!(!public_state(&session, &pack)
                .available_actions
                .iter()
                .any(|action| action.id == *action_id));
        }
    }

    #[test]
    fn high_generic_progress_cannot_bypass_multifactor_victory() {
        let mut pack = sample_pack();
        for (id, value) in [
            ("evidence_strength", 60),
            ("public_support", 60),
            ("institutional_pressure", 60),
        ] {
            pack.value_definitions.push(ValueDefinition {
                id: id.into(),
                label: id.into(),
                description: String::new(),
                group: ValueGroup::Metric,
                min: 0,
                max: 100,
                format: ValueFormat::Percent,
                hidden_from_hud: false,
            });
            pack.citizens[0].starting_values.insert(id.into(), value);
        }
        pack.citizens[0].starting_metrics.progress = 100;
        pack.endings = vec![EndingDefinition {
            id: "multifactor_win".into(),
            title: "Multifactor win".into(),
            message: "Every necessary condition is present.".into(),
            status: SessionStatus::Won,
            conditions: vec![
                Requirement {
                    metric: "evidence_strength".into(),
                    min: Some(70.0),
                    max: None,
                    message: String::new(),
                },
                Requirement {
                    metric: "public_support".into(),
                    min: Some(70.0),
                    max: None,
                    message: String::new(),
                },
                Requirement {
                    metric: "institutional_pressure".into(),
                    min: Some(70.0),
                    max: None,
                    message: String::new(),
                },
            ],
        }];
        let mut session = create_session(&pack, "shopkeeper", 7, "factors".into()).unwrap();
        resolve_end_state(&mut session, &pack);
        assert_eq!(session.status, SessionStatus::Active);
        session.values.insert("evidence_strength".into(), 70);
        session.values.insert("public_support".into(), 70);
        session.values.insert("institutional_pressure".into(), 70);
        resolve_end_state(&mut session, &pack);
        assert_eq!(session.status, SessionStatus::Won);
    }

    #[test]
    fn generic_values_can_unlock_actions_and_resolve_endings() {
        let mut pack = sample_pack();
        pack.value_definitions.push(ValueDefinition {
            id: "coalition_strength".into(),
            label: "Coalition strength".into(),
            description: String::new(),
            group: ValueGroup::Metric,
            min: 0,
            max: 100,
            format: ValueFormat::Percent,
            hidden_from_hud: false,
        });
        pack.citizens[0]
            .starting_values
            .insert("coalition_strength".into(), 50);
        pack.actions[0].requirements = vec![Requirement {
            metric: "coalition_strength".into(),
            min: Some(40.0),
            max: None,
            message: "Build a coalition first.".into(),
        }];
        pack.actions[0]
            .guaranteed_effect
            .values
            .insert("coalition_strength".into(), 50);
        pack.endings = vec![EndingDefinition {
            id: "coalition_wins".into(),
            title: "Coalition wins".into(),
            message: "The coalition achieved its objective.".into(),
            status: SessionStatus::Won,
            conditions: vec![Requirement {
                metric: "coalition_strength".into(),
                min: Some(100.0),
                max: None,
                message: String::new(),
            }],
        }];
        let mut session = create_session(&pack, "shopkeeper", 7, "generic".into()).unwrap();
        let response = apply_action(
            &mut session,
            &pack,
            &ActionRequest {
                action_id: "file_complaint".into(),
                client_action_id: "generic-action".into(),
            },
        )
        .unwrap();

        assert_eq!(response.state.values["coalition_strength"], 100);
        assert_eq!(response.state.status, SessionStatus::Won);
        assert_eq!(response.state.ending_id.as_deref(), Some("coalition_wins"));
    }
}
