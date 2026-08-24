use crate::engine::dialogue::types::{HostilityCandidate, HostilityPlanRequest};

/// The hostility planner is a mechanical decision role, so its prompt is
/// structural English with a JSON world snapshot rather than locale-templated
/// narration prose.
pub(crate) fn build_hostility_plan_prompt(request: &HostilityPlanRequest) -> String {
    let candidates = request
        .candidates
        .iter()
        .map(|candidate| {
            let HostilityCandidate {
                actor_id,
                actor_name,
                room_id,
                hp,
                strength,
                minutes_since_last_strike,
                attack_interval_minutes,
            } = candidate;
            format!(
                "- id: {actor_id}\n  name: {actor_name}\n  room: {room_id}\n  hp: {hp}\n  strength: {strength}\n  minutes_since_last_strike: {minutes_since_last_strike}\n  attack_interval_minutes: {attack_interval_minutes}"
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "You are the combat director for a text adventure. On each background tick you decide which hostile characters strike the player.\n\nWorld snapshot:\n- player_room: {}\n- player_hp: {}\n\nHostile actors ready to strike (cooldown already elapsed):\n{}\n\nRules:\n- You may only return ids from the list above.\n- An actor strikes at most once per tick.\n- Returning no strikes is a valid choice.\n\nRespond with ONLY a JSON object of the form {{\"strikes\": [\"<actor_id>\", ...]}}. Use [] to hold all strikes.\n",
        request.player_room_id, request.player_hp, candidates
    )
}

pub(crate) fn hostility_planner_system_prompt(request: &HostilityPlanRequest) -> &str {
    &request.system_prompt
}
