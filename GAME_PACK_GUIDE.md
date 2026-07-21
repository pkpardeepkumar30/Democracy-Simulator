# Game-pack guide

A game pack is a JSON or YAML document loaded and semantically validated when the server starts. The server recursively discovers files named `game.json`, `game.yaml` or `game.yml` under `GAME_PACKS_PATH`; every pack uses the same engine.

## Top-level fields

- `id`: stable game identifier.
- `title`: public title.
- `description`: profile-selection introduction.
- `version`: scenario version.
- `generated`: server-owned provenance for composed packs; omit this in authored packs.
- `mission`: objective and legacy progress compatibility fields.
- `citizens`: selectable player contexts.
- `actions`: decisions, requirements, costs and outcomes.

Schema-v2 packs also define:

- `schema_version`: use `2` for the reusable environment format; omitted legacy packs are schema 1.
- `environment`: world region, government level, political system, administrative/corruption/legal/media/economic context and modifiers.
- `value_definitions`: arbitrary public resources, metrics, relationships and skills with labels, bounds and display formats.
- `hidden_variable_definitions`: server-only ranges sampled for each new session.
- `institutions`, `stakeholders` and `barriers`: reusable civic context and validated references.
- `random_events`: deterministic per-turn event chances, conditions, effects and visual cues.
- `endings`: ordered declarative won/lost conditions. When endings exist, the engine does not apply a generic progress-only win.
- `visual_theme`: colors, layout and institution/location coordinates used by the shared clients.
- `campaign`: optional declarative import/export rules that connect this mission to a cross-mission civic identity.

## Public metrics

These metrics may be used in requirements and weighted conditions:

- `progress`
- `documentation`
- `community_support`
- `public_attention`
- `integrity`
- `money`
- `energy`
- `influence`
- `days_remaining`

Schema-v2 actions and conditions may use any ID declared by `value_definitions`, a profile's starting values/skills/relationships, an effect, or a persistent consequence. The browser and Expo clients render declared public values dynamically.

## Hidden metrics

Each new session generates these values from 0–100 ranges:

- `departmental_backlog`
- `officer_integrity`
- `election_pressure`
- `corruption_pressure`

Players do not receive these values. They influence outcome weights.

Schema-v2 packs should declare scenario-specific hidden variables instead of relying on these four legacy names. For example, the examination pack declares `minister_support`, `coalition_cohesion`, `police_tolerance`, `leak_credibility` and `election_proximity`.

## Action model

```json
{
  "id": "file_complaint",
  "title": "File a formal complaint",
  "description": "Submit a written complaint.",
  "cost": {
    "money": 250,
    "energy": 3,
    "influence": 0,
    "days": 3,
    "values": {}
  },
  "guaranteed_effect": {
    "documentation": 8,
    "progress": 3,
    "values": {},
    "consequences": {}
  },
  "requirements": [],
  "max_uses": 1,
  "outcomes": []
}
```

## Requirement model

```json
{
  "metric": "documentation",
  "min": 45,
  "message": "A court filing needs a stronger documentary record."
}
```

`min` and `max` are optional. All requirements must pass. The server returns only actions whose requirements, resources and use limit currently pass, and it repeats the same checks when an action is submitted.

Actions are one-shot by default, and `max_uses` may only be omitted or set to `1`; changed circumstances must be modeled as a new action with explicit state requirements. This prevents repeat limits from becoming a second route to action spamming. For example, the examination scenario moves `evidence_stage` through unverified, independently verified, corroborated, chain-of-custody confirmed and legally admissible states. Each transition is a distinct action; old evidence cannot be verified repeatedly for more progress.

## Outcome model

```json
{
  "id": "registered",
  "message": "The complaint is registered.",
  "base_weight": 42,
  "progress_min": 7,
  "progress_max": 12,
  "effect": {
    "public_attention": 3,
    "resources": {
      "days_remaining": -2
    }
  },
  "conditions": [
    {
      "metric": "documentation",
      "min": 15,
      "multiplier": 1.35
    }
  ]
}
```

The engine multiplies `base_weight` by every matching condition. It then samples one outcome proportionally to the resulting weights.

## Declarative endings

Endings are evaluated in array order after the selected outcome and any random event. All conditions in one ending must match.

```json
{
  "id": "binding_inquiry",
  "title": "Independent inquiry",
  "message": "Parliament compels an independent inquiry.",
  "status": "won",
  "conditions": [
    { "metric": "evidence_stage", "min": 4 },
    { "metric": "documentation", "min": 70 },
    { "metric": "community_support", "min": 50 },
    { "metric": "institutional_pressure", "min": 55 }
  ]
}
```

