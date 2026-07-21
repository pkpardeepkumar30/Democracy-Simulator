# Architecture

## Runtime

The production Docker image contains one compiled Rust binary and the default game pack. The binary embeds the browser HTML, CSS, JavaScript, web manifest and service worker at compile time.

```text
Browser/PWA or Expo client
          |
       HTTP JSON
          |
Rust Axum API
          |
Pure transition engine
          |
Atomic JSON session store on /data
```

## Latency-sensitive path

One player action uses one API request and one locked state transition:

1. Validate session and action requirements.
2. Deduct the declared action cost.
3. Calculate weighted stochastic outcomes using the session seed and turn.
4. Apply outcome effects.
5. Resolve win/loss conditions.
6. Append the event.
7. Persist an atomic snapshot.
8. Return the complete public state.

There are no sequential client calls for cost deduction, probability selection or state refresh.

## Determinism and uncertainty

A session receives a random 64-bit seed and hidden administrative context. Each turn derives its pseudo-random stream from the session seed and turn number. This permits deterministic debugging while keeping future outcomes hidden from clients.

The server stores each result by `client_action_id`. Repeating the same identifier returns the original response without charging the player twice.

## Storage

The MVP writes all sessions to `/data/sessions.json` through an atomic temporary-file rename. Docker mounts `/data` as the named volume `civic_game_data`.

For a public service, replace `SessionStore` with a PostgreSQL implementation and keep the engine unchanged.

## Clients

- `web/`: dependency-free responsive PWA, embedded into the Rust executable.
- `apps/mobile/`: Expo/React Native client consuming the same API.

The Rust server remains authoritative for competitive or multiplayer use. A later offline version can expose the pure Rust engine through native bindings and store local games in SQLite.
