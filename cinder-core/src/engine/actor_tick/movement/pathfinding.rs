use crate::content::types::ContentPack;
use crate::engine::state::WorldState;
use crate::engine::turn_policies::story_var_is_truthy;
use std::collections::{BTreeSet, VecDeque};

/// BFS through reachable exits returning the first room on the shortest path
/// from `current_room_id` to `target_room_id`.
pub(crate) fn next_room_toward(
    content: &ContentPack,
    state: &WorldState,
    current_room_id: &str,
    target_room_id: &str,
) -> Option<String> {
    if current_room_id.is_empty() || target_room_id.is_empty() || current_room_id == target_room_id
    {
        return None;
    }
    let mut queue = VecDeque::from([(current_room_id.to_string(), None::<String>)]);
    let mut visited = BTreeSet::from([current_room_id.to_string()]);

    while let Some((room_id, first_step)) = queue.pop_front() {
        let room = content.room(&room_id)?;
        for exit in &room.exits {
            if !exit.requires_story_var.is_empty()
                && !story_var_is_truthy(state, &exit.requires_story_var)
            {
                continue;
            }
            let is_target = exit.room_id == target_room_id;
            if (!is_target && !content.room_is_reachable(&exit.room_id))
                || !visited.insert(exit.room_id.clone())
            {
                continue;
            }
            let candidate_first_step = first_step.clone().unwrap_or_else(|| exit.room_id.clone());
            if is_target {
                return Some(candidate_first_step);
            }
            queue.push_back((exit.room_id.clone(), Some(candidate_first_step)));
        }
    }
    None
}
