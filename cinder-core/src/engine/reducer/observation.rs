use crate::content::types::{ActorDefinition, ContentPack};
use crate::engine::events::ObservationMode;
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::{WorldState, display_actor_name};

pub(super) fn actors_in_room<'a>(
    content: &'a ContentPack,
    state: &WorldState,
    room_id: &str,
) -> Vec<&'a ActorDefinition> {
    content
        .actors
        .iter()
        .filter(|actor| {
            !state.actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id)
                && state.actor_room_id(&actor.id, &actor.room_id) == room_id
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
    let features = if room.features.is_empty() {
        String::new()
    } else {
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
    };
    let people = {
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
        let present = actors_in_room(content, state, room_id)
            .into_iter()
            .map(|actor| {
                let name = display_actor_name(state, actor);
                format!("{name}{}", suffix(state.stance(&actor.id)))
            })
            .collect::<Vec<_>>();
        if present.is_empty() {
            String::new()
        } else {
            let grouped = group_duplicate_names(&present);
            let grouped_refs = grouped.iter().map(String::as_str).collect::<Vec<_>>();
            content.render_template(
                &content.presentation.presentation_text.people,
                &[("people", &grouped_refs.join(", "))],
            )
        }
    };
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
    let exits = if visible_exits.is_empty() {
        String::new()
    } else {
        content.render_template(
            &content.presentation.presentation_text.exits,
            &[("exits", &visible_exits.join(", "))],
        )
    };
    let items = {
        let loose = state.loose_room_items(room_id);
        if loose.is_empty() {
            String::new()
        } else {
            // Items with a `look_description` read as part of the room; the
            // rest are listed generically as loose items on the ground.
            let described = loose
                .iter()
                .filter_map(|(id, _)| {
                    content
                        .item(id)
                        .filter(|item| !item.look_description.is_empty())
                        .map(|item| item.look_description.clone())
                })
                .collect::<Vec<_>>();
            let plain = loose
                .iter()
                .filter(|(id, _)| {
                    content
                        .item(id)
                        .map_or(true, |item| item.look_description.is_empty())
                })
                .map(|(id, count)| {
                    let label = content
                        .item(id)
                        .map(|item| item.label.as_str())
                        .unwrap_or(id);
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
    };
    let objective = render_objective(content, state);
    let body_text = content.render_template(
        &content.presentation.presentation_text.room_observation,
        &[
            ("body", body.as_str()),
            ("features", features.as_str()),
            ("items", items.as_str()),
            ("people", people.as_str()),
            ("exits", exits.as_str()),
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
    let room = content.room(room_id)?;
    let feature = room
        .features
        .iter()
        .find(|feature| feature.id == feature_id)?;
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
