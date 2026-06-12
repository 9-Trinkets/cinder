# Cinder

Cinder is a **terminal-based narrative game engine** built on top of
[Neuron](https://github.com/9-Trinkets/neuron), a Synapse-first workflow runtime.
The engine is separate from authored content — each game is a *content pack*
loaded at startup via `--content <name>`.

## Games

| Content Pack | Description |
|---|---|
| `ella` | First-person bedroom escape — a sandbox for exploring the engine |
| `isla` | Bibliotherapy simulation with structured session progression, relationship stats, and LLM-generated book recommendations |
| `aera` | Documentary house experiment — four strangers, seven days, and a camera that decides where to look |

## Run

Dialogue roles are executed through Neuron using `neuron.toml` for LLM provider/backend
configuration and `config/workflows/<game>.toml` for per-role model, profile, and
prompt overrides.

```bash
# Start an LLM backend e.g. llama.cpp
llama-server -m ~/models/qwen2.5-7b-instruct.gguf --host 127.0.0.1 --port 8080

# Run the Isla content pack
OPENAI_API_KEY=local cargo run -- --content isla
```

Enable Synapse event traces:

```bash
cargo run -- --trace-events
```

Trace files are written under `.cinder-state/runs/`.

Display language can be switched in-game from `? Menu` → `Language`.

## Layout

```
src/app/          Terminal app shell (TUI, input, effects, transcript)
src/engine/       Generic runtime, events, reducer, workflow loading
src/content/      Content loading and type definitions
config/workflows/ Synapse workflow TOML files
content/<pack>/   Authored content packs (characters, rooms, beats, menus, locale data)
docs/             Concept and design docs
```

### Runtime setup

1. `src/main.rs` parses CLI flags (`--trace-events`, `--content`).
2. `src/lib.rs::run_cli` loads a content pack (`src/content/loader.rs`) and builds
   `CinderRuntime` (`src/engine/runtime.rs`).
3. `src/app/cli.rs` starts the terminal loop, submits player turns, and runs periodic NPC ticks.

### Player turn flow

1. UI submits text command to `CinderRuntime::run_turn`.
2. `src/engine/turn_runner.rs` runs the workflow `config/workflows/cinder_turn.toml`.
3. Roles process command in sequence:
   - `command_parser` parses raw input to `PlayerCommand`
   - `state_reader` reads minimal world snapshot
   - `turn_merge` aggregates command + world signals
   - `turn_planner` emits planned `WorldEvent` values
   - optional `menu_intent_clarifier`, `dialogue_grounder`, `actor_dialogue` for conversation path
   - `turn_reducer` applies events
   - `turn_narrator` joins reducer lines into `TurnOutcome`
4. UI appends output text to transcript and updates modal/menu state.

## Architecture

- `src/engine/state.rs` owns mutable world state (rooms, time, stats, memory, objectives, menus).
- `src/engine/events.rs` defines the world event vocabulary used by both player and NPC flows.
- `content/*/*.json` provides authored content per game pack.
- `hooks.json` provides rule-driven side effects (stat changes, affordance visibility, guidance).
