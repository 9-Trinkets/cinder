use crate::content::types::ContentPack;
use crate::engine::state::WorldState;

/// Canonical, content-agnostic world mutations shared across content surfaces
/// (scripted world-hooks, beat-advance `AdvanceEffect`s). Each surface keeps
/// its own serde authoring shape for pack compatibility and converts into this
/// enum before touching state, so the mutation itself is interpreted exactly
/// once instead of being re-implemented inside every interpreter.
#[derive(Debug, Clone)]
pub(crate) enum WorldMutation {
    AdjustActorStat {
        actor_id: String,
        stat: String,
        delta: i32,
    },
    AdjustPairStat {
        participant_a_id: String,
        participant_b_id: String,
        stat: String,
        delta: i32,
    },
    SetStoryVar {
        key: String,
        value: String,
    },
}

/// Applies [`WorldMutation`]s against live engine state. Constructed per batch
/// from the `&mut WorldState` and `&ContentPack` the surfaces already hold; the
/// returned `Result` lets each surface keep its own error policy (hooks
/// propagate, beat-advance logs).
pub(crate) struct WorldMutationApplier<'a> {
    state: &'a mut WorldState,
    content: &'a ContentPack,
}

impl<'a> WorldMutationApplier<'a> {
    pub(crate) fn new(state: &'a mut WorldState, content: &'a ContentPack) -> Self {
        Self { state, content }
    }

    pub(crate) fn apply(&mut self, mutation: &WorldMutation) -> Result<(), String> {
        match mutation {
            WorldMutation::AdjustActorStat {
                actor_id,
                stat,
                delta,
            } => self
                .state
                .adjust_actor_stat(self.content, actor_id, stat, *delta),
            WorldMutation::AdjustPairStat {
                participant_a_id,
                participant_b_id,
                stat,
                delta,
            } => self
                .state
                .adjust_pair_stat(participant_a_id, participant_b_id, stat, *delta),
            WorldMutation::SetStoryVar { key, value } => {
                self.state.story_vars.set_unchecked(key, value);
                Ok(())
            }
        }
    }
}
