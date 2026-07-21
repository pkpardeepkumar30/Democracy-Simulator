use civic_sim_server::{
    engine::{apply_action, create_session},
    game_pack::PackRegistry,
    generator::{generate_pack, AbstractionCatalog, GenerateScenarioRequest},
    model::{ActionDefinition, ActionRequest, GamePack, SessionStatus, StateDelta},
};
use std::{collections::BTreeMap, env, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let trace_requested = arguments.first().is_some_and(|value| value == "trace");
    let generated_requested = arguments.first().is_some_and(|value| value == "generated");
    let runs: u64 = if trace_requested {
        1
    } else if generated_requested {
        arguments
            .get(3)
            .map(String::as_str)
            .unwrap_or("250")
            .parse()?
    } else {
        arguments
            .first()
            .map(String::as_str)
            .unwrap_or("250")
            .parse()?
    };
    let packs_path = env::var("GAME_PACKS_PATH").unwrap_or_else(|_| "game-packs".to_string());
    let registry = PackRegistry::load(
        Some(Path::new(&packs_path)),
        Path::new("game-packs/drainage/game.json"),
        Some("civic-drainage-v1"),
    )?;

    if trace_requested {
        let pack_id = arguments
            .get(1)
            .map(String::as_str)
            .unwrap_or("factory-ground-v1");
        let pack = registry.get(pack_id).ok_or("trace pack not found")?;
        let profile_id = arguments
            .get(2)
            .map(String::as_str)
            .unwrap_or(&pack.citizens[0].id);
        let seed = arguments
            .get(3)
            .map(String::as_str)
            .unwrap_or("17")
            .parse()?;
        trace_session(&pack, profile_id, seed)?;
        return Ok(());
    }

    if generated_requested {
        let objective = arguments
            .get(1)
            .cloned()
            .unwrap_or_else(|| "infrastructure".to_string());
        let difficulty = arguments
            .get(2)
            .cloned()
            .unwrap_or_else(|| "standard".to_string());
        let ids = registry.all().iter().map(|pack| pack.id.clone()).collect();
        let catalog = AbstractionCatalog::load(Path::new("game-packs/abstractions.json"), &ids)?;
        let pack = generate_pack(
            &catalog,
            GenerateScenarioRequest {
                seed: Some(42_4242),
                selections: [("objective_type".to_string(), objective)]
                    .into_iter()
                    .collect(),
                difficulty,
                ..Default::default()
            },
            |id| registry.get(id),
        )?;
        println!("Democracy Simulator generated Monte Carlo: {runs} runs per profile");
        simulate_pack(&pack, runs)?;
        return Ok(());
    }

    println!("Democracy Simulator Monte Carlo: {runs} runs per profile");
    for summary in registry.summaries() {
        let pack = registry.get(&summary.id).expect("summary pack exists");
        simulate_pack(&pack, runs)?;
    }
    Ok(())
}

fn simulate_pack(pack: &GamePack, runs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{} ({})", pack.title, pack.id);
    for profile in &pack.citizens {
        let mut statuses: BTreeMap<String, u64> = BTreeMap::new();
        let mut endings: BTreeMap<String, u64> = BTreeMap::new();
        let mut total_turns = 0_u64;
        for seed in 0..runs {
            let mut session = create_session(
                pack,
                &profile.id,
                seed.wrapping_mul(7_919).wrapping_add(17),
                format!("simulation-{seed}"),
            )?;
            for turn in 0..80 {
                if session.status != SessionStatus::Active {
                    break;
                }
                let state = civic_sim_server::engine::public_state(&session, pack);
                let Some(action) = choose_action(pack, &state) else {
                    break;
                };
                apply_action(
                    &mut session,
                    pack,
                    &ActionRequest {
                        action_id: action.id.clone(),
                        client_action_id: format!("simulation-{seed}-{turn}"),
                    },
                )?;
            }
            total_turns += session.turn as u64;
            *statuses
                .entry(format!("{:?}", session.status).to_lowercase())
                .or_default() += 1;
            *endings
                .entry(
                    session
                        .ending_id
                        .clone()
                        .unwrap_or_else(|| "no_ending".to_string()),
                )
                .or_default() += 1;
        }
        let average_turns = total_turns as f64 / runs.max(1) as f64;
        println!(
            "  {:<22} avg turns {:>5.1}  statuses {:?}  endings {:?}",
            profile.id, average_turns, statuses, endings
        );
    }
    Ok(())
}

