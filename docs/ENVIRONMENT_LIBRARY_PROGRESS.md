# Environment library implementation status

This document tracks implementation against `DEMOCRACY_SIMULATION_ENVIRONMENT_LIBRARY.txt`. It is deliberately evidence-based: “implemented” means the current code and tests demonstrate the capability; it does not mean a plausible design exists.

## Delivered shared architecture

- `src/lib.rs` exposes reusable engine, model, pack-registry, generator and persistence modules. The HTTP server and Monte Carlo executable use the same library.
- Schema-v2 packs define environment profiles, arbitrary public values, hidden-variable ranges, institutions, stakeholders, barriers, random events, declarative endings, persistent consequences and visual themes.
- `src/game_pack.rs` discovers `game.json`, `game.yaml` and `game.yml` below `GAME_PACKS_PATH` and rejects incoherent IDs, references, metrics, ranges, probabilities, outcomes and endings before startup.
- `src/engine.rs` evaluates generic values instead of a scenario-specific metric list for schema-v2 behavior. It keeps probability and hidden state server-side, deterministically applies outcomes/world events, records history, resolves declarative endings and detects sessions with no remaining action. Actions are one-shot state transitions by default; consumed and otherwise invalid actions are omitted from public state and rejected if submitted directly.
- Sessions retain their game-pack ID/version and survive server/container restarts in the shared JSON store.
- `/api/v1/scenarios` lists environments and `/api/v1/scenarios/{id}` returns one public environment. Existing `/api/v1/scenario` and `citizen_id` requests remain compatible.
- `game-packs/abstractions.json` and `src/generator.rs` provide typed, seeded composition across 12 categories, three difficulty models, 17 modifiers and three playable objective templates. Generated packs receive stable IDs, pass the normal validator and persist across restarts.
- `src/campaign.rs` provides optional cross-mission identity, full completed-mission histories, idempotent completion export and pack-declared inheritance into later sessions. The browser can create/select campaigns; one-off play remains compatible.
- The browser and Expo clients select from the same catalog and render pack-declared public indicators. The browser uses one scenario-neutral civic-map component driven by theme/location/action metadata.
- `src/bin/simulate.rs` runs deterministic goal-directed Monte Carlo batches or traces one seed turn by turn.

## Three starter environments

| Environment | Profiles | Actions | Shared systems exercised | Observed winning endings |
| --- | ---: | ---: | --- | --- |
| The Flooded Ward (`civic-drainage-v1`) | 3 | 8 | Public service, evidence, resident coalition, media, court, corruption trade-off, rain/budget events | Full repair, court order, temporary cleanup |
| The Examination Scandal (`examination-scandal-v1`) | 3 | 12 | Staged evidence verification, coalition building, protest, media/disinformation, parliament, court, crackdown risk and whistleblower-specific evidence | Minister resignation, binding inquiry, fair-exam remedy |
| Factory Ground (`factory-ground-v1`) | 3 | 11 | Business planning, finance, land/title, community negotiation, zoning, environment, utilities, tax, broker trade-off | Community-partnered factory, factory opens |

A 250-seed Monte Carlo run observed at least one win for every starter profile. This establishes reachable endings under the included automated policy, not final balance quality. Factory Ground currently has the narrowest automated win range and should receive additional human playtesting.

## Requirement audit

| Specification requirement | Status | Current evidence / gap |
| --- | --- | --- |
| Reusable platform, not hard-coded drainage | Implemented foundation | Multi-pack registry and generic engine load three structurally different packs. |
| Store abstractions separately from instances | Implemented foundation | A typed catalog is separate from authored/generated packs and `GameSession`; generation records its catalog selections as provenance. |
| JSON or YAML plus strong validation | Implemented foundation | JSON/YAML share the same typed deserializer and semantic validator; an executable YAML fixture is included. A machine-readable JSON Schema is still desirable. |
| Region, political environment, role and category selection | Implemented foundation | Browser composer exposes region, political/administrative/corruption/legal/media dimensions, role, objective and modifiers; unspecified dimensions are randomized. Expo exposes random composition and difficulty. |
| Difficulty selection | Implemented foundation | Accessible, standard and hard overlays compose resource, cost, event and hidden-range effects into generated packs. More human balance testing is needed. |
| Random-world option | Implemented foundation | Seeded server generation, stable identities, normal pack validation, API/client flows and restart-safe generated-pack persistence are implemented. |
| Visuals generated from theme packs | Implemented foundation | Pack/generated palettes, locations, player starts, progress and visual events drive one locally bundled Phaser scene; code-native visuals require no external assets and a DOM fallback remains. External pack-art validation/serving is not implemented. |
| Hidden probabilities and variables server-side | Implemented | Public DTOs omit outcome tables and hidden values; server performs seeded selection. |
| Persistent consequences | Implemented foundation | Session consequences persist; campaign export rules can transform metrics or declared consequences into long-lived civic values. |
| Preserve game history across missions | Implemented foundation | Optional campaigns store each terminal mission's full event list, result, ending, role, version, exports and resulting campaign values. |
| Earlier choices affect later scenarios | Implemented foundation | Pack-owned bounded import/export mappings carry reputation, knowledge, networks and scenario consequences into later starting state. |
| Scenario validator | Implemented foundation | Loader validation plus 23 Rust tests cover all starters, one-shot action rules, per-factor outcome feedback, multi-factor wins, YAML loading, deterministic/coherent generation, mechanical overlays, store safety and campaign persistence/idempotency. JSON Schema/authoring CLI output is still desirable. |
| Balancing/Monte Carlo tool | Implemented foundation | `cargo run --bin simulate -- N` plus deterministic `trace` mode. It needs configurable strategies and CI thresholds. |
| Three complete starter environments | Playable vertical slice | All three load, create/advance/persist sessions, expose all actions/events/endings and have observed winning routes. Browser-based full manual completion of every ending remains unverified. |
| Same simulation and event systems | Implemented | One `apply_action`, random-event pipeline and generic state model serve every pack. |
| Same persistence | Implemented foundation | Authored/generated packs share session routing; session, generated-pack and campaign stores survived verified container restarts. Mutations are serialized and corrupt JSON fails visibly; explicit store migration fixtures remain desirable. |
| Same visual framework and animation system | Implemented foundation | Every pack uses the same Phaser scene/view-model and DOM fallback. Server visual events drive focus/flash/caption animation with reduced-motion support. External art/audio delivery remains optional future work. |
| Same scenario-generation architecture | Implemented foundation | One catalog-driven generator produces normal validated packs for all three objective graphs; no generator branch exists in the turn engine. Coherence rules are currently template-level rather than a richer constraint language. |
| Add future scenarios without engine changes | Implemented for authored JSON/YAML packs | A new validated schema-v2 pack is discovered without Rust changes. Catalog options/effects are data-only; a new objective graph needs a playable template mapping. |

