# Phase 1 — Public Web Architecture

## Goal

Deploy Cinder as a public web application where any player can sign up, start a game session, and play — with Postgres persistence, stateless request execution, and the ability to scale horizontally.

## Principles

1. **Stateless turns** — every HTTP request is self-contained; no in-memory session map
2. **Postgres as single source of truth** — all state, transcript, and player history lives in the DB
3. **Implicit autosave** — every turn persists world state; manual save/load becomes a checkpoint bookmark
4. **Request-driven by default** — NPC ticks run synchronously after each player turn; no background threads per session

## Architecture Stack

```
Client (React SPA)  ──HTTPS──>  Axum (cinder-srv)  ──sqlx──>  Postgres
                                        │
                                        └── cargo──>  cinder-core (CinderRuntime)
                                                         │
                                                         └── Neuron (LLM workflow)
```

## Database Schema

### `players`
| Column | Type | Notes |
|---|---|---|
| `id` | `UUID` | PK |
| `username` | `VARCHAR(255)` | UNIQUE, NOT NULL |
| `password_hash` | `VARCHAR(255)` | NOT NULL |
| `created_at` | `TIMESTAMPTZ` | DEFAULT `NOW()` |

### `game_sessions`
| Column | Type | Notes |
|---|---|---|
| `id` | `UUID` | PK |
| `player_id` | `UUID` | FK → players(id) |
| `pack_id` | `VARCHAR(255)` | NOT NULL |
| `state_json` | `JSONB` | Serialized `WorldState` |
| `created_at` | `TIMESTAMPTZ` | DEFAULT `NOW()` |
| `updated_at` | `TIMESTAMPTZ` | DEFAULT `NOW()` |

### `transcript_entries` (new)
| Column | Type | Notes |
|---|---|---|
| `id` | `UUID` | PK |
| `session_id` | `UUID` | FK → game_sessions(id) ON DELETE CASCADE |
| `turn_number` | `INT` | NOT NULL |
| `role` | `VARCHAR(10)` | `'player'` or `'npc'` |
| `text` | `TEXT` | NOT NULL |
| `inserted_at` | `TIMESTAMPTZ` | DEFAULT `NOW()` |

Index: `(session_id, turn_number)` for ordered retrieval.

### `checkpoints` (future, replaces manual save)
| Column | Type | Notes |
|---|---|---|
| `id` | `UUID` | PK |
| `session_id` | `UUID` | FK → game_sessions(id) ON DELETE CASCADE |
| `label` | `VARCHAR(255)` | Player-provided name |
| `state_json` | `JSONB` | Snapshot of `WorldState` at checkpoint time |
| `created_at` | `TIMESTAMPTZ` | DEFAULT `NOW()` |

## Execution Flow (Stateless Turn)

```
1. POST /api/games/{id}/command  { input: "look around" }
2. Server loads game_sessions.state_json from Postgres
3. Reconstruct CinderRuntime from state
4. Run turn → NPC tick → projector sequence
5. Write updated state_json back to game_sessions
6. Append to transcript_entries
7. Return CommandResponse
8. Drop CinderRuntime (no in-memory cache)
```

This means `CinderRuntime::new()` / `from_state()` is called on every command. The overhead of deserializing `WorldState` (~tens of KB) is negligible compared to the LLM call.

## API Changes

| Endpoint | Change |
|---|---|
| `POST /api/auth/signup` | Unchanged |
| `POST /api/auth/login` | Unchanged |
| `GET /api/games` | Fetch from DB (already does) |
| `POST /api/games` | Create in DB (already does) |
| `POST /api/games/{id}/command` | **Stateless** — load/reconstruct/persist/drop |
| `GET /api/games/{id}/transcript` | **New** — load from `transcript_entries` table |
| `GET /api/games/{id}/ui` | **Stateless** — reconstruct runtime, build snapshot, drop |
| `DELETE /api/games/{id}` | Unchanged |
| `POST /api/games/{id}/save` | **Demoted** — creates a named checkpoint row |
| `POST /api/games/{id}/load` | **Demoted** — restores from a checkpoint |
| `GET /api/games/{id}/stream` | **Removed** — no WS streaming |

## Realtime Strategy (Phase 1)

No background NPC ticks. After each player turn, `run_tick()` is called synchronously and the tick text is merged into the same `CommandResponse.text`. This eliminates:

- Per-session background threads
- WebSocket state management
- Tick pause/resume complexity
- Connection affinity for horizontal scaling

If NPC ticks are needed independently of player actions in the future, a lightweight SSE or polling endpoint can be added.

## Auth & Security

- JWT remains (7-day expiration, configurable)
- All queries filter by `player_id` for session isolation
- Rate limiting via `tower` middleware (e.g., `tower-governor`)
- CORS locked to deployed frontend origin
- Input validation on all request bodies
- Postgres connection encrypted (TLS) in production

## Horizontal Scaling

- Stateless turns: any instance handles any request
- Postgres is the single bottleneck — connection pooling via `PgPool` with `max_connections` tuned
- For reads: replicas can serve `GET /api/games/{id}/ui` and transcript
- Session affinity not required

## Migration Path

1. **Swap SQLite → Postgres** (this task)
2. **Refactor session execution** to stateless DB-backed turns
3. **Implicit autosave** — persist state on every command
4. **Persist transcript** to `transcript_entries` table
5. **Simplify realtime** — remove WS, inline ticks in command response
6. **Harden auth** — rate limiting, CORS, validation

Steps 2-5 can be done incrementally; the in-memory session map can be replaced one endpoint at a time.