fn trace_session(
    pack: &GamePack,
    profile_id: &str,
    seed: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut session = create_session(pack, profile_id, seed, "trace".to_string())?;
    println!("Trace: {} / {} / seed {}", pack.id, profile_id, seed);
    for turn in 0..80 {
        let state = civic_sim_server::engine::public_state(&session, pack);
        println!(
            "turn {:>2} status {:?} money {} energy {} days {} values {:?}",
            state.turn,
            state.status,
            state.resources.money,
            state.resources.energy,
            state.resources.days_remaining,
            state.values
        );
        if state.status != SessionStatus::Active {
            println!("ending {:?}: {}", state.ending_id, state.current_status);
            break;
        }
        let Some(action) = choose_action(pack, &state) else {
            println!("no enabled action");
            break;
        };
        let target = pack
            .endings
            .iter()
            .find(|ending| ending.status == SessionStatus::Won);
        println!(
            "  choose {} (score {:.4})",
            action.id,
            action_score(action, &state, target)
        );
        let response = apply_action(
            &mut session,
            pack,
            &ActionRequest {
                action_id: action.id.clone(),
                client_action_id: format!("trace-{turn}"),
            },
        )?;
        println!("  outcome {}: {}", response.outcome_id, response.message);
    }
    Ok(())
}

fn choose_action<'a>(
    pack: &'a GamePack,
    state: &civic_sim_server::model::PublicGameState,
) -> Option<&'a ActionDefinition> {
    let enabled: Vec<_> = state
        .available_actions
        .iter()
        .filter(|action| action.enabled)
        .collect();
    let target = pack
        .endings
        .iter()
        .find(|ending| ending.status == SessionStatus::Won);

    pack.actions
        .iter()
        .filter(|action| enabled.iter().any(|available| available.id == action.id))
        .max_by(|left, right| {
            action_score(left, state, target)
                .total_cmp(&action_score(right, state, target))
                .then_with(|| right.id.cmp(&left.id))
        })
}

fn action_score(
    action: &ActionDefinition,
    state: &civic_sim_server::model::PublicGameState,
    target: Option<&civic_sim_server::model::EndingDefinition>,
) -> f64 {
    let mut score = 0.0;
    let requirements = target
        .map(|ending| ending.conditions.as_slice())
        .unwrap_or(&[]);
    for requirement in requirements {
        let Some(minimum) = requirement.min else {
            continue;
        };
        let current = state
            .values
            .get(&requirement.metric)
            .copied()
            .unwrap_or_default() as f64;
        if current >= minimum {
            continue;
        }
        let deficit = (minimum - current).max(1.0);
        let guaranteed = delta_for(&action.guaranteed_effect, &requirement.metric) as f64;
        let total_weight: f64 = action
            .outcomes
            .iter()
            .map(|outcome| outcome.base_weight)
            .sum();
        let expected = action
            .outcomes
            .iter()
            .map(|outcome| {
                let progress = if requirement.metric == "progress" {
                    (outcome.progress_min + outcome.progress_max) as f64 / 2.0
                } else {
                    0.0
                };
                outcome.base_weight
                    * (delta_for(&outcome.effect, &requirement.metric) as f64 + progress)
            })
            .sum::<f64>()
            / total_weight.max(0.001);
        score += (guaranteed + expected).max(-deficit) / deficit;
    }
    let expected_progress = action
        .outcomes
        .iter()
        .map(|outcome| {
            outcome.base_weight
                * ((outcome.progress_min + outcome.progress_max) as f64 / 2.0
                    + outcome.effect.progress as f64)
        })
        .sum::<f64>()
        / action
            .outcomes
            .iter()
            .map(|outcome| outcome.base_weight)
            .sum::<f64>()
            .max(0.001);
    score + (action.guaranteed_effect.progress as f64 + expected_progress) * 0.002
        - action.cost.days as f64 * 0.0005
}

fn delta_for(delta: &StateDelta, id: &str) -> i32 {
    delta.values.get(id).copied().unwrap_or_default()
        + match id {
            "money" => delta.resources.money,
            "energy" => delta.resources.energy,
            "influence" => delta.resources.influence,
            "days_remaining" => delta.resources.days_remaining,
            "progress" => delta.progress,
            "documentation" => delta.documentation,
            "community_support" => delta.community_support,
            "public_attention" => delta.public_attention,
            "integrity" => delta.integrity,
            _ => 0,
        }
}
