# Visual City implementation plan

## Planning basis and status

This plan is based on a complete inspection of the repository at commit `181eba5` on `main`.

The requested input file, `docs/VISUAL_CITY_UPGRADE_SPEC.md`, is not present in the working tree, Git history, or the only branch advertised by `origin`. There is currently no `docs/` directory. Therefore, this is a repository-derived plan rather than a claim of compliance with that specification. Before implementation starts, the missing specification must be restored and reconciled with the assumptions below, especially its required art style, map interactions, animation list, browser/device targets, and mobile scope.

The central recommendation is to add Phaser as a visual presentation layer inside the existing browser application. The Rust engine, session API, stochastic rules, persistence, DOM-based HUD, custom-game-pack mechanism, and Expo client should be extended rather than replaced.

Implementation update: the repository has since implemented the central boundary in this plan. A pinned Phaser 3/Vite/TypeScript module consumes the existing public theme/location/action/visual-event contract, the production Docker build runs its typecheck/tests/build, the Rust binary serves the local bundle, and the DOM city/actions remain the fallback and accessible equivalent. Headless browser verification against the final production image exercised canvas interaction, outcome display and responsive layout, then loaded all three authored environments and one generated environment through the same renderer without fallback or browser errors. The larger external asset schema/route described later in this document remains a future option because the current scene generates theme visuals in code and needs no pack files. Current platform-wide evidence is maintained in `docs/ENVIRONMENT_LIBRARY_PROGRESS.md`.

Action-integrity update: the server now returns only actions that pass current state, resource and one-shot-use rules; direct submissions are checked against the same rules. Phaser, the DOM client, Expo and the generated single-file client filter the public list defensively and never render unavailable actions as disabled choices. The starter packs use separate civic factors, staged evidence transitions and multi-factor endings instead of a repeatable generic progress route. The earlier baseline observations below are retained as historical repository findings, not descriptions of the current implementation.

## 1. Current architecture

### 1.1 Runtime overview

```text
Browser/PWA                         Expo/React Native
  web/index.html + app.js             apps/mobile/app/index.tsx
  optional offline JS engine          server API only
            \                              /
             \-------- HTTP JSON --------/
                          |
                    Axum routes
                    src/main.rs
                          |
              pure state-transition engine
                    src/engine.rs
                          |
             in-memory RwLock<HashMap>
                          |
          atomic JSON snapshot at SESSION_STORE_PATH
```

The production image is a single Rust service plus the selected game pack. The browser shell is compiled into the Rust binary with `include_str!`; the Expo application is developed and packaged separately.

### 1.2 Rust backend

- `src/main.rs` loads exactly one `GamePack` from `GAME_PACK_PATH`, loads a `SessionStore`, constructs the Axum router, and listens on `0.0.0.0:$PORT` (default `8080`). CORS is permissive and HTTP tracing is enabled.
- Current routes are:

  | Method | Route | Purpose |
  | --- | --- | --- |
  | `GET` | `/` | Embedded browser application |
  | `GET` | `/app.js`, `/styles.css`, `/manifest.webmanifest`, `/sw.js` | Embedded PWA assets |
  | `GET` | `/api/v1/health` | Health, server version, pack ID, and session count |
  | `GET` | `/api/v1/scenario` | Public mission and citizen-selection data |
  | `POST` | `/api/v1/sessions` | Create a seeded session for a citizen |
  | `GET` | `/api/v1/sessions/{id}` | Restore the complete public state |
  | `POST` | `/api/v1/sessions/{id}/actions` | Apply one authoritative action transition |
  | `POST` | `/api/v1/sessions/{id}/reset` | Recreate the session with the same citizen and a new seed |

- `src/model.rs` contains both internal persistence models and public API DTOs. Serde defaults make many numeric pack fields optional, but there is no explicit pack-schema version or semantic validation pass.
- `src/engine.rs` creates the hidden administrative context, checks costs and requirements, applies guaranteed effects, selects a weighted outcome using a turn-derived `ChaCha8Rng`, clamps state, resolves win/loss, and records an event. The server is the source of truth.
- A `client_action_id` indexes stored `ActionResponse` values. Reusing the same ID returns the original response without charging resources or advancing the turn again.
- Citizen `modifiers` are parsed and exposed by the scenario response but are not currently used by the engine.
- Unknown requirement/condition metric names silently evaluate to `0.0`; invalid packs can therefore load with unintended behavior.
- `src/static_assets.rs` has one handler per embedded text asset. It cannot currently serve binary art, nested asset paths, or a generated bundle without additional work.

