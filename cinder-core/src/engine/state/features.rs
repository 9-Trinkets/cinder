//! The state of room features that can be operated: a lamp that is off, dim or
//! bright, a valve that is closed or open.

use super::WorldState;
use crate::content::types::ContentPack;

/// Key a feature's state is stored under.
pub(super) fn feature_state_key(room_id: &str, feature_id: &str) -> String {
    format!("{room_id}::{feature_id}")
}

impl WorldState {
    /// The state a feature is in now: what has been set, or the state it starts
    /// in. `None` for a feature that does not exist or has no states.
    pub fn feature_state(
        &self,
        content: &ContentPack,
        room_id: &str,
        feature_id: &str,
    ) -> Option<String> {
        let feature = content.feature(room_id, feature_id)?;
        if feature.states.is_empty() {
            return None;
        }
        Some(
            self.feature_states
                .get(&feature_state_key(room_id, feature_id))
                .cloned()
                .unwrap_or_else(|| feature.initial_state().to_string()),
        )
    }

    /// Puts a feature in a state. Does nothing for a feature that is unknown, has
    /// no states, or does not declare this one.
    pub fn set_feature_state(
        &mut self,
        content: &ContentPack,
        room_id: &str,
        feature_id: &str,
        state: &str,
    ) -> bool {
        let Some(feature) = content.feature(room_id, feature_id) else {
            return false;
        };
        if !feature.states.iter().any(|declared| declared == state) {
            return false;
        }
        self.feature_states
            .insert(feature_state_key(room_id, feature_id), state.to_string());
        true
    }
}
