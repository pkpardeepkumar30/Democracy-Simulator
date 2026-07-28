# Democracy Simulator

A reusable, data-driven civic-simulation platform. The initial library contains three stochastic environments: repair a flooded ward, organize accountability after a national examination leak, and lawfully acquire land and approvals for a food-processing factory.

The package contains:

- A low-latency Rust game server using Axum and Tokio.
- A responsive browser game served by the Rust binary.
- A locally bundled Phaser 3 civic-city renderer with compact OpenStreetMap street geometry for New Delhi, Beijing and Rio de Janeiro, server-driven animations, reduced-motion handling and an SVG/DOM fallback.
- Persistent saved sessions stored in a Docker volume.
- A validated schema-v2 JSON/YAML game-pack library with three complete environments, nine player profiles, 31 state-gated actions, declarative institutions, stakeholders, hidden variables, random events, endings and visual themes.
- A multi-scenario registry and catalog API; adding a validated `game.json` directory does not require engine changes.
- A typed world-abstraction catalog and deterministic scenario composer with selectable environment dimensions, roles, objectives, modifiers and difficulty.
- Optional cross-mission campaigns that preserve complete histories and carry pack-declared reputation, knowledge, relationships and consequences into later missions.
- Seeded stochastic decisions and idempotent action requests.
- An installable progressive web app suitable for phone testing.
- An Expo/React Native client for later Android and iOS packaging.
- Docker and one-click Windows scripts.

## Immediate test with no installation

Double-click `PLAY_WITHOUT_DOCKER.html`. It runs the full drainage scenario locally in a modern browser and saves its state in browser storage.

This standalone mode is intended for immediate gameplay and interface testing. Use the Docker test below to verify the authoritative Rust server, server-side persistence and duplicate-action protection.

## Authoritative Rust/Docker test on Windows

### Prerequisite

Install and start **Docker Desktop**. No Rust, Node.js, PostgreSQL or other project dependency is required.

### Start

1. Extract the ZIP.
2. Open the extracted folder.
3. Double-click `START_GAME.bat`.
4. The first build downloads the Rust builder image and compiles the server.
5. Your browser opens at `http://localhost:8080`.

The first Docker build may take longer because it downloads and compiles dependencies. Later starts reuse Docker’s build cache.

### Stop

Double-click `STOP_GAME.bat`.

Saved games remain available because they are stored in the Docker volume named `civic_game_data`.

### Delete all saves

Double-click `RESET_GAME_DATA.bat` and confirm the warning.

## Command-line alternative

From the extracted folder:

```powershell
docker compose up --build -d
```

Open:

```text
http://localhost:8080
```

View logs:

```powershell
docker compose logs -f game
```

Stop without deleting saves:

```powershell
docker compose down
```

Delete containers and saved sessions:

```powershell
docker compose down -v
```

## What to test manually

Follow `MANUAL_TEST_CHECKLIST.md`. The minimum useful test is:

1. Select an environment, then select a player profile such as **Ramesh Kumar**.
2. Confirm that only actions valid in the initial state are shown; locked actions are absent rather than disabled.
3. File a formal complaint.
4. Confirm that money, energy, days and several civic factors change; there is no generic progress-only victory bar.
5. Refresh the browser; the same session should return.
6. Try building evidence strength, public support, media pressure, institutional pressure and movement unity while reducing government resistance.
7. Observe how valid actions appear as their prerequisites are met and disappear after their one meaningful use.
8. In the examination scenario, advance evidence from unverified through independent verification, corroboration, confirmed chain of custody and legal admissibility.
9. If a whistleblower packet arrives, confirm that the new **Verify whistleblower evidence** action appears.
10. Start a new game with another citizen and compare the constraints.

Outcomes are stochastic. Restarting a session creates a new hidden administrative context and random seed.

The real street geometry is contextual only: every civic scenario, character and institution location is fictional. Map data is © OpenStreetMap contributors and licensed under ODbL; import and attribution details are in `docs/MAP_DATA.md`.

## Test on an Android phone or iPhone now

The web client is responsive and installable as a PWA.

