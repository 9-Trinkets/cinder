# Cinder

Cinder is a **terminal-based narrative game engine** built on top of
[Neuron](https://github.com/9-Trinkets/neuron), a Synapse-first workflow runtime.
The engine is separate from authored content — each game is a *content pack*
loaded at startup via `--content <name>`.

## Games

| Content Pack | Description |
|---|---|
| `ella` | A young woman's last night at home before leaving for college — reflecting on the life she's about to leave behind |
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
Cargo.toml            Workspace root (members: cinder-core, cinder-tui)
cinder-core/          Engine library (runtime, content loading, event system)
cinder-tui/           Terminal frontend (ratatui, effects, input)
  src/main.rs         CLI entry point + content pack loading
  src/cli.rs          Terminal loop, turn submission, NPC tick scheduling
config/workflows/     Synapse workflow TOML files
content/<pack>/       Authored content packs (characters, rooms, beats, menus, locale data)
docs/                 Concept and design docs
```

### Runtime setup

1. `cinder-tui/src/main.rs` parses CLI flags (`--trace-events`, `--content`) and loads the content pack.
2. `CinderRuntime::new` builds the engine from the content pack.
3. `cinder-tui/src/cli.rs` starts the terminal loop, submits player turns, and runs periodic NPC ticks.

### Player turn flow

1. UI submits text command to `CinderRuntime::run_turn`.
2. `cinder-core/src/engine/turn_runner.rs` runs the workflow `config/workflows/cinder_turn.toml`.
3. Roles process command in sequence:
   - `command_parser` parses raw input to `PlayerCommand`
   - `state_reader` reads minimal world snapshot
   - `turn_merge` aggregates command + world signals
   - `turn_planner` emits planned `WorldEvent` values
   - optional `menu_intent_clarifier`, `dialogue_grounder`, `actor_dialogue` for conversation path
   - `turn_reducer` applies events
   - `turn_narrator` joins reducer lines into `TurnOutcome`
4. UI appends output text to transcript and updates modal/menu state.

## Neuron

[Neuron](https://github.com/9-Trinkets/neuron) is Cinder's underlying workflow
runtime. It orchestrates every turn through a directed graph of *roles* — each
role is either an LLM prompt, a symbolic planner (decision table), or a Rust
handler. Cinder authors the game logic as a graph of these roles, and Neuron
routes inputs through the graph, collects outputs, and handles retries,
scheduling, and event traces.

- **Repository**: [github.com/9-Trinkets/neuron](https://github.com/9-Trinkets/neuron)
- **Cinder**: [github.com/9-Trinkets/cinder](https://github.com/9-Trinkets/cinder)

## Architecture

- `cinder-core/src/engine/state.rs` owns mutable world state (rooms, time, stats, memory, objectives, menus).
- `cinder-core/src/engine/events.rs` defines the world event vocabulary used by both player and NPC flows.
- `content/*/*.json` provides authored content per game pack.
- `hooks.json` provides rule-driven side effects (stat changes, affordance visibility, guidance).