### 1.3 Browser frontend and offline copy

- `web/index.html`, `web/app.js`, and `web/styles.css` form a dependency-free, responsive DOM application. `app.js` owns API calls, local session-ID storage, complete-page rendering, action request locking, outcome dialogs, session reset, and restore-on-load.
- The server always returns complete public state, including action availability and disabled reasons. The client does not calculate authoritative availability or outcomes.
- `web/sw.js` implements a small network-first PWA shell cache. It caches only `/`, CSS, JavaScript, and the manifest, and deliberately ignores `/api/` requests.
- `web/offline-api.js` is a separate JavaScript reimplementation of the Rust rules. It replaces `globalThis.fetch`, persists sessions in browser storage, and is used only by `PLAY_WITHOUT_DOCKER.html`.
- `PLAY_WITHOUT_DOCKER.html` is a 1,304-line self-contained copy. Its embedded CSS, game pack, offline engine, and application JavaScript exactly match the corresponding source files, but no checked-in generator or drift test exists.
- `offline-preview.png` shows the current phone layout and outcome modal. The UI is cards, action buttons, meters, and history; there is no canvas, map, sprite, asset pipeline, or Phaser dependency.

### 1.4 Game-pack format

`game-packs/drainage/game.json` is the only pack. It is `civic-drainage-v1`, content version `1.0.0`, with one mission, three citizens, eight actions, and 24 weighted outcomes.

Current top-level structure:

```json
{
  "id": "civic-drainage-v1",
  "title": "...",
  "description": "...",
  "version": "1.0.0",
  "mission": { "title": "...", "objective": "...", "starting_status": "...", "win_progress": 100 },
  "citizens": [],
  "actions": []
}
```

- Citizens define identity/context, starting resources, starting public metrics, and currently unused numeric modifiers.
- Actions define display text, resource cost, guaranteed state delta, zero or more metric requirements, and one or more weighted outcomes.
- Outcomes define an ID, message, base weight, a random progress range, state effects, and conditional weight multipliers.
- Conditions can use public resources/metrics or four hidden metrics. Hidden values are never sent to clients.
- `GAME_PACK_GUIDE.md` documents syntax and engine behavior. Validation is presently limited to Serde shape checking at startup; IDs, references, weight ranges, asset safety, duplicate IDs, and metric names are not validated.
- `PublicScenario` intentionally omits actions and outcomes. `PublicGameState.available_actions` exposes only the current action ID, text, cost, enabled flag, and disabled reason.

### 1.5 Persistence

- `src/store.rs` holds all sessions in an `Arc<RwLock<HashMap<String, GameSession>>>`.
- Every insert or successful action serializes the entire map as `{ "sessions": { ... } }`, writes a sibling `.tmp` file, and renames it over `SESSION_STORE_PATH`.
- Docker maps the default `/data/sessions.json` path to the named `civic_game_data` volume.
- There is no store format version, migration mechanism, per-session pack version, backup, test suite, or database transaction boundary.
- Malformed JSON is handled with `unwrap_or_default()`, which silently starts with an empty store. Concurrent `persist()` calls also share one temporary filename after releasing the mutation lock, so concurrency behavior needs explicit testing before load increases.
- Persisted sessions copy mission/citizen text and retain hidden state, seed, events, and idempotency responses. Action availability on restore is calculated from the currently loaded pack, not a stored pack version.

### 1.6 Docker and local operations

- `Dockerfile` uses `rust:1.88-bookworm` to build the binary and `debian:bookworm-slim` at runtime. The runtime installs `curl`, runs as UID `10001`, exposes port `8080`, and declares an API health check.
- The build attempts `cargo build --release --locked` and falls back to unlocked resolution. No `Cargo.lock` is committed, so builds are not reproducible even though `--locked` appears in the file.
- The runtime image copies only `game-packs/drainage/game.json`, not a pack directory containing visual assets.
- `compose.yaml` runs one container and mounts the named save volume. `compose.custom-pack.example.yaml` mounts an arbitrary pack directory read-only and points `GAME_PACK_PATH` at its JSON file.
- Windows batch files and small shell scripts provide start/stop/log/reset operations. `Makefile` delegates its test target to the running-server smoke script.

