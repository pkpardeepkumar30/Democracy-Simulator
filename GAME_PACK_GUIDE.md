# Game-pack guide

A game pack is a JSON document loaded when the server starts.

## Top-level fields

- `id`: stable game identifier.
- `title`: public title.
- `description`: profile-selection introduction.
- `version`: scenario version.
- `mission`: objective and winning progress.
- `citizens`: selectable player contexts.
- `actions`: decisions, requirements, costs and outcomes.

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

## Hidden metrics

Each new session generates these values from 0–100 ranges:

- `departmental_backlog`
- `officer_integrity`
- `election_pressure`
- `corruption_pressure`

Players do not receive these values. They influence outcome weights.

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
    "days": 3
  },
  "guaranteed_effect": {
    "documentation": 8,
    "progress": 3
  },
  "requirements": [],
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

`min` and `max` are optional. All requirements must pass.

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

## Validate a modified pack

Python is sufficient for JSON syntax validation:

```bash
python -m json.tool my-new-game-pack/game.json > /dev/null
```

The server also fails fast with a descriptive parsing error if the schema does not match its Rust structures.
