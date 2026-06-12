# Ella

Ella is a small narrative game example built to showcase a **Synapse-first** game engine architecture.

## First pass

This first pass is intentionally narrow:

- the player starts in her own bedroom
- the content is loaded from authored files
- the engine is separate from the content layer
- each turn runs through an **event-driven Synapse workflow**
- the workflow is authored in `config/workflows/cinder_turn.toml`
- Cinder role handlers are authored in `config/workflows/cinder_turn.toml`
- the playable actions are:
  - `look`
  - `inspect room`
  - `inspect <thing>`
  - `go <room>`
  - `talk to <person> [message]`
  - `help`
  - `quit`

## Run

Dialogue roles are executed through Neuron using the standard Neuron config file
(`neuron.toml` or `neuron.config.toml`) plus the workflow role config in
`config/workflows/cinder_turn.toml`.

- Neuron config owns the runtime LLM provider/backend, base URLs, and retry policy
- workflow role config owns per-role `model`, `agent_profile`, and optional `system_prompt`
- locale-authored `content/ella/locales/*/system.json` remains the fallback system prompt when
  a role does not override it

```bash
MODEL_TAG="qwen2.5:7b"
MODEL_BLOB="$(ollama show "$MODEL_TAG" --modelfile | awk '/^FROM / {print $2; exit}')"
cp "$MODEL_BLOB" "$HOME/models/qwen2.5-7b-instruct.gguf"

llama-server \
  -m "$HOME/models/qwen2.5-7b-instruct.gguf" \
  --alias qwen2.5-7b-instruct \
  --host 127.0.0.1 \
  --port 8080

OPENAI_API_KEY=local cargo run -p cinder
```

If your `FROM` value is a `sha256-*` blob name instead of an absolute path, copy from:

```bash
cp "$HOME/.ollama/models/blobs/$MODEL_BLOB" "$HOME/models/qwen2.5-7b-instruct.gguf"
```

Enable Synapse event traces:

```bash
cargo run -p cinder -- --trace-events
```

Trace files are written under `examples/cinder/.cinder-state/runs/`.

Display language can be switched in-game from `? Menu` → `Language`.

## Layout

- `src/app/` - terminal app shell
- `src/engine/` - generic runtime, events, reducer, and workflow loading
- `src/content/` - authored content loading and content types
- `config/workflows/` - authored Synapse workflow TOML files
- `content/ella/` - the first authored content pack
- `content/ella/assets/` - shared projector art and other pack-owned static assets
- `content/ella/locales/` - per-locale authored overrides for Ella
- `content/ella/locales/*/opening.json` - opening text and prompt context
- `content/ella/locales/*/beats.json` - authored story beats and progression stages
- `content/ella/locales/*/menus.json` - authored menu definitions
- `content/ella/locales/*/movies.json` - authored movie playback content
- `content/ella/locales/*/presentation.json` - runtime error and presentation text
- `content/ella/locales/*/actors.json` - actor definitions, including actor prompt context, `behavior_examples`, and actor-owned `movement_rules`
- `docs/` - concept and setting docs

## Architecture map

### Runtime setup

1. `src/main.rs` parses CLI flags (`--trace-events`, `--content`).
2. `src/lib.rs::run_cli` loads a content pack (`src/content/loader.rs`) and builds `CinderRuntime` (`src/engine/runtime.rs`).
3. `src/app/cli.rs` starts the terminal loop, submits player turns, and runs periodic NPC ticks.

### Player turn flow

1. UI submits text command to `CinderRuntime::run_turn`.
2. `src/engine/turn_runner.rs` runs workflow `config/workflows/cinder_turn.toml`.
3. Roles process command in this sequence:
   - `command_parser` parses raw input to `PlayerCommand` (`src/engine/commands.rs`)
   - `state_reader` reads minimal world snapshot
   - `turn_merge` aggregates command + world signals
   - `turn_planner` emits planned `WorldEvent` values
   - optional `menu_intent_clarifier`, `dialogue_grounder`, `actor_dialogue` for conversation path
   - `turn_reducer` applies events with `apply_events` (`src/engine/reducer.rs`)
   - `turn_narrator` joins reducer lines into `TurnOutcome`
4. UI appends output text to transcript and updates modal/menu state.

### NPC tick flow

1. Background loop in `src/app/cli.rs` calls `CinderRuntime::run_tick`.
2. `src/engine/runtime.rs::run_npc_turns` starts tick trace scope and emits `TurnStarted`.
3. `src/engine/npc_runner.rs::run_npc_tick` runs workflow `config/workflows/cinder_npc_tick.toml`:
   - `npc_tick_orchestrator` selects next actor
   - `npc_actor_turn` delegates behavior selection and movement
4. Movement decisions use `config/workflows/cinder_npc_turn.toml`.
5. Emitted NPC events are reduced through the same reducer (`src/engine/reducer.rs`) and conversation summaries are refreshed.

### Data and behavior layers

- `src/engine/state.rs` owns mutable world state (rooms, time, stats, memory, objectives, menus, follow state).
- `src/engine/events.rs` defines world event vocabulary used by both player and NPC flows.
- `content/*/*.json` provides authored content.
- `hooks.json` provides rule-driven side effects (stat changes, affordance visibility, guidance, and relationship progression).