### 1.7 Expo/mobile client

- `apps/mobile` is Expo 53, Expo Router 5, React 19, React Native 0.79, and strict TypeScript 5.8.
- `_layout.tsx` configures a headerless stack. `index.tsx` contains all API types, networking, state, screens, and styles in one file.
- It supports scenario loading, citizen selection, action submission, resource/metric display, end state, and history.
- It does not currently persist or restore a session ID, call health/reset, or provide offline play.
- It points to `EXPO_PUBLIC_API_URL` and otherwise defaults to `http://localhost:8080`. App-store identifiers remain examples.
- Phaser targets browser Canvas/WebGL and should not be imported directly into the native React Native render tree. The mobile client must continue to work by ignoring optional visual fields; reuse of the Phaser view in native apps should be a later, explicit WebView decision.

### 1.8 Existing tests and observed results

The only automated Rust tests are two unit tests in `src/engine.rs`. There are no checked-in API integration, store, browser, service-worker, schema, Docker, or mobile component tests.

Results from this inspection:

| Check | Result |
| --- | --- |
| `cargo fmt --check` on the host | **Failed** on pre-existing formatting differences in all four Rust source areas; no files were reformatted. |
| `cargo test` on the host | **Could not compile**. With no lockfile, Cargo 1.79 resolved `getrandom 0.4.3`, whose manifest requires edition-2024 support unavailable in that Cargo version. Generated artifacts were removed afterward. |
| `docker build --target builder ...` | **Passed** using the repository's Rust 1.88 builder. |
| `cargo test` in the Rust 1.88 builder | **Passed: 2 passed, 0 failed**. Determinism and duplicate-action idempotency tests passed. |
| `docker build` for the runtime image | **Passed**. |
| `docker compose config --quiet` | **Passed**. |
| `scripts/smoke-test.sh` against a temporary runtime container | **Failed due to the script**. Its greedy `sed` expression extracted the last JSON `id` (`informal_payment`) rather than the session ID, so the action request addressed a nonexistent session. |
| Corrected equivalent API smoke flow | **Passed**. Health and scenario loaded; session creation, action submission, duplicate-ID idempotency, and session restore all succeeded. |
| JSON parse and inventory | **Passed**: three citizens, eight actions, and 24 outcomes. |
| Expo `npm run typecheck` | **Not run** because dependencies and a lockfile are absent. Installing them would alter the checkout during a planning-only task. |
| Manual/PWA/device checklist | **Not run**; it requires interactive browser/device testing. |

## 2. Proposed Phaser integration

### 2.1 Responsibility boundary

Phaser should visualize state and collect intent; it must not become another game engine.

```text
DOM shell/HUD ── owns boot, accessibility, profiles, meters, history, errors
      |
      ├── API controller ── owns request locking and /api/v1 calls
      |         |
      |         └── Rust engine/store ── authoritative rules and persistence
      |
      └── Phaser city scene ── renders the latest public state
                 └── emits only selected action/location intent
```

Keep profile selection, resource/metric text, action details, disabled reasons, history, dialogs, and keyboard/screen-reader controls in DOM. Mount a responsive Phaser canvas inside the existing game screen. Clicking a city location selects or invokes one of the already returned `available_actions`; the existing controller posts the action and passes the returned state and visual event back to both DOM and Phaser.

Phaser must never predict costs, roll outcomes, modify progress, reveal hidden conditions, or optimistically commit state. While an action request is in flight, both DOM controls and scene hotspots are disabled. A transport failure restores the last confirmed state.

### 2.2 Frontend structure and lifecycle

1. Boot and fetch health/scenario as today.
2. Load optional `visual_city` metadata and its declared assets. If metadata, WebGL/canvas support, or an asset fails, retain the existing card interface as a fully playable fallback.
3. Create one `Phaser.Game` only after a session exists and the canvas container is visible.
4. Pass immutable public-state snapshots through a small `CityBridge`; do not let the scene fetch APIs directly.
5. Resolve the active progress stage, player sprite, enabled locations, focus target, and one-shot outcome animation from server data.
6. On reset/new game, explicitly reset or destroy the scene to avoid duplicate listeners and GPU resources.
7. On resize/orientation changes, use Phaser Scale Manager with `FIT` and a fixed logical map size, while keeping the DOM HUD responsive.

