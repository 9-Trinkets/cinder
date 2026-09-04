mod content;
mod dialogue;
mod dispatch;
mod observe;
mod targeted;
mod targetless;

use crate::engine::state::WorldState;

pub(super) struct PlanningContext<'a> {
    pub(super) raw_input: &'a str,
    pub(super) current_room_id: &'a str,
    pub(super) planner_state: &'a WorldState,
    pub(super) channel_surfing_only: bool,
    pub(super) turn_number: u32,
}

pub(super) use dispatch::plan_authored_command;
