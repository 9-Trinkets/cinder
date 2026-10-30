use super::{ContentValidator, Diagnostics};
use crate::content::types::{AdvanceEffect, AdvanceSignal, Condition, ContentPack};

/// Checks every condition in the pack, wherever it is written: a condition that
/// names a feature state must name a feature that has that state, and every
/// interaction must be well formed.
pub(super) struct ConditionsValidator;

impl ContentValidator for ConditionsValidator {
    fn validate(&self, pack: &ContentPack, diagnostics: &mut Diagnostics) {
        let mut found: Vec<(String, &Condition)> = Vec::new();
        collect_conditions(pack, &mut found);
        for (place, condition) in found {
            check_condition(pack, &place, condition, diagnostics);
        }
        check_features(pack, diagnostics);
    }
}

fn add<'a>(found: &mut Vec<(String, &'a Condition)>, place: String, list: &'a [Condition]) {
    for condition in list {
        found.push((place.clone(), condition));
    }
}

fn collect_conditions<'a>(pack: &'a ContentPack, found: &mut Vec<(String, &'a Condition)>) {
    for room in &pack.rooms {
        for exit in &room.exits {
            add(
                found,
                format!("exit {} to {}", room.id, exit.room_id),
                &exit.when,
            );
        }
        for description in &room.descriptions {
            add(
                found,
                format!("room {} description", room.id),
                &description.when,
            );
        }
        for feature in &room.features {
            for description in &feature.descriptions {
                add(
                    found,
                    format!("feature {}/{} description", room.id, feature.id),
                    &description.when,
                );
            }
            for interaction in &feature.interactions {
                add(
                    found,
                    format!("interaction {}/{}:{}", room.id, feature.id, interaction.id),
                    &interaction.when,
                );
            }
        }
    }
    for actor in &pack.actors {
        add(
            found,
            format!("actor {} unlock", actor.id),
            &actor.unlocked_when,
        );
        for interaction in &actor.interactions {
            add(
                found,
                format!("interaction {}:{}", actor.id, interaction.id),
                &interaction.when,
            );
        }
    }
    for action in &pack.actions {
        add(
            found,
            format!("action {}", action.id),
            &action.available.when,
        );
        if let Some(creation) = &action.item_creation {
            for (item, when) in &creation.craftable_item_gates {
                add(
                    found,
                    format!("action {} craftable {item}", action.id),
                    when,
                );
            }
        }
    }
    for stage in &pack.beats.stages {
        for signal in &stage.advance_signals {
            if let AdvanceSignal::Conditional { conditions, .. } = signal {
                add(found, format!("stage {}", stage.id), conditions);
            }
        }
    }
    for sequence in &pack.sequences.sequences {
        add(found, format!("sequence {}", sequence.id), &sequence.when);
    }
    for channel in &pack.settings.channels {
        add(found, format!("channel {}", channel.id), &channel.when);
    }
    for section in &pack.settings.sidebar.sections {
        add(
            found,
            format!("sidebar section {}", section.id),
            &section.when,
        );
    }
    add(
        found,
        "fallback stage".to_string(),
        &pack.settings.fallback_when,
    );
    add(
        found,
        "minimap entity tracking".to_string(),
        &pack.settings.minimap_entity_tracking_when,
    );
    for anchor in &pack.teleports.permanent_anchors {
        add(
            found,
            format!("teleport anchor {}", anchor.room_id),
            &anchor.when,
        );
    }
    for map in &pack.maps {
        add(found, format!("map {} reveal", map.id), &map.reveal_when);
    }
}

fn check_condition(
    pack: &ContentPack,
    place: &str,
    condition: &Condition,
    diagnostics: &mut Diagnostics,
) {
    match condition {
        Condition::FeatureIs { feature, state } => {
            if !feature_has_state(pack, feature, state) {
                diagnostics.push(format!(
                    "{place} waits for feature '{feature}' to be '{state}', which no feature declares"
                ));
            }
        }
        Condition::Not(inner) => check_condition(pack, place, inner, diagnostics),
        Condition::Any(group) | Condition::All(group) => {
            for inner in group {
                check_condition(pack, place, inner, diagnostics);
            }
        }
        _ => {}
    }
}

/// A feature reference is `room/feature`, or just `feature` for any room.
fn feature_has_state(pack: &ContentPack, reference: &str, state: &str) -> bool {
    pack.rooms.iter().any(|room| {
        room.features.iter().any(|feature| {
            let named = match reference.split_once('/') {
                Some((room_id, feature_id)) => room.id == room_id && feature.id == feature_id,
                None => feature.id == reference,
            };
            named && feature.states.iter().any(|declared| declared == state)
        })
    })
}