Recommended scene composition:

- `BootScene`: loader, asset validation, and low-memory/fallback handling.
- `CityScene`: background/tile map, progress-stage overlays, locations, citizen marker, input, camera, and reduced-motion behavior.
- DOM overlays rather than a Phaser UI scene for text-heavy controls and accessible equivalents.
- A declarative adapter that maps `PublicGameState`, `AvailableAction`, and `visual_event` to scene view models. This adapter should be independently unit tested without Phaser.

### 2.3 Build and asset delivery

Add a small, pinned web build rather than loading Phaser from a CDN. A committed npm lockfile, Phaser dependency, TypeScript, and Vite provide reproducible bundling and allow pure view-model tests. Port the current `app.js` behavior incrementally; do not redesign the whole application in the same phase.

Build `web/src` to `web/dist`, then recursively embed the generated browser shell in the Rust binary (for example with `rust-embed` plus MIME detection). Keep game-pack art outside that bundle so custom packs remain data-driven. Serve assets relative to the directory containing the active `game.json` through a validated read-only route. The default Docker image must copy the complete default pack directory.

The Docker build should gain a pinned Node frontend-builder stage, followed by the current Rust builder. Local developer commands should build the web output explicitly; `cargo build` should not silently install packages or require network access.

The PWA service worker must precache the versioned Phaser bundle and cache declared game-pack assets. The standalone HTML must be generated by a checked-in script from the same sources. It may inline the default pack's bundle/assets as data URLs, or it must be explicitly retired only if the missing upgrade specification permits that compatibility break.

### 2.4 Mobile strategy

Phase one should leave Expo as a native, text/card client that consumes the additive API unchanged. It can display a compact static city thumbnail or location label, but it should not load Phaser directly.

After the web scene is stable, evaluate a separate Expo route using `react-native-webview` to host the server's visual-city page. That approach reuses Phaser but adds navigation, authentication/origin, offline, message-bridge, and store-review concerns. Do not commit to it until device performance and the missing specification's mobile requirements are known. A native renderer would be a separate implementation sharing the same API/view-model contract.

## 3. Files to create or modify

Names below are proposed and can be adjusted during the specification reconciliation.

### 3.1 Create

| File/path | Purpose |
| --- | --- |
| `web/package.json`, `web/package-lock.json` | Pin Phaser, TypeScript, Vite, and frontend test tooling. |
| `web/tsconfig.json`, `web/vite.config.ts` | Strict module build with stable output suitable for Rust embedding. |
| `web/src/main.ts` | Incremental successor to `web/app.js`; boot and top-level controller. |
| `web/src/api.ts` | Typed API client and error handling. |
| `web/src/types.ts` | Public API and visual-pack contracts shared by DOM and scene adapters. |
| `web/src/city/CityBridge.ts` | One-way state snapshots and scene intent events. |
| `web/src/city/BootScene.ts` | Asset loading and graceful fallback. |
| `web/src/city/CityScene.ts` | Map, locations, citizen marker, overlays, input, and animations. |
| `web/src/city/toCityViewModel.ts` | Pure public-state-to-visual-state mapping. |
| `web/src/accessibility.ts` | Keyboard/reduced-motion/canvas alternatives and focus synchronization. |
| `web/tests/*.test.ts` | Contract, mapping, request-lock, and fallback unit tests. |
| `web/e2e/visual-city.spec.ts` | Playwright API-backed browser flow and responsive checks. |
| `scripts/build-standalone.mjs` | Deterministically regenerate `PLAY_WITHOUT_DOCKER.html`. |
| `game-packs/schema/game-pack.schema.json` | Machine-readable schema for both legacy and visual packs. |
| `game-packs/drainage/assets/city/` | Licensed/owned map, sprites, overlays, atlases, and optional audio. |
| `src/game_pack.rs` | Semantic validation, pack-root resolution, and safe asset lookup. |
| `src/app.rs` and `src/lib.rs` | Testable router construction separated from process startup. |
| `tests/api.rs`, `tests/game_pack.rs`, `tests/store.rs` | API, schema/assets, compatibility, and persistence integration tests. |

### 3.2 Modify

