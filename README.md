# Cinder

Cinder is a **narrative game engine** built on top of
[Neuron](https://github.com/9-Trinkets/neuron), a Synapse-first workflow runtime.
The engine is separate from authored content — each game is a *content pack*
(a `content/<pack>/` directory of JSON authored data) selected per game session.

Cinder runs as a backend service (`cinder-srv`) with a React frontend (`cinder-web-ui`)
and a core engine library (`cinder-core`). Everything is authored in JSON, executed
by the Rust engine, and rendered through a responsive, themeable web UI.

## Games

| Content Pack | Title | Description |
|---|---|---|
| `aera` | Aera | You're the director behind a reality show house, watching four strangers navigate attraction and trust over a few days. Mostly you stay silent and let it play out — a quiet instruction or well-timed twist pushes the story where you want it. |
| `ella` | Ella | The last night home before everything changes. In a small apartment with her Taiwanese immigrant parents, love gets said through food and small gestures more than words — see what finally gets spoken before she walks out the door. |
| `isla` | Isla | One book, one evening, one thing they can't quite say. A quiet, rainy evening and Isla meeting a new patient — someone composed, guarded, and carrying a hurt they won't name outright. Find the story that can reach what conversation alone cannot. |
| `layla` | Layla | Wake lost. Learn the rules. Choose who you become. Layla wakes with no memory in a goblin cave, beside a golem that does not move. To win her way out she must capture the cave from the shaman and its chalk-marked servants — claiming territory one room at a time. Below it waits an elf army that refuses to be bound, and beyond them, old fire held prisoner until it is released. Every rule she learns was built on purpose, for a purpose she cannot remember — and each one is training her to become something she will have to decide for herself. |

Each pack carries its own locales, stats, movement rules, beat objectives, menus,
items/levels (where combat applies), and presentation/theme settings.

## Architecture

```
Cargo.toml            Workspace root (members: cinder-core, cinder-srv)
cinder-core/          Engine library — runtime, state, turn & NPC flows, dialogue, content loading
cinder-srv/           Backend HTTP server: auth, DB, session management, API + WebSocket
cinder-web-ui/        React + Vite frontend
config/workflows/     Synapse workflow TOML files (turn, NPC tick, per-game overrides)
content/<pack>/       Authored content packs (JSON)
scripts/              Dev guardrails and utilities
docs/                 Concept and design docs
```

The engine is organized by responsibility:

- `cinder-core/src/engine/state/` — mutable world state (clock, rooms, stats,
  inventory, relationships, conversation memory, story vars, act cast, seeding).
- `cinder-core/src/engine/runtime/` — session orchestration, actor ticks, queries,
  menus (driven by a generic data-driven panel system), act closure, perspective
  review, stage assignment.
- `cinder-core/src/engine/turn_runner/` — the player-turn pipeline, including the
  `planning/` domain modules and the shared `plan_dialogue_command` dispatcher.
- `cinder-core/src/engine/actor_turn/` — autonomous NPC turns: builder, movement,
  affordances, targeting, symbolic planner, realization, and policy filters.
- `cinder-core/src/engine/dialogue/` — the `DialogueGenerator` trait and prompt
  builders; the Synapse/Neuron-backed generators live in `synapse.rs`.
- `cinder-core/src/engine/events.rs` — the world-event vocabulary shared by both
  player and NPC flows.
- `cinder-core/src/content/` — content loading (`loader/` with validation) and the
  typed content definitions (`types/`, `text_defs/`).

### Web UI

The frontend is a React + Vite app (`cinder-web-ui/`). Routes:

- `/login` — sign in (JWT auth, backed by `bcrypt` passwords).
- `/games` — pick a content pack to start or resume a playthrough.
- `/games/pack/:packId` — pack detail.
- `/games/:id` — the live game (transcript, status panels, menus, quick actions,
  act/movie modals, NPC ticks, relationship chart).

The backend exposes a REST API under `/api` plus a WebSocket endpoint
(`/api/games/{id}/ws`) for real-time NPC ticks. Playthroughs persist to Postgres.

## Run

Dialogue and NPC roles are executed through Neuron using `neuron.toml` for LLM
provider/backend configuration and `config/workflows/<game>.toml` for per-role
model, profile, and prompt overrides.

```bash
# 1. Start a Postgres instance (Docker Compose brings up postgres + the whole stack)
docker compose up -d postgres

# 2. Run the backend server
OPENAI_API_KEY=<key> cargo run -p cinder-srv

# 3. Run the frontend dev server (proxies /api to the backend)
cd cinder-web-ui && npm install && npm run dev
```

By default the server listens on `127.0.0.1:3000`. Configuration is read from
environment variables (`CINDER_DATABASE_URL`, `CINDER_JWT_SECRET`,
`CINDER_CORS_ORIGIN`, `CINDER_STRICT_CONFIG`, `CINDER_HOST`, `CINDER_PORT`);
`cinder-srv/src/config.rs` is the single source of truth for those.

To run the whole stack containerized (Postgres + server + web):

```bash
OPENAI_API_KEY=<key> docker compose up --build
```

The web UI is served on `8081` by default (`WEB_PORT`), with nginx proxying `/api`
to the server.

Display language can be switched in-game from the menu.

## API

`CINDER_HOST`/`CINDER_PORT` configure the server. Endpoints:

- `POST /api/auth/signup`, `POST /api/auth/login` — JWT auth.
- `GET  /api/packs` — list available content packs.
- `GET  /api/games`, `POST /api/games` — list or create a playthrough.
- `POST /api/games/{id}/command` — submit a player command.
- `POST /api/games/{id}/tick` — advance an NPC tick.
- `GET  /api/games/{id}/ui`, `GET /api/games/{id}/transcript` — session snapshot.
- `POST /api/games/{id}/room`, `POST /api/games/{id}/follow`,
  `POST /api/games/{id}/locale`, `POST /api/games/{id}/continue` — session actions.
- `DELETE /api/games/{id}` — delete a playthrough.
- `GET  /api/games/{id}/ws` — WebSocket for live NPC ticks.

## Debug utilities

Server-side debug binaries live under `cinder-srv/src/bin/debug/`.

Inspect the latest persisted playthrough, including transcript and `beat_objective:*`
story vars:

```bash
cargo run -p cinder-srv --bin debug_playthrough -- --pack aera --transcript-limit 20
```

Print actor-turn prompt data from a saved playthrough, including system prompt,
request JSON, and rendered prompt text:

```bash
cargo run -p cinder-srv --bin debug_prompts -- --pack aera --actor ren
```

When Postgres is only reachable on the Docker Compose network, run the utilities
inside that network:

```bash
docker run --rm --network cinder_default \
  -v "$PWD":/work -w /work \
  -e CINDER_DATABASE_URL='postgres://cinder:cinder@postgres:5432/cinder' \
  rust:1-bookworm \
  cargo run -p cinder-srv --bin debug_playthrough -- --pack aera --transcript-limit 20

docker run --rm --network cinder_default \
  -v "$PWD":/work -w /work \
  -e CINDER_DATABASE_URL='postgres://cinder:cinder@postgres:5432/cinder' \
  rust:1-bookworm \
  cargo run -p cinder-srv --bin debug_prompts -- --pack aera --actor ren
```

## Workflow

`config/workflows/cinder_turn.toml` describes the player-turn graph (read by Neuron):

```
turn_dispatch -> command_parser
command_parser -> turn_merge
state_reader -> turn_merge
turn_merge -> turn_planner
turn_planner -> menu_intent_clarifier
menu_intent_clarifier -> dialogue_grounder
menu_intent_clarifier -> turn_reducer
turn_planner -> turn_reducer
dialogue_grounder -> actor_dialogue
actor_dialogue -> turn_reducer
turn_reducer -> turn_narrator
turn_narrator -> complete
```

Roles process the command in sequence: the parser normalizes raw input into a
`PlayerCommand`, the planner emits planned `WorldEvent` values (with optional
LLM-driven paths for menu intent, dialogue grounding, actor dialogue, hostility
planning, and stage assignment), the reducer applies events to world state, and
the narrator joins the result into a `TurnOutcome`.

## Development

Guide rails are enforced as a warning-only git pre-commit hook (see
`scripts/pre-commit` and `scripts/install_hooks.sh`):

- `scripts/check_file_lengths.py` — flags source files over 500 lines.
- `scripts/check_test_placement.py` — flags integration suites living under `src/`
  instead of `tests/`.

Before committing:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
python3 scripts/check_file_lengths.py
python3 scripts/check_test_placement.py
# web UI
cd cinder-web-ui && npx tsc -b && npm run build
```

## Neuron

[Neuron](https://github.com/9-Trinkets/neuron) is Cinder's underlying workflow
runtime. It orchestrates every turn through a directed graph of *roles* — each
role is either an LLM prompt, a symbolic planner (decision table), or a Rust
handler. Cinder authors the game logic as a graph of these roles, and Neuron
routes inputs through the graph, collects outputs, and handles retries,
scheduling, and event traces.

- **Repository**: [github.com/9-Trinkets/neuron](https://github.com/9-Trinkets/neuron)
- **Cinder**: [github.com/9-Trinkets/cinder](https://github.com/9-Trinkets/cinder)