1. Run the Docker game on your computer.
2. Find the computer’s local network address. On Windows, run `ipconfig` and use the IPv4 address, for example `192.168.1.50`.
3. Ensure the phone and computer use the same Wi-Fi network.
4. On the phone, open `http://192.168.1.50:8080`.
5. If Windows Firewall asks, allow access on private networks.
6. Android Chrome: use **Add to Home screen** or **Install app**.
7. iPhone Safari: use **Share → Add to Home Screen**.

This tests the phone layout and PWA packaging. The included Expo client is the path for later App Store and Play Store binaries.

## Automated smoke test

With the game running, use Git Bash, WSL, macOS or Linux:

```bash
./scripts/smoke-test.sh
```

The script checks health, scenario loading, session creation and the first stochastic action.

The visual bundle has its own reproducible checks:

```powershell
cd web
npm ci
npm run typecheck
npm test
npm run build
```

With a server running, `npm run e2e` uses a locally installed Chromium/Edge executable supplied through `BROWSER_PATH`. It verifies desktop/mobile canvas layout and submits a real server action by clicking a Phaser location.

You can also inspect the health endpoint directly:

```text
http://localhost:8080/api/v1/health
```

## Native Android/iOS development

The native client is in `apps/mobile`. It is not needed for the Docker browser test.

For mobile development, install Node.js and Expo tooling, then configure the server address:

```powershell
cd apps/mobile
$env:EXPO_PUBLIC_API_URL="http://192.168.1.50:8080"
npm install
npm start
```

Use the LAN address of the computer running the Rust server. `localhost` on a physical phone refers to the phone, not the computer.

Before store publication, replace the example identifiers in `apps/mobile/app.json`:

- `ios.bundleIdentifier`
- `android.package`

## Repository structure

```text
civic-sim-mvp/
├── src/                         Rust server and game engine
├── web/                         Responsive PWA served by Rust
├── game-packs/drainage/         Data-driven first game
├── apps/mobile/                 Expo Android/iOS/web client
├── scripts/                     Smoke test
├── Dockerfile                   Multi-stage production image
├── compose.yaml                 Runtime and persistent volume
├── START_GAME.bat               Windows launcher
├── STOP_GAME.bat                Windows stop script
├── RESET_GAME_DATA.bat          Delete saved sessions
└── MANUAL_TEST_CHECKLIST.md
```

## Add another environment

The server discovers every `game.json`, `game.yaml` or `game.yml` below `GAME_PACKS_PATH` (default `game-packs`). `GAME_PACK_PATH` remains available for a single legacy/custom pack. To create another environment:

1. Copy `game-packs/drainage/game.json` into a new folder.
2. Change the environment profile, mission, value definitions, player profiles, institutions, stakeholders, barriers, random events, endings, visual theme and actions.
3. Validate the pack using the server tests/validator.
4. Place it below `game-packs/` or mount its directory into the existing image.

An example override is provided in `compose.custom-pack.example.yaml`:

```bash
docker compose -f compose.yaml -f compose.custom-pack.example.yaml up --build
```

See `GAME_PACK_GUIDE.md` for the format.

## Balance and simulation tooling

Run deterministic, goal-directed Monte Carlo simulations across every role and discovered scenario inside the pinned builder:

```bash
cargo run --bin simulate -- 250
cargo run --bin simulate -- trace factory-ground-v1 local_entrepreneur 17
cargo run --bin simulate -- generated business_land hard 250
```

The first command reports status/ending distributions. The trace command explains one action path turn by turn. The generated form composes a deterministic objective/difficulty environment and checks its reachability with the same policy.

## Current platform boundaries

This release still uses an atomic JSON session store rather than PostgreSQL. The engine, pack registry, API and storage module are separated so storage can later be replaced without changing scenario rules or clients.

Schema-v2 packs, multi-environment selection, generic variables, random events, declarative endings, JSON/YAML loading, persistent generated worlds, coherence and difficulty overlays, cross-mission campaigns, the shared Phaser/DOM visual system and Expo random-world flow are implemented. Authentication, multiplayer roles, external pack-art pipelines and an admin scenario editor remain future work tracked in `docs/ENVIRONMENT_LIBRARY_PROGRESS.md`.