| File/path | Change |
| --- | --- |
| `Cargo.toml` | Pin the resolved Rust graph via a committed `Cargo.lock`; add recursive embedding/MIME support only if selected after a small prototype. |
| `src/model.rs` | Add backward-compatible optional visual schema and public DTO fields. |
| `src/engine.rs` | Propagate only the selected outcome's presentation cue; retain all rules/RNG unchanged. Add pack validation tests around metric/reference use. |
| `src/main.rs` | Delegate router creation and add the safe game-pack asset route. |
| `src/static_assets.rs` | Serve generated files recursively with correct MIME/cache headers rather than one text handler per file. |
| `src/store.rs` | Add store-version compatibility tests and safe migration/error behavior if persisted models gain fields. |
| `web/index.html` | Add the city canvas container, accessible equivalent controls/status, and generated module entry. |
| `web/styles.css` | Integrate responsive canvas/HUD layout, focus states, reduced motion, and fallback styles. |
| `web/app.js` | Preserve behavior during the transition, then remove only after parity in `web/src/main.ts`. |
| `web/sw.js` | Version cache names; precache built assets and safely cache versioned pack art. |
| `web/manifest.webmanifest` | Add real icons if provided by the visual asset work. |
| `web/offline-api.js` | Return new optional public fields and cues; do not alter outcome logic. Long term, generate/share contract fixtures to prevent Rust/JS drift. |
| `PLAY_WITHOUT_DOCKER.html` | Regenerate from sources and inline the default visual bundle/assets if standalone mode remains required. |
| `game-packs/drainage/game.json` | Opt the default scenario into schema version 2 and add visual metadata/references. Keep rules and IDs stable unless the specification explicitly changes them. |
| `Dockerfile` | Add frontend build stage and copy the complete default pack directory, not only `game.json`. |
| `compose.custom-pack.example.yaml` | Document/verify read-only asset-directory mounting; its basic mount shape can remain. |
| `apps/mobile/app/index.tsx` | Move duplicated types/API code out, accept optional new fields, and preserve the native fallback. |
| `apps/mobile/package.json` | Add tests only if mobile behavior changes; commit a lockfile. |
| `README.md`, `ARCHITECTURE.md`, `GAME_PACK_GUIDE.md`, `MANUAL_TEST_CHECKLIST.md` | Document the build, schema, asset licensing, custom visual packs, fallback behavior, and visual/manual checks. |
| `scripts/smoke-test.sh` | Parse JSON robustly instead of using the greedy `sed` expression; assert the final action response and duplicate behavior. |
| `.dockerignore` | Ensure source and required lockfiles enter build contexts while generated frontend output is handled consistently. |

## 4. API changes

All changes should remain under `/api/v1` and be additive. Existing clients must continue to create, restore, act, and reset without sending new fields.

### 4.1 Scenario response

Extend `GET /api/v1/scenario` with:

```json
{
  "schema_version": 2,
  "asset_base_url": "/api/v1/scenario/assets/",
  "visual_city": {
    "logical_width": 1600,
    "logical_height": 900,
    "map": { "image": "city/map.webp" },
    "locations": [],
    "progress_stages": []
  }
}
```

For a legacy pack, return `schema_version: 1`, omit or return `null` for `visual_city`, and keep the current DOM UI. Do not return outcome weights, hidden conditions, or hidden administrative metrics.

### 4.2 Asset route

Add `GET /api/v1/scenario/assets/{*path}` for files beneath the active pack root.

- Canonicalize the pack root once at startup.
- Reject absolute paths, `..`, encoded traversal, symlink escapes, undeclared files, and overly large assets.
- Return correct MIME types, `ETag`, and cache headers keyed to pack ID/version or a content hash.
- Never allow this route to become a general filesystem server.
- A missing optional asset should degrade the browser to the DOM UI; an invalid declared asset should fail pack validation in development/CI.

### 4.3 Session and action DTOs

Add optional fields only:

- `AvailableAction.location_id`: lets DOM and Phaser associate the server-calculated action with a map hotspot.
- `ActionResponse.visual_event`: the selected, non-authoritative presentation cue such as location, animation key, effect key, and duration cap.
- Public event `visual_event`: supports restore/replay without exposing outcome definitions. Prefer deriving it from the persisted `action_id`/`outcome_id` and the session's pack version, or persist it with `#[serde(default)]` if presentation history must survive pack changes.

The request bodies and HTTP status behavior remain unchanged. The API should ignore no unknown security-sensitive fields and should validate bounded string lengths for action/client IDs as part of hardening.