Put more specific endings before broad endings. Each ending must contain at least one condition and use `won` or `lost` status. In schema-v2 packs, every winning ending must require at least two distinct non-progress factors; generic progress alone cannot win a mission.

## Random events

Random events use the session seed and turn, so repeated debugging runs remain deterministic. At most one random event fires after an action. Event effects can set a state value that unlocks a context-specific action. A whistleblower event, for example, sets `whistleblower_packet_available`; only then does `verify_whistleblower_evidence` enter the public action list.

```json
{
  "id": "budget_reallocation",
  "title": "Emergency budget reallocation",
  "message": "Funding is diverted to another emergency.",
  "chance_per_turn": 0.06,
  "once": true,
  "conditions": [{ "metric": "progress", "min": 20, "max": 75 }],
  "effect": { "resources": { "days_remaining": -12 }, "progress": -4 }
}
```

## Visual themes

Rendering code is scenario-neutral. Packs provide colors and a list of reusable location nodes; actions reference a location ID, and outcomes/events may reference it in a `visual_event`.

```json
{
  "visual_theme": {
    "id": "federal_capital_campaign",
    "layout": "civic_city_map",
    "primary_color": "#312e5f",
    "accent_color": "#d94f70",
    "background_color": "#f1eef8",
    "locations": [
      { "id": "parliament", "label": "Federal Assembly", "institution_id": "parliament", "x": 78, "y": 28 }
    ]
  }
}
```

Coordinates are percentages consumed by both the Phaser city renderer and its DOM fallback. Phaser renders code-native roads, water, civic buildings, player position, separate public factors and server-selected visual events; no simulation rule moves into rendering code.

## Semantic validation

The loader rejects duplicate IDs, unsupported schema versions, unknown condition metrics, missing outcomes, invalid use limits, nonpositive/nonfinite weights, invalid ranges or probabilities, unknown institution/location references, active endings, endings without conditions and schema-v2 wins without multiple non-progress factors. A bad pack prevents startup with the pack path and all validation errors.

Campaign imports must target a known session metric; exports must read a known session metric or declared persistent consequence. Multipliers must be finite and optional clamps must be ordered.

## Campaign transfers

Campaign rules keep inheritance scenario-owned and data-driven:

```json
{
  "campaign": {
    "imports": [
      { "source_id": "civic_reputation", "target_id": "community_support", "multiplier": 0.25, "min": -15, "max": 15 }
    ],
    "exports": [
      { "source_id": "integrity", "target_id": "civic_reputation", "multiplier": 0.15, "source_offset": 50, "min": -8, "max": 8 }
    ],
    "won_effect": { "civic_reputation": 8 },
    "lost_effect": { "civic_reputation": -3 }
  }
}
```

The transfer formula is `(source - source_offset) * multiplier`, rounded and then clamped. Imports add the result to a new session. Exports add it to campaign memory after a terminal result. `won_effect` and `lost_effect` are direct campaign deltas. A pack may omit all campaign rules and will still contribute its full event history when played inside a campaign.

## Validate a modified pack

Python is sufficient for JSON syntax validation:

```bash
python -m json.tool my-new-game-pack/game.json > /dev/null
```

YAML packs use the identical strongly typed model and semantic validator. Name the entry file `game.yaml` or `game.yml`; do not put both a JSON and YAML entry with the same pack ID below the discovery root.

The server also fails fast with descriptive parsing and semantic-validation errors. Run `cargo test` to validate all three starter packs and the YAML loading fixture, and `cargo run --bin simulate -- 250` to inspect balance and ending reachability.

## Abstraction catalog and generation

`game-packs/abstractions.json` is deliberately separate from scenario instances. It defines selectable category options, difficulty overlays, event modifiers, regional palettes and objective-to-template mappings. The browser reads it through `GET /api/v1/scenario-generator`; generated packs are created through `POST /api/v1/scenarios/generate` and then use the same public-scenario, session and action APIs as authored packs.

An explicit request can provide any subset of category selections. With `randomize_unspecified: true`, all missing dimensions and an empty modifier list are filled from the seeded catalog. Reusing the same seed and selections produces the same pack ID and content. Only objective types with an installed template can be generated; adding new catalog values or effects requires no engine change, while a genuinely new objective graph needs a normal playable pack plus a template mapping.

Catalog and template `constraints` use `when` and `require` maps of category IDs to allowed option IDs. An explicitly contradictory request is rejected with the rule's message. If the incompatible target field was randomized, the seeded generator repairs it from the allowed set. Template `limits` bound composed resource, cost and event multipliers so difficult environment combinations remain reachable without erasing their mechanical differences.
