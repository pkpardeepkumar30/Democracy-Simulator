# The Republic — Civic Simulation MVP

A playable stochastic civic-strategy game. The first mission asks a citizen to get a neglected neighbourhood drainage system repaired before money, energy and time run out.

The package contains:

- A low-latency Rust game server using Axum and Tokio.
- A responsive browser game served by the Rust binary.
- Persistent saved sessions stored in a Docker volume.
- A data-driven JSON game pack with three citizens, eight actions and multiple weighted outcomes.
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

1. Select **Ramesh Kumar**.
2. Confirm that some actions are initially locked.
3. File a formal complaint.
4. Confirm that money, energy, days and progress change.
5. Refresh the browser; the same session should return.
6. Try building documentation and community support.
7. Observe how actions such as media escalation and court action unlock.
8. Start a new game with another citizen and compare the constraints.

Outcomes are stochastic. Restarting a session creates a new hidden administrative context and random seed.

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

## Reuse the image for another game

The server loads a game pack from `GAME_PACK_PATH`. To create another game:

1. Copy `game-packs/drainage/game.json` into a new folder.
2. Change the mission, citizen profiles, actions, costs, requirements and weighted outcomes.
3. Validate the JSON syntax.
4. Mount the folder into the existing image.

An example override is provided in `compose.custom-pack.example.yaml`:

```bash
docker compose -f compose.yaml -f compose.custom-pack.example.yaml up --build
```

See `GAME_PACK_GUIDE.md` for the format.

## Current MVP boundaries

This release deliberately uses an atomic JSON session store rather than PostgreSQL. For the intended local test and a small number of users, this removes an unnecessary service and gives a single-container package. The engine, API and storage module are separated so the store can later be replaced with PostgreSQL without changing the game rules or clients.

The first release contains one complete mission. Government-job recruitment, multiplayer roles, authentication, cloud synchronization and an admin scenario editor are not implemented yet.
