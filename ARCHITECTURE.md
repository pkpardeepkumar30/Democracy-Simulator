# Architecture

## Runtime

The production Docker image contains one compiled Rust binary and a discoverable game-pack library. The binary embeds the browser HTML, CSS, JavaScript, web manifest and service worker at compile time; packs remain data files so new environments can be mounted without recompiling the engine.

```text
Browser/PWA or Expo client
          |
       HTTP JSON
          |
Rust Axum scenario catalog/API
       /                  \
typed abstraction       validated pack registry
catalog + generator     + generic transition/event engine
       \                  /
atomic generated-pack and session stores on /data
```

## Latency-sensitive path

One player action uses one API request and one locked state transition:

1. Resolve the session's pack and validate generic action requirements.
2. Deduct the declared action cost.
3. Calculate weighted stochastic outcomes using the session seed and turn.
4. Apply generic outcome effects and persistent consequences.
5. Evaluate and apply at most one deterministic random world event.
6. Resolve the pack's declarative ending conditions.
7. Append action/world events.
8. Persist an atomic snapshot.
9. Return the complete public state, generic indicators and the exact per-factor deltas for outcome feedback.

There are no sequential client calls for cost deduction, probability selection or state refresh.

## Determinism and uncertainty

A session receives a random 64-bit seed and hidden variables generated from pack-declared ranges. Each turn derives its pseudo-random stream from the session seed and turn number. This permits deterministic debugging while keeping future outcomes hidden from clients.

The server stores each result by `client_action_id`. Repeating the same identifier returns the original response without charging the player twice.

## Storage

The MVP writes sessions to `/data/sessions.json`, campaigns to `/data/campaigns.json` and generated game packs to `/data/generated-packs.json`, all through atomic temporary-file renames. Docker mounts `/data` as the named volume `civic_game_data`, so generated scenarios and cross-mission histories remain resolvable after restart. Startup reconciles terminal campaign-linked sessions idempotently in case a process stopped between the session and campaign snapshots.

For a public service, replace `SessionStore` with a PostgreSQL implementation and keep the engine unchanged.

## Clients

- `web/`: responsive PWA with an accessible DOM application and a locally bundled Phaser 3 city scene, embedded into the Rust executable.
- `apps/mobile/`: Expo/React Native client consuming the same scenario catalog and game API.

## Game-pack registry

`GAME_PACKS_PATH` is recursively scanned for files named `game.json`, `game.yaml` or `game.yml`. Every pack is strongly deserialized and semantically validated before the server starts. IDs, references, metrics, ranges, outcome weights, visual locations, random-event probabilities and endings are checked. Sessions store pack ID/version and are always advanced using their originating pack.

Schema 1 remains readable for compatibility. Schema 2 adds environment abstractions, arbitrary public/hidden values, institutions, stakeholders, procedural barriers, random events, declarative endings and a visual theme. The Rust engine contains no scenario-specific win text or metric list for schema-v2 behavior.

The Rust server remains authoritative for competitive or multiplayer use. A later offline version can expose the pure Rust engine through native bindings and store local games in SQLite.

## Visual city boundary

The pinned Node builder compiles `web/city-src` into `web/dist/city.bundle.js`; the Rust binary serves that bundle locally. `toCityViewModel` maps only public scenario/state DTOs into palette, locations, separate civic factors, player marker and interactive hotspots. Phaser never fetches the API, calculates availability, applies costs or predicts outcomes. The server omits unavailable actions; browser, Phaser and Expo additionally filter on the public `enabled` flag as a compatibility safeguard.

The DOM controller creates and destroys one renderer with each confirmed state snapshot. A canvas location may emit only an already enabled action ID. The resulting server `visual_event` drives a bounded focus/flash/caption animation. If Phaser is missing or throws during initialization, the scenario-neutral DOM location map is revealed; the persistent DOM action list remains the keyboard and screen-reader equivalent in either mode. Reduced-motion preferences disable repeating and focus tweens.

## Scenario composition

`game-packs/abstractions.json` stores reusable regions, political and administrative dimensions, player roles, objective categories, modifiers, difficulty overlays, palettes and mappings to playable template packs. `POST /api/v1/scenarios/generate` fills unspecified dimensions with a seeded ChaCha8 stream, composes numeric effects, clones a compatible validated template, assigns a stable content identity and validates the result as an ordinary `GamePack`.

Catalog/template constraints reject explicit incoherence and deterministically repair only randomized fields. Per-objective effect limits prevent composed scarcity/cost/event multipliers from making an installed action graph structurally unreachable; the Monte Carlo command can exercise generated objective/difficulty pairs directly.

The generator changes framing, starting capacity, costs, disruption probabilities, hidden ranges and palette while reusing the template's institutions, actions, outcome graphs, events and endings. This is intentionally compositional: the engine has no generator-specific action path. Installed packs that are not referenced by an available template still load normally; the catalog exposes only templates present in the active registry.

## Cross-mission campaigns

Campaigns are optional identities exposed through `/api/v1/campaigns`. A session stores its campaign ID and reset-attempt number. When a linked mission ends, the server records the complete event history exactly once and applies the pack's declarative `campaign.exports`, plus won/lost effects, to the campaign value map.

On a later session, `campaign.imports` transform those values into bounded deltas on ordinary pack metrics. The starter packs currently share `civic_reputation`, `institutional_knowledge` and `civic_network`; Factory Ground also exports community partnership and broker dependency. Transfer source/target metrics and numeric bounds are validated with the rest of each pack, so the campaign store knows nothing about a particular political system or objective.
