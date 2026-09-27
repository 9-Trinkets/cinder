use crate::content::types::{ActorDefinition, ContentPack, RoomDefinition};
use crate::engine::events::ObservationMode;
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::{WorldState, display_actor_name};

pub(super) fn actors_in_room<'a>(
    content: &'a ContentPack,
    state: &'a WorldState,
    room_id: &str,
) -> Vec<&'a ActorDefinition> {
    state
        .onstage_actors(content)
        .filter(|actor| {
            !content.is_player_actor(&actor.id)
                && !state.actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id)
                && state.actor_is_in_room(content, &actor.id, room_id)
        })
        .collect()
}

/// Collapse repeated names while preserving first-seen order, suffixing
/// duplicates with a count ("warden ×2").
pub(super) fn group_duplicate_names(names: &[String]) -> Vec<String> {
    let mut grouped: Vec<(String, usize)> = Vec::new();
    for name in names {
        if let Some(entry) = grouped.iter_mut().find(|(existing, _)| existing == name) {
            entry.1 += 1;
        } else {
            grouped.push((name.clone(), 1));
        }
    }
    grouped
        .into_iter()
        .map(|(name, count)| {
            if count > 1 {
                format!("{name} ×{count}")
            } else {
                name
            }
        })
        .collect()
}

pub(super) fn render_room_observation(
    content: &ContentPack,
    state: &WorldState,
    room_id: &str,
    mode: ObservationMode,
) -> Option<NarrativeLines> {
    let room = content.room(room_id)?;
    let health_stat_id = &content.settings.combat.health_stat_id;
    let active = room.descriptions.iter().find(|override_| {
        override_.actor_defeated.is_empty()
            || state.actor_is_defeated(&override_.actor_defeated, health_stat_id)
    });
    let body = match mode {
        ObservationMode::Summary => active
            .map(|override_| override_.summary.clone())
            .unwrap_or_else(|| room.summary.clone()),
        ObservationMode::Detailed => active
            .map(|override_| override_.inspect_text.clone())
            .unwrap_or_else(|| room.inspect_text.clone()),
    };
    let features = render_features(content, room);
    let (people_slot, party_slot) = render_occupants(content, state, room_id);
    let visible_exits = render_exits(content, state, room);
    let items = render_items(content, state, room_id);
    let objective = render_objective(content, state);
    let body_text = content.render_template(
        &content.presentation.presentation_text.room_observation,
        &[
            ("body", body.as_str()),
            ("features", features.as_str()),
            ("items", items.as_str()),
            ("people", people_slot.as_str()),
            ("party", party_slot.as_str()),
            ("exits", visible_exits.as_str()),
            ("objective", objective.as_str()),
        ],
    );
    let mut lines = NarrativeLines::default();
    lines.heading(format!("== {} ==", room.title));
    if !body_text.trim().is_empty() {
        lines.narration(body_text);
    }
    Some(lines)
}