### 4.4 Pack/version visibility

Add `game_pack_id` and `game_pack_version` to public session state, and store the version on new sessions. This makes cache keys, compatibility reporting, and visual-event reconstruction deterministic. Legacy saves need an explicit migration/default policy rather than silent deletion.

## 5. Game-pack schema changes

Add an explicit integer `schema_version`. Existing packs without it are treated as schema 1. Visual packs use schema 2. All presentation fields are optional to maintain legacy and nonvisual custom packs.

Illustrative schema-2 additions:

```json
{
  "schema_version": 2,
  "visual_city": {
    "logical_width": 1600,
    "logical_height": 900,
    "map": {
      "image": "assets/city/map.webp",
      "fallback_image": "assets/city/map-low.webp"
    },
    "locations": [
      {
        "id": "municipal_office",
        "label": "Municipal office",
        "x": 1040,
        "y": 330,
        "hit_radius": 64,
        "icon": "assets/city/office-icon.webp"
      }
    ],
    "progress_stages": [
      {
        "id": "neglected",
        "min_progress": 0,
        "max_progress": 39,
        "overlays": ["assets/city/drain-blocked.webp"]
      }
    ]
  },
  "citizens": [
    {
      "id": "shopkeeper",
      "visual": {
        "sprite": "assets/city/citizen-shopkeeper.webp",
        "start_location_id": "neighbourhood"
      }
    }
  ],
  "actions": [
    {
      "id": "visit_office",
      "location_id": "municipal_office",
      "outcomes": [
        {
          "id": "officer_meeting",
          "visual_event": {
            "focus_location_id": "municipal_office",
            "animation": "inspection-promised",
            "effect": "progress-positive"
          }
        }
      ]
    }
  ]
}
```

Schema rules:

- Paths are pack-root-relative; URLs and absolute filesystem paths are forbidden.
- IDs remain stable strings. Every `location_id`, start/focus location, animation, and asset reference must resolve.
- Location coordinates and hit areas must fall within logical bounds; progress stages must be ordered, nonoverlapping, and cover the intended `0..100` range.
- Texture sizes, atlas frame counts, audio duration, and total pack bytes should be bounded for mobile memory and denial-of-service safety.
- Visual events are presentation only. They cannot contain resource/metric deltas or alter availability.
- The server validates duplicate citizen/action/outcome/location IDs, supported metric names, nonempty outcome sets, finite/nonnegative weights, valid progress ranges, and visual references at startup.
- Schema 1 loads unchanged and produces `visual_city: null`. Schema 2 without usable visual metadata should be rejected with a precise startup error rather than partially running unpredictably.
- `version` remains the content version; `schema_version` identifies structure. Any changed art or metadata that affects clients should increment the content version for cache invalidation.

## 6. Implementation phases

### Phase 0: Recover and reconcile the specification

- Restore `docs/VISUAL_CITY_UPGRADE_SPEC.md` and turn its requirements into acceptance criteria.
- Decide whether Phaser is web/PWA-only or also required inside native packages.
- Confirm art ownership/licensing, map dimensions, required animations/audio, browser floor, performance budgets, and whether single-file offline play remains mandatory.
- Resolve discrepancies in this plan before changing application code.

Exit criterion: signed-off behavior/asset/API checklist with no unresolved compatibility decision.

### Phase 1: Reproducible baseline and contracts

- Commit `Cargo.lock`, a web npm lockfile, and a mobile npm lockfile.
- Fix formatting and the smoke-test parser in separate baseline commits.
- Refactor router construction into testable library code without changing endpoints.
- Add schema 1 fixtures, API snapshots, store fixtures, and semantic pack validation.
- Define schema-2 Rust/TypeScript types and additive DTO fields.

Exit criterion: old pack, old API clients, old save fixture, Docker build, Rust tests, mobile typecheck, and corrected smoke test all pass.

### Phase 2: Frontend build and static/pack asset pipeline

- Add the pinned Vite/TypeScript build and port current `app.js` behavior with pixel/interaction parity.
- Embed `web/dist` recursively and implement secure pack-asset serving.
- Update Docker to build the frontend and copy the full pack.
- Add cache/version behavior and a deterministic standalone generator or an approved deprecation path.

Exit criterion: current nonvisual UI works online, installed as PWA, in the custom-pack compose example, and in the supported offline mode.

