use crate::content::types::{ActorDefinition, ContentPack};
use crate::engine::events::ObservationMode;
use crate::engine::state::WorldState;

pub(super) fn actors_in_room<'a>(
    content: &'a ContentPack,
    state: &WorldState,
    room_id: &str,
) -> Vec<&'a ActorDefinition> {
    content
        .actors
        .iter()
        .filter(|actor| state.actor_room_id(&actor.id, &actor.room_id) == room_id)
        .collect()
}

pub(super) fn render_room_observation(
    content: &ContentPack,
    state: &WorldState,
    room_id: &str,
    mode: ObservationMode,
) -> Option<String> {
    let room = content.room(room_id)?;
    let body = match mode {
        ObservationMode::Summary => room.summary.clone(),
        ObservationMode::Detailed => room.inspect_text.clone(),
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
        let present = actors_in_room(content, state, room_id)
            .into_iter()
            .map(|actor| actor.name.as_str())
            .collect::<Vec<_>>();
        if present.is_empty() {
            String::new()
        } else {
            content.render_template(
                &content.presentation.presentation_text.people,
                &[("people", &present.join(", "))],
            )
        }
    };
    let exits = if room.exits.is_empty() {
        String::new()
    } else {
        content.render_template(
            &content.presentation.presentation_text.exits,
            &[(
                "exits",
                &room
                    .exits
                    .iter()
                    .map(|exit| exit.label.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            )],
        )
    };
    let objective = render_objective(content, state);
    Some(content.render_template(
        &content.presentation.presentation_text.room_observation,
        &[
            ("room_title", room.title.as_str()),
            ("body", body.as_str()),
            ("features", features.as_str()),
            ("people", people.as_str()),
            ("exits", exits.as_str()),
            ("objective", objective.as_str()),
        ],
    ))
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
    let mut rendered = template.to_string();
    for (key, value) in &state.story_vars {
        rendered = rendered.replace(&format!("{{{key}}}"), value);
    }
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
