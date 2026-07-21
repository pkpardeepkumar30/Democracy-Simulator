use crate::model::*;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GameError {
    #[error("citizen profile not found")]
    CitizenNotFound,
    #[error("action not found")]
    ActionNotFound,
    #[error("session is already finished")]
    SessionFinished,
    #[error("insufficient resources: {0}")]
    InsufficientResources(String),
    #[error("action unavailable: {0}")]
    RequirementNotMet(String),
    #[error("game pack contains no outcomes for this action")]
    NoOutcomes,
}

pub fn create_session(pack: &GamePack, citizen_id: &str, seed: u64, id: String) -> Result<GameSession, GameError> {
    let citizen = pack
        .citizens
        .iter()
        .find(|c| c.id == citizen_id)
        .ok_or(GameError::CitizenNotFound)?;

    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let hidden = HiddenContext {
        departmental_backlog: rng.random_range(30..=90),
        officer_integrity: rng.random_range(25..=95),
        election_pressure: rng.random_range(10..=90),
        corruption_pressure: rng.random_range(10..=85),
    };

    Ok(GameSession {
        id,
        game_pack_id: pack.id.clone(),
        citizen_id: citizen.id.clone(),
        citizen_name: citizen.name.clone(),
        citizen_context: citizen.context.clone(),
        mission_title: pack.mission.title.clone(),
        objective: pack.mission.objective.clone(),
        current_status: pack.mission.starting_status.clone(),
        resources: citizen.starting_resources,
        metrics: citizen.starting_metrics,
        hidden,
        status: SessionStatus::Active,
        turn: 0,
        seed,
        events: Vec::new(),
        action_results: HashMap::new(),
    })
}

pub fn public_state(session: &GameSession, pack: &GamePack) -> PublicGameState {
    let available_actions = pack
        .actions
        .iter()
        .map(|action| action_availability(session, action))
        .collect();

    PublicGameState {
        id: session.id.clone(),
        citizen_id: session.citizen_id.clone(),
        citizen_name: session.citizen_name.clone(),
        citizen_context: session.citizen_context.clone(),
        mission_title: session.mission_title.clone(),
        objective: session.objective.clone(),
        current_status: session.current_status.clone(),
        resources: session.resources,
        metrics: session.metrics,
        status: session.status.clone(),
        turn: session.turn,
        events: session.events.clone(),
        available_actions,
    }
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
        .find(|a| a.id == request.action_id)
        .ok_or(GameError::ActionNotFound)?;

    ensure_available(session, action)?;
    session.resources.apply_cost(&action.cost);
    apply_delta(session, &action.guaranteed_effect);

    let mut rng = ChaCha8Rng::seed_from_u64(
        session.seed
            ^ ((session.turn as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15)),
    );

    let outcome = choose_outcome(session, action, &mut rng)?;
    let random_progress = if outcome.progress_max > outcome.progress_min {
        rng.random_range(outcome.progress_min..=outcome.progress_max)
    } else {
        outcome.progress_min
    };

    let progress_before = session.metrics.progress;
    apply_delta(session, &outcome.effect);
    session.metrics.progress += random_progress;
    clamp_session(session);
    session.turn += 1;
    session.current_status = outcome.message.clone();
    resolve_end_state(session, pack);

    let progress_change = session.metrics.progress - progress_before;
    let event = GameEvent {
        turn: session.turn,
        action_id: action.id.clone(),
        action_title: action.title.clone(),
        outcome_id: outcome.id.clone(),
        message: outcome.message.clone(),
        progress_change,
        resources_after: session.resources,
    };
    session.events.push(event);

    let response = ActionResponse {
        outcome_id: outcome.id.clone(),
        message: outcome.message.clone(),
        progress_change,
        state: public_state(session, pack),
    };

    session
        .action_results
        .insert(request.client_action_id.clone(), response.clone());

    Ok(response)
}

fn action_availability(session: &GameSession, action: &ActionDefinition) -> AvailableAction {
    let result = ensure_available(session, action);
    AvailableAction {
        id: action.id.clone(),
        title: action.title.clone(),
        description: action.description.clone(),
        cost: action.cost,
        enabled: result.is_ok() && session.status == SessionStatus::Active,
        disabled_reason: result.err().map(|e| e.to_string()),
    }
}

fn ensure_available(session: &GameSession, action: &ActionDefinition) -> Result<(), GameError> {
    if !session.resources.can_afford(&action.cost) {
        return Err(GameError::InsufficientResources(format!(
            "requires ₹{}, {} energy, {} influence and {} days",
            action.cost.money, action.cost.energy, action.cost.influence, action.cost.days
        )));
    }

    for requirement in &action.requirements {
        let value = metric_value(session, &requirement.metric);
        if requirement.min.is_some_and(|min| value < min)
            || requirement.max.is_some_and(|max| value > max)
        {
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

    let weights: Vec<f64> = action
        .outcomes
        .iter()
        .map(|outcome| {
            let multiplier = outcome.conditions.iter().fold(1.0, |acc, condition| {
                let value = metric_value(session, &condition.metric);
                let matches = condition.min.is_none_or(|min| value >= min)
                    && condition.max.is_none_or(|max| value <= max);
                if matches {
                    acc * condition.multiplier
                } else {
                    acc
                }
            });
            (outcome.base_weight * multiplier).max(0.001)
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

fn metric_value(session: &GameSession, metric: &str) -> f64 {
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

fn apply_delta(session: &mut GameSession, delta: &StateDelta) {
    session.resources.apply_delta(&delta.resources);
    session.metrics.progress += delta.progress;
    session.metrics.documentation += delta.documentation;
    session.metrics.community_support += delta.community_support;
    session.metrics.public_attention += delta.public_attention;
    session.metrics.integrity += delta.integrity;
}

fn clamp_session(session: &mut GameSession) {
    session.metrics.progress = session.metrics.progress.clamp(0, 100);
    session.metrics.documentation = session.metrics.documentation.clamp(0, 100);
    session.metrics.community_support = session.metrics.community_support.clamp(0, 100);
    session.metrics.public_attention = session.metrics.public_attention.clamp(0, 100);
    session.metrics.integrity = session.metrics.integrity.clamp(0, 100);
    session.resources.energy = session.resources.energy.clamp(0, 100);
    session.resources.influence = session.resources.influence.clamp(0, 100);
    session.resources.days_remaining = session.resources.days_remaining.max(0);
}

fn resolve_end_state(session: &mut GameSession, pack: &GamePack) {
    if session.metrics.progress >= pack.mission.win_progress {
        session.status = SessionStatus::Won;
        session.current_status = "The drainage repair has been completed and verified by residents.".to_string();
        return;
    }

    if session.resources.days_remaining <= 0 {
        session.status = SessionStatus::Lost;
        session.current_status = "The deadline expired before the work could be completed.".to_string();
    } else if session.resources.energy <= 0 {
        session.status = SessionStatus::Lost;
        session.current_status = "The citizen exhausted their capacity to continue pursuing the case.".to_string();
    } else if session.resources.money < 0 {
        session.status = SessionStatus::Lost;
        session.current_status = "The citizen can no longer finance the campaign.".to_string();
    }
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
}
