pub mod actor_tick;
pub mod actor_turn;
pub mod behavior;
pub mod commands;
pub mod conversation_memory;
pub mod dialogue;
pub mod dialogue_grounding;
pub mod events;
pub mod hook_ids;
pub mod hooks;
pub mod hostile_actions;
pub mod menus;
pub mod narrative;
pub mod neuron;
pub mod reducer;
pub mod roles;
pub mod runtime;
pub mod state;
pub mod turn_policies;
pub mod turn_runner;
pub mod workflows;

pub use hostile_actions as hostility;

/// Shared synthetic content packs for tests and integrations (e.g. the
/// `tests/` suite). Deliberately public so integration tests can build the
/// canonical minimal pack without duplicating fixture code.
pub mod test_fixtures;
