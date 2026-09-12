use crate::content::types::ContentPack;
use crate::engine::narrative::NarrativeLines;
use crate::engine::reducer::beat_advance::advance_objective_for_signal;
use crate::engine::reducer::command_effects::{
    ActorMoveTransitionContext, actor_display_name, apply_actor_move_transition,
};
use crate::engine::reducer::summaries::summarize_actor_names;
use crate::engine::reducer::tick::advance_house_progress_objectives;
use crate::engine::state::WorldState;

use super::feedback::push_message;

pub(crate) fn handle_actor_relocated(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    to_room_id: &str,
    lines: &mut NarrativeLines,
) {
    state.mark_actor_room_visited(actor_id, to_room_id);
    state
        .actor_room_overrides
        .insert(actor_id.to_string(), to_room_id.to_string());
    lines.extend_narration(advance_house_progress_objectives(state, content));
}

pub(crate) fn handle_actor_moved(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    from_room_id: &str,
    to_room_id: &str,
    lines: &mut NarrativeLines,
) {
    apply_actor_move_transition(
        state,
        content,
        ActorMoveTransitionContext {
            actor_id,
            actor_name: None,
            from_room_id,
            to_room_id,
            command_text: None,
        },
        lines,
    );
}

pub(crate) fn handle_player_moved(
    state: &mut WorldState,
    content: &ContentPack,
    to_room_id: &str,
    lines: &mut NarrativeLines,
) {
    let from_room_id = state.current_room_id.clone();
    lines.extend_narration(advance_objective_for_signal(
        state,
        content,
        &format!("room_left:{from_room_id}"),
    ));
    state.current_room_id = to_room_id.to_string();
    state.mark_actor_room_visited(&content.settings.combat.player_actor_id, to_room_id);
    lines.extend_narration(advance_objective_for_signal(
        state,
        content,
        &format!("room_entered:{to_room_id}"),
    ));
    sync_followers_to_room(state, content, to_room_id, lines);
}

/// Moves every living follower into the player's room (used when the player
/// moves or descends, so the party stays together).
pub(crate) fn sync_followers_to_room(
    state: &mut WorldState,
    content: &ContentPack,
    to_room_id: &str,
    lines: &mut NarrativeLines,
) {
    let follower_actor_ids: Vec<String> = state
        .relationships
        .iter()
        .filter(|(_, relationship)| relationship.follows_player)
        .map(|(actor_id, _)| actor_id.clone())
        .collect();
    let mut moved_actor_names = Vec::new();
    for follower_id in follower_actor_ids {
        if follower_id == content.settings.combat.player_actor_id {
            continue;
        }
        // Offstage followers (e.g. a remote handler) are not physically drawn
        // into the player's room by party membership.
        if content.actor_is_offstage(&follower_id) {
            continue;
        }

        if state.actor_stat(&follower_id, &content.settings.combat.health_stat_id) <= 0 {
            continue;
        }
        let default_room_id = content
            .actor(&follower_id)
            .map(|actor| actor.room_id.clone())
            .unwrap_or_default();
        let already_here = state.actor_room_id(&follower_id, &default_room_id) == to_room_id;
        if !already_here {
            state
                .actor_room_overrides
                .insert(follower_id.clone(), to_room_id.to_string());
            moved_actor_names.push(actor_display_name(content, &follower_id));
        }
    }
    let count = moved_actor_names.len();
    let group_key = follower_group_message_key(content, count);
    if let Some(key) = group_key
        && let Some(actors) = summarize_actor_names(&moved_actor_names)
    {
        let count = count.to_string();
        push_message(
            lines,
            content,
            key,
            &[("actors", actors.as_str()), ("count", count.as_str())],
        );
        return;
    }
    for actor in moved_actor_names {
        push_message(
            lines,
            content,
            "follow.actor_follows",
            &[("actor", actor.as_str())],
        );
    }
}