## Verification performed

- `cargo fmt --check`: passed after formatting the Rust codebase.
- Rust 1.88 tests: 23 passed, zero failed, including replay rejection, per-factor response deltas, hidden/unlocked actions, evidence-stage progression, multi-factor victories, legacy-save loading, concurrent session writes, corrupt-store handling, YAML loading, generator coherence and campaign idempotency/reload.
- Pack registry: all three starter packs parse and pass semantic validation.
- JavaScript syntax: `web/app.js` and `web/offline-api.js` pass `node --check`.
- Expo strict TypeScript: `npm run typecheck` passed with the installed locked dependency graph.
- Docker: the final locked multi-stage production image passed its Node typecheck/unit/build stages and Rust release build; `docker compose config --quiet` passed.
- Corrected runtime smoke: health, default scenario, three-scenario catalog, session creation, action and duplicate-action idempotency passed.
- Anti-spam API acceptance on the exact release image: the initial drainage response exposed only two valid actions; filing a complaint removed that action and unlocked the office/RTI actions; a new-ID replay returned HTTP 400 without advancing turn 1. The examination evidence chain advanced through stages 1, 2, 3 and 4, hiding each consumed action and exposing only its next valid transition.
- API vertical slice: one profile/session/action succeeded in each of the three scenarios through the live server.
- Persistence: an examination-scandal session retained its pack, turn and event history across a real container restart using the same temporary volume.
- Generated-world API/restart: an explicit hard Factory Ground composition persisted as a fourth pack; after container restart its public scenario and session loaded and advanced to turn 1.
- Campaign API/restart: a linked drainage mission won in 10 turns and stored all 10 events. A later examination mission inherited +4 support, +2 documentation and +1 influence; the history/value map survived container restart unchanged.
- Generation coherence: explicit incompatible ministerial/village scale is rejected; randomized fields are deterministically repaired to template-compatible values.
- Generated hard-mode Monte Carlo (250 runs each, one seeded composition per objective) retained reachable wins: infrastructure 20, accountability 66, business/land 2. Template-owned effect limits were added after the first balance run found two unreachable hard compositions.
- Phaser module: strict TypeScript, two pure view-model tests, reproducible Vite build and `npm audit` with zero findings passed.
- Browser E2E: headless Edge rendered the exact production image at 735×328 desktop and 332×278 mobile canvas sizes, kept accessible DOM actions, hid the fallback only after Phaser initialized, clicked a canvas location, advanced the server session and displayed the result dialog without browser errors. The same Phaser renderer then loaded all three authored environments and a generated Factory Ground environment. Screenshots were visually inspected.
- Standalone browser E2E: `PLAY_WITHOUT_DOCKER.html` is now generated from the current shell, drainage pack, offline engine and application controller. Headless Edge confirmed exactly two initial valid actions, no disabled unavailable buttons, removal of the consumed complaint action and appearance of the newly unlocked office action.
- Final release-image API/restart: all three authored environments plus a deterministic generated environment created and advanced sessions; an incompatible accountability/village request returned HTTP 400; a linked drainage mission reached `court_order` in 10 turns and exported campaign values. After recreating the container on the same volume, its generated pack, terminal session, full campaign mission and values were intact, and a new examination mission inherited +4 community support, +2 documentation and +1 influence.
- Monte Carlo: 250 deterministic runs per profile reported reachable multi-factor wins for every authored profile after the one-shot conversion. Hard generated compositions also remained reachable: infrastructure 143 wins, accountability 10 and business/land 7.

`npm audit --omit=dev` reports 11 moderate transitive vulnerabilities in the Expo 53 toolchain. The available automated remediation is a breaking upgrade to Expo 57, so this must be handled as a tested mobile-upgrade task rather than with `npm audit fix --force`.

## Next implementation sequence

1. Add optional external pack-art validation/serving and a generated standalone visual bundle; bring the offline engine to full schema-v2 event/ending parity or replace it with a shared compiled engine.
2. Add explicit format migrations and legacy-save fixtures before the next incompatible store change.
3. Expand browser automation with accessibility audits, add mobile component/device tests and enforce CI balance thresholds.
4. Add a machine-readable JSON Schema and authoring command after the schema-v2 contract stabilizes.
