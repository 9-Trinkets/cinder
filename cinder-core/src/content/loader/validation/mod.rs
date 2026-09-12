mod definitions;
mod party;
mod references;

pub(crate) use definitions::{
    validate_actions, validate_combat_settings, validate_items, validate_maps,
    validate_periodic_actor_effects,
};
pub(crate) use party::validate_party_policy;
pub(crate) use references::{
    IdIndex, IdKind, PackContext, validate_contents, validate_feedback_channel,
    validate_scripted_sequences,
};

use std::error::Error;

pub(super) fn require_known_id(
    id: &str,
    known: &[&str],
    subject: &str,
    collection: &str,
) -> Result<(), Box<dyn Error>> {
    if known.contains(&id) {
        Ok(())
    } else {
        Err(format!("{subject} not found in {collection}").into())
    }
}