### Phase 3: Phaser thin vertical slice

- Mount Phaser only during active/finished sessions.
- Render one map, all locations, one citizen marker, responsive scaling, enabled/disabled highlighting, and location selection.
- Route action intent through the existing controller and animate one returned visual event.
- Implement canvas failure, reduced-motion, keyboard, and DOM-only fallbacks from the start.

Exit criterion: one full complaint action works through a hotspot, refresh restores the same state, duplicate clicks remain idempotent, and the DOM action is equivalently usable.

### Phase 4: Default pack visual content

- Add approved optimized assets, all action/location mappings, citizen sprites, progress stages, and outcome cues.
- Complete schema/reference validation and asset budgets.
- Tune camera, hit targets, animation timing, and outcome-dialog synchronization without changing engine probabilities or costs.

Exit criterion: all eight actions and 24 outcomes have intentional behavior or documented generic fallbacks; win/loss/reset and all three citizens render correctly.

### Phase 5: Offline, accessibility, PWA, and mobile compatibility

- Finish PWA cache invalidation and offline asset behavior.
- Regenerate/test the standalone HTML if retained.
- Add accessible location/action equivalents, focus restoration, high contrast, reduced motion, readable zoom, and screen-reader announcements.
- Update Expo types and test the unchanged native fallback on Android/iOS. Prototype WebView reuse only if required.

Exit criterion: supported desktop/mobile browsers, installed PWA, offline mode, keyboard/screen reader, reduced motion, and Expo fallback pass the agreed matrix.

### Phase 6: Hardening and release

- Run load/concurrency/store-recovery tests and asset traversal/security tests.
- Measure startup, bundle, asset, memory, frame-time, and API latency budgets on low-tier target hardware.
- Run custom legacy and visual packs through CI.
- Update all guides, migration notes, and manual checklist; create before/after screenshots.

Exit criterion: CI, security checks, performance budgets, Docker smoke, persistence restart, and release checklist pass.

## 7. Risks and compatibility concerns

| Risk | Mitigation |
| --- | --- |
| Missing upgrade specification | Treat Phase 0 as a hard gate. Do not infer final art/features/mobile commitments from this plan. |
| Duplicate sources of game truth | Phaser consumes confirmed public state and emits intent only. Keep all costs, requirements, RNG, effects, and end-state resolution in Rust. |
| Legacy/custom packs have no visuals | Make schema-2 visuals opt-in and retain the current DOM interface as a first-class fallback. Test the compose custom-pack path. |
| Existing saves do not record pack version | Add a backward-compatible version field and migration policy before deriving presentation from pack data. Never silently discard incompatible saves. |
| Persisted DTO changes break all saves | Use Serde defaults/additive fields, fixture tests, a store envelope version, and explicit migration/backup behavior. Avoid persisting derived scene state. |
| Phaser bundle and art make startup/offline heavy | Pin/local-bundle dependencies, compress WebP/AVIF where supported, use atlases and lazy loading, define size budgets, and provide a low-memory fallback. |
| Phaser is not a native React Native renderer | Keep Expo compatible via optional fields. Decide separately between native fallback, WebView reuse, or a native renderer. |
| Canvas reduces accessibility | Keep actions/status in semantic DOM, synchronize focus/selection, expose text alternatives, and honor reduced motion/high contrast. |
| Asset endpoint enables path traversal/data exposure | Canonicalize and validate relative declared paths, bound sizes, restrict MIME types, and add encoded/symlink traversal tests. |
| Service-worker caches mix pack versions | Key caches and asset URLs by pack ID/content version or hash; purge old caches on activation; test upgrades. |
| `PLAY_WITHOUT_DOCKER.html` drifts or becomes enormous | Generate and compare it in CI; confirm whether single-file offline support remains a requirement before choosing inlining. |
| Web build complicates the single-binary image | Use a deterministic Node stage and recursively embed only generated output. Keep the runtime one-container shape. |
| No Rust or npm lockfiles | Commit lockfiles and remove unlocked Docker fallback after validating the pinned toolchains. |
| Current store may race concurrent snapshot writes | Hold/serialize persistence correctly or move to a store worker/database before higher concurrency; add stress tests. |
| Current offline JS engine can diverge from Rust | Use shared fixtures/contract tests and keep it scoped to standalone mode; do not let Phaser add a third rules implementation. |
| Art/licensing/provenance uncertainty | Record source, license, attribution, and optimization pipeline for every asset before committing it. |
| WebGL/device failure | Configure Canvas fallback where practical, detect initialization failure, and keep the DOM game fully functional. |

