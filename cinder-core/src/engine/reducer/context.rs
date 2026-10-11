//! The shared reducer context threaded through event dispatch.
//!
//! `ReduceCtx` is the single dependency seam for the reducer: it bundles the
//! read-only content pack, the mutable world state, the narrative sink, the
//! per-turn stat suppression metadata, and the spawned-event outbox. Handlers
//! receive exactly this object instead of five positional arguments, which
//! keeps the dispatch layer decoupled from how the reducer is wired.

use crate::content::types::ContentPack;
use crate::engine::events::WorldEvent;
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::WorldState;

pub(crate) struct ReduceCtx<'a> {
    pub state: &'a mut WorldState,
    pub content: &'a ContentPack,
    pub lines: &'a mut NarrativeLines,
    pub suppressed_actor_stats: &'a [(&'a str, &'a str)],
    pub outbox: &'a mut Vec<WorldEvent>,
}