fn follower_group_message_key(content: &ContentPack, count: usize) -> Option<&'static str> {
    if count >= 4 && content.messages.contains_key("follow.large_party_follows") {
        Some("follow.large_party_follows")
    } else if count > 1 && content.messages.contains_key("follow.party_follows") {
        Some("follow.party_follows")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::{PackMessage, PackMessageVoice};
    use crate::engine::narrative::NarrativeLineKind;
    use crate::engine::test_fixtures::{minimal_test_pack, rebuild_test_pack_indexes};

    #[test]
    fn player_movement_records_room_visits() {
        let mut content = minimal_test_pack();
        content.settings.combat.player_actor_id = "player".to_string();
        let mut state = WorldState::new(&content);
        let mut lines = NarrativeLines::default();

        assert!(state.actor_has_visited_room("player", "lounge"));

        handle_player_moved(&mut state, &content, "kitchen", &mut lines);

        assert!(state.actor_has_visited_room("player", "kitchen"));
    }

    #[test]
    fn offstage_follower_is_not_dragged_into_the_player_room() {
        let mut content = minimal_test_pack();
        content.settings.combat.player_actor_id = "player".to_string();
        let blair = content
            .actors
            .iter_mut()
            .find(|actor| actor.id == "blair")
            .unwrap();
        blair.room_id.clear();
        let mut state = WorldState::new(&content);
        let mut relationship = state.relationship("blair");
        relationship.follows_player = true;
        state.set_relationship("blair", relationship);
        let mut lines = NarrativeLines::default();

        handle_player_moved(&mut state, &content, "kitchen", &mut lines);

        assert!(!state.actor_room_overrides.contains_key("blair"));
        assert_eq!(state.actor_room_id("blair", ""), "");
    }

    #[test]
    fn coordinated_party_movement_emits_one_authored_summary() {
        let mut content = minimal_test_pack();
        content.settings.combat.player_actor_id = "player".to_string();
        content.settings.combat.health_stat_id = "stamina".to_string();
        content.messages.insert(
            "follow.party_follows".to_string(),
            PackMessage::Voiced {
                voice: PackMessageVoice::System,
                text: "{actors} move with you.".to_string(),
            },
        );
        content.messages.insert(
            "follow.actor_follows".to_string(),
            PackMessage::Narration("{actor} moves with you.".to_string()),
        );
        let mut state = WorldState::new(&content);
        state.set_follows_player("blair", true);
        state.set_follows_player("casey", true);
        let mut lines = NarrativeLines::default();

        handle_player_moved(&mut state, &content, "kitchen", &mut lines);

        assert_eq!(
            lines
                .0
                .iter()
                .filter(|line| line.text.contains("move with you"))
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>(),
            vec!["Blair and Casey move with you."]
        );
        assert_eq!(
            lines
                .0
                .iter()
                .find(|line| line.text == "Blair and Casey move with you.")
                .map(|line| line.kind),
            Some(NarrativeLineKind::System)
        );
        assert_eq!(state.actor_room_id("blair", "lounge"), "kitchen");
        assert_eq!(state.actor_room_id("casey", "lounge"), "kitchen");
    }

    #[test]
    fn four_or_more_followers_use_the_scalable_party_message() {
        let mut content = minimal_test_pack();
        content.settings.combat.player_actor_id = "player".to_string();
        content.settings.combat.health_stat_id = "stamina".to_string();
        let mut blair_two = content.actor("blair").unwrap().clone();
        blair_two.id = "blair-2".to_string();
        let mut casey_two = content.actor("casey").unwrap().clone();
        casey_two.id = "casey-2".to_string();
        content.actors.extend([blair_two, casey_two]);
        rebuild_test_pack_indexes(&mut content);
        content.messages.insert(
            "follow.party_follows".to_string(),
            PackMessage::Narration("{actors} move with you.".to_string()),
        );
        content.messages.insert(
            "follow.large_party_follows".to_string(),
            PackMessage::Narration("Your party follows close behind.".to_string()),
        );
        let mut state = WorldState::new(&content);
        for actor_id in ["blair", "blair-2", "casey", "casey-2"] {
            state.set_follows_player(actor_id, true);
        }
        let mut lines = NarrativeLines::default();

        handle_player_moved(&mut state, &content, "kitchen", &mut lines);

        assert_eq!(
            lines
                .0
                .iter()
                .filter(|line| line.text.contains("party"))
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>(),
            vec!["Your party follows close behind."]
        );
    }

    #[test]
    fn packs_without_a_party_summary_keep_per_member_narration() {
        let mut content = minimal_test_pack();
        content.settings.combat.player_actor_id = "player".to_string();
        content.settings.combat.health_stat_id = "stamina".to_string();
        content.messages.insert(
            "follow.actor_follows".to_string(),
            PackMessage::Narration("{actor} moves with you.".to_string()),
        );
        let mut state = WorldState::new(&content);
        state.set_follows_player("blair", true);
        state.set_follows_player("casey", true);
        let mut lines = NarrativeLines::default();

        handle_player_moved(&mut state, &content, "kitchen", &mut lines);

        assert_eq!(
            lines
                .0
                .iter()
                .filter(|line| line.text.contains("moves with you"))
                .count(),
            2
        );
    }
}