fn render_features(content: &ContentPack, room: &RoomDefinition) -> String {
    if room.features.is_empty() {
        return String::new();
    }
    content.render_template(
        &content.presentation.presentation_text.features,
        &[(
            "features",
            &room
                .features
                .iter()
                .map(|feature| feature.label.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        )],
    )
}

fn render_exits(content: &ContentPack, state: &WorldState, room: &RoomDefinition) -> String {
    let visible_exits: Vec<&str> = room
        .exits
        .iter()
        .filter(|exit| {
            exit.requires_story_var.is_empty()
                || crate::engine::turn_policies::story_var_is_truthy(
                    state,
                    &exit.requires_story_var,
                )
        })
        .map(|exit| exit.label.as_str())
        .collect();
    if visible_exits.is_empty() {
        String::new()
    } else {
        content.render_template(
            &content.presentation.presentation_text.exits,
            &[("exits", &visible_exits.join(", "))],
        )
    }
}

fn render_items(content: &ContentPack, state: &WorldState, room_id: &str) -> String {
    let loose = state.loose_room_items(room_id);
    if loose.is_empty() {
        return String::new();
    }
    let described = loose
        .iter()
        .filter_map(|(id, _)| {
            content
                .item(id)
                .filter(|item| !item.is_takeable() && !item.look_description.is_empty())
                .map(|item| item.look_description.clone())
        })
        .collect::<Vec<_>>();
    let plain = loose
        .iter()
        .filter(|(id, _)| {
            content
                .item(id)
                .is_none_or(|item| item.is_takeable() || item.look_description.is_empty())
        })
        .map(|(id, count)| {
            let label = content.item_label(id);
            if *count > 1 {
                format!("{label} ×{count}")
            } else {
                label.to_string()
            }
        })
        .collect::<Vec<_>>();
    let mut out = String::new();
    if !described.is_empty() {
        out.push_str("\n\n");
        out.push_str(&described.join(" "));
    }
    if !plain.is_empty() {
        out.push_str(&content.render_template(
            &content.presentation.presentation_text.loose_items,
            &[("items", &plain.join(", "))],
        ));
    }
    out
}

fn render_actor_names(
    content: &ContentPack,
    state: &WorldState,
    actors: &[&ActorDefinition],
) -> Vec<String> {
    let suffix = |stance: crate::engine::state::ActorStance| match stance {
        crate::engine::state::ActorStance::Allied => {
            content.presentation.presentation_text.ally_suffix.clone()
        }
        crate::engine::state::ActorStance::Hostile => content
            .presentation
            .presentation_text
            .hostile_suffix
            .clone(),
        crate::engine::state::ActorStance::Neutral => String::new(),
    };
    let names = actors
        .iter()
        .map(|actor| {
            let name = display_actor_name(state, actor);
            format!("{name}{}", suffix(state.stance(&actor.id)))
        })
        .collect::<Vec<_>>();
    group_duplicate_names(&names)
}

fn render_occupants(content: &ContentPack, state: &WorldState, room_id: &str) -> (String, String) {
    let (party_actors, room_actors): (Vec<_>, Vec<_>) = actors_in_room(content, state, room_id)
        .into_iter()
        .partition(|actor| state.is_party_member(content, &actor.id));

    let people_text = if room_actors.is_empty() {
        String::new()
    } else {
        let names = render_actor_names(content, state, &room_actors);
        let refs = names.iter().map(String::as_str).collect::<Vec<_>>();
        content.render_template(
            &content.presentation.presentation_text.people,
            &[("people", &refs.join(", "))],
        )
    };

    let party_text = if party_actors.is_empty() {
        String::new()
    } else {
        let names = render_actor_names(content, state, &party_actors);
        let refs = names.iter().map(String::as_str).collect::<Vec<_>>();
        let party_template = if content
            .presentation
            .presentation_text
            .party
            .trim()
            .is_empty()
        {
            "Your party: {party}."
        } else {
            &content.presentation.presentation_text.party
        };
        content.render_template(party_template, &[("party", &refs.join(", "))])
    };

    if content
        .presentation
        .presentation_text
        .room_observation
        .contains("{party}")
    {
        let party_slot = if party_text.is_empty() {
            String::new()
        } else if people_text.is_empty() {
            format!("\n\n{}", party_text.trim())
        } else {
            format!("\n{}", party_text.trim())
        };
        (people_text, party_slot)
    } else {
        let combined = match (!people_text.is_empty(), !party_text.is_empty()) {
            (true, true) => format!("{people_text}\n{}", party_text.trim()),
            (true, false) => people_text,
            (false, true) => format!("\n\n{}", party_text.trim()),
            (false, false) => String::new(),
        };
        (combined, String::new())
    }
}

pub(super) fn render_objective(content: &ContentPack, state: &WorldState) -> String {
    let summary = state
        .active_objective_stage_ids
        .first()
        .and_then(|stage_id| {
            content
                .beats
                .stages
                .iter()
                .find(|stage| stage.id == *stage_id)
        })
        .map(|stage| render_story_text(&stage.summary, state))
        .unwrap_or_default();
    if summary.is_empty() {
        return String::new();
    }
    content.render_template(
        &content.presentation.presentation_text.objective,
        &[("objective", &summary)],
    )
}

pub(super) fn render_feature_consumables_line(
    content: &ContentPack,
    state: &WorldState,
    room_id: &str,
    feature_id: &str,
) -> Option<String> {
    let feature = content.feature(room_id, feature_id)?;
    let available = feature
        .consumables
        .iter()
        .filter(|consumable| {
            state.remaining_consumable_stock(room_id, feature_id, &consumable.id) > 0
                || state.has_item_in_storage(
                    &consumable.id,
                    crate::content::types::ItemStorageTarget::CurrentRoom,
                    room_id,
                )
        })
        .map(|consumable| consumable.label.as_str())
        .collect::<Vec<_>>();
    if available.is_empty() {
        return None;
    }
    Some(content.render_template(
        &content.presentation.presentation_text.feature_consumables,
        &[
            ("feature_label", feature.label.as_str()),
            ("items", &available.join(", ")),
        ],
    ))
}

pub(crate) fn render_actor_speech_line(
    content: &ContentPack,
    actor_name: &str,
    target_name: Option<&str>,
    text: &str,
) -> String {
    let template = match target_name.filter(|target| !target.trim().is_empty()) {
        Some(_) => &content.presentation.presentation_text.actor_targeted_speech,
        None => &content.presentation.presentation_text.actor_speech,
    };
    content.render_template(
        template,
        &[
            ("actor_name", actor_name),
            ("target_name", target_name.unwrap_or("")),
            ("text", text),
        ],
    )
}

pub(super) fn render_story_text(template: &str, state: &WorldState) -> String {
    let mut rendered = state.story_vars.render_template(template);
    for (actor_id, stats) in &state.actor_stats {
        for (stat_key, stat_value) in stats {
            rendered = rendered.replace(
                &format!("{{actor.{actor_id}.{stat_key}}}"),
                &stat_value.to_string(),
            );
        }
    }
    rendered
}