fn check_features(pack: &ContentPack, diagnostics: &mut Diagnostics) {
    for room in &pack.rooms {
        for feature in &room.features {
            let at = format!("feature {}/{}", room.id, feature.id);
            if !feature.state.is_empty() && !feature.states.contains(&feature.state) {
                diagnostics.push(format!(
                    "{at} starts in '{}', which is not one of its states",
                    feature.state
                ));
            }
            let mut seen = std::collections::BTreeSet::new();
            for state in &feature.states {
                if state.trim().is_empty() || !seen.insert(state.as_str()) {
                    diagnostics.push(format!("{at} lists a state twice or leaves one empty"));
                }
            }
            check_interactions(
                pack,
                &at,
                &feature.interactions,
                Some((&room.id, feature)),
                diagnostics,
            );
        }
    }
    for actor in &pack.actors {
        check_interactions(
            pack,
            &format!("actor {}", actor.id),
            &actor.interactions,
            None,
            diagnostics,
        );
    }
}

fn check_interactions(
    pack: &ContentPack,
    at: &str,
    interactions: &[crate::content::types::Interaction],
    owner_feature: Option<(&String, &crate::content::types::RoomFeatureDefinition)>,
    diagnostics: &mut Diagnostics,
) {
    let mut seen = std::collections::BTreeSet::new();
    for interaction in interactions {
        if interaction.id.trim().is_empty() || interaction.label.trim().is_empty() {
            diagnostics.push(format!("{at} has an interaction with no id or label"));
        }
        if !seen.insert(interaction.id.as_str()) {
            diagnostics.push(format!(
                "{at} has two interactions called '{}'",
                interaction.id
            ));
        }
        for effect in &interaction.effects {
            let AdvanceEffect::SetFeatureState { feature, state } = effect else {
                continue;
            };
            let valid = if feature.is_empty() {
                owner_feature.is_some_and(|(_, owner)| owner.states.contains(state))
            } else {
                feature_has_state(pack, feature, state)
            };
            if !valid {
                diagnostics.push(format!(
                    "{at} interaction '{}' sets feature '{}' to '{state}', which is not a state it has",
                    interaction.id,
                    if feature.is_empty() { "its owner" } else { feature }
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::{
        Interaction, RoomDefinition, RoomExitDefinition, RoomFeatureDefinition,
    };
    use crate::engine::test_fixtures::minimal_test_pack;

    fn lamp(interactions: Vec<Interaction>) -> RoomFeatureDefinition {
        RoomFeatureDefinition {
            id: "lamp".to_string(),
            label: "lamp".to_string(),
            states: vec!["off".to_string(), "dim".to_string()],
            interactions,
            ..Default::default()
        }
    }

    fn room(
        features: Vec<RoomFeatureDefinition>,
        exits: Vec<RoomExitDefinition>,
    ) -> RoomDefinition {
        RoomDefinition {
            id: "den".to_string(),
            title: "Den".to_string(),
            summary: String::new(),
            inspect_text: String::new(),
            allow_rest: false,
            floor: None,
            features,
            exits,
            descriptions: Vec::new(),
            items: Default::default(),
        }
    }

    fn problems(pack: &ContentPack) -> String {
        let mut diagnostics = Diagnostics::default();
        ConditionsValidator.validate(pack, &mut diagnostics);
        diagnostics
            .finish()
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default()
    }

    #[test]
    fn a_condition_on_a_state_no_feature_has_is_reported() {
        let mut pack = minimal_test_pack();
        pack.rooms.push(room(
            vec![lamp(Vec::new())],
            vec![RoomExitDefinition {
                room_id: "hall".to_string(),
                label: "hall".to_string(),
                aliases: Vec::new(),
                menu_label: None,
                when: vec![Condition::FeatureIs {
                    feature: "lamp".to_string(),
                    state: "bright".to_string(),
                }],
            }],
        ));
        let message = problems(&pack);
        assert!(message.contains("'lamp' to be 'bright'"), "{message}");
    }

    #[test]
    fn a_declared_state_is_fine() {
        let mut pack = minimal_test_pack();
        pack.rooms.push(room(
            vec![lamp(Vec::new())],
            vec![RoomExitDefinition {
                room_id: "hall".to_string(),
                label: "hall".to_string(),
                aliases: Vec::new(),
                menu_label: None,
                when: vec![Condition::FeatureIs {
                    feature: "den/lamp".to_string(),
                    state: "dim".to_string(),
                }],
            }],
        ));
        assert_eq!(problems(&pack), "");
    }

    #[test]
    fn an_interaction_must_set_a_state_its_feature_has() {
        let mut pack = minimal_test_pack();
        let bad = Interaction {
            id: "blaze".to_string(),
            label: "Blaze".to_string(),
            when: Vec::new(),
            lines: Vec::new(),
            effects: vec![AdvanceEffect::SetFeatureState {
                feature: String::new(),
                state: "bright".to_string(),
            }],
            repeatable: true,
            ..Default::default()
        };
        pack.rooms.push(room(vec![lamp(vec![bad])], Vec::new()));
        let message = problems(&pack);
        assert!(message.contains("not a state it has"), "{message}");
    }

    #[test]
    fn a_feature_must_start_in_one_of_its_states() {
        let mut pack = minimal_test_pack();
        let mut broken = lamp(Vec::new());
        broken.state = "glowing".to_string();
        pack.rooms.push(room(vec![broken], Vec::new()));
        let message = problems(&pack);
        assert!(message.contains("starts in 'glowing'"), "{message}");
    }
}