## 8. Test strategy

### 8.1 Rust unit and schema tests

- Preserve seeded determinism and idempotency tests.
- Test every metric name, resource boundary, progress clamp, win/loss precedence, empty/invalid outcomes, finite weights, and duplicate/reference validation.
- Deserialize the current schema-1 pack unchanged and a complete schema-2 fixture.
- Assert that hidden metrics, outcome weights, and hidden conditions never enter public DTOs.
- Assert `location_id`/`visual_event` propagation without changing selected outcomes for fixed seeds.

### 8.2 Store compatibility and concurrency

- Load a fixture written by commit `181eba5`, migrate/default it, act, save, restart, and restore.
- Verify corrupted/truncated files fail visibly and preserve recoverable data rather than silently returning zero sessions.
- Exercise concurrent actions on the same and different sessions; verify no lost updates, temporary-file collisions, or invalid JSON.
- Verify duplicate action IDs before and after restart return the original response and do not charge twice.
- Test pack ID/version mismatch behavior explicitly.

### 8.3 API integration and security

- Build the Axum router in-process and cover health, scenario, create/get/action/reset, bad IDs, finished sessions, malformed JSON, and status/error bodies.
- Snapshot schema-1 and schema-2 JSON to protect Expo and legacy web compatibility.
- Test asset MIME/cache headers, declared/unlisted files, missing files, large files, absolute paths, plain/encoded `..`, separators, symlinks, and pack-root isolation.
- Confirm action requests cannot submit or override any visual/state delta.

### 8.4 Frontend unit and component tests

- Unit-test API serialization, `toCityViewModel`, progress-stage boundaries, action/location matching, unavailable actions, generic visual fallbacks, request locking, reset, and restore.
- Test BootScene failure and reduced-motion branches with Phaser interfaces mocked at the adapter boundary.
- Test that scene intent produces exactly one API request and that only a successful response updates both DOM and scene.
- Contract-test TypeScript fixtures against serialized Rust DTO fixtures.

### 8.5 Browser/PWA tests

- Use Playwright against the real temporary Rust server for profile selection, every action location, disabled reasons, outcome dialog, history, win/loss/reset, refresh restore, and duplicate click suppression.
- Run at agreed desktop, narrow-phone, tablet, touch, keyboard-only, reduced-motion, and forced Canvas/WebGL-failure configurations.
- Assert no horizontal overflow, adequate hit targets, focus visibility/restoration, text alternatives, and screen-reader live status.
- Install/update the service worker, go offline after first load, restore a session/shell as specified, and verify a pack-version cache upgrade.
- Capture deterministic screenshots using fixed test seeds exposed only by a test harness, never by production API input.

### 8.6 Docker, custom pack, and standalone tests

- CI builds the complete runtime image from lockfiles and runs health plus corrected smoke checks in an isolated container/volume.
- Restart the container with the same temporary volume and verify the exact session/turn returns.
- Start with a schema-1 custom pack, a schema-2 custom visual pack, and invalid packs; verify fallback or precise startup failure as appropriate.
- Validate that runtime runs as the nonroot user and can read pack assets while only writing the save directory.
- Regenerate `PLAY_WITHOUT_DOCKER.html` and fail CI on drift. If retained, run its full flow from a local file context with browser storage and no network.

### 8.7 Expo/mobile tests

- Run strict typecheck and API fixture tests on every contract change.
- Add React Native Testing Library coverage for loading/error, selection, action, end state, and optional-field tolerance.
- Test physical Android/iOS connectivity, safe areas, orientation policy, large text, and low-memory behavior.
- If a Phaser WebView is approved later, separately test bridge message validation, navigation allowlists, offline behavior, back handling, lifecycle resume, and parity with browser state.

### 8.8 Release gates

At minimum, require formatting/lint, Rust unit/integration tests, schema validation, frontend unit tests, browser smoke/accessibility checks, Expo typecheck, Docker build/smoke/restart, and legacy/custom-pack compatibility. Keep the existing manual stochastic-path checklist, augmented with visual, input, offline, and device checks; automated tests should use fixed fixtures/seeds so failures are reproducible.
