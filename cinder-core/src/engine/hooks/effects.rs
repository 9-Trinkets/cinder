use crate::content::types::ContentPack;
use crate::engine::narrative::{NarrativeLines, PendingCommentaryUpgrade};
use crate::engine::reducer::handlers::push_rendered_message;
use crate::engine::state::{
    ActorStance, ConversationMemoryKind, ConversationMemoryLine, SpawnActorConfig,
    SpawnActorOutcome, WorldState,
};
use crate::engine::world_mutation::{WorldMutation, WorldMutationApplier};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum WorldHookEffect {
    AdjustPairStat {
        participant_a_id: String,
        participant_b_id: String,
        stat: String,
        #[serde(deserialize_with = "deserialize_i32ish")]
        delta: i32,
    },
    AdjustActorStat {
        actor_id: String,
        stat: String,
        #[serde(deserialize_with = "deserialize_i32ish")]
        delta: i32,
    },
    /// Turns an actor into an ally (and optionally a follower). `messages` are
    /// pack-authored message keys rendered with `{actor}` for narration.
    ConvertActorToAlly {
        actor_id: String,
        #[serde(default)]
        follows_player: bool,
        #[serde(default)]
        messages: Vec<String>,
    },
    /// Sets an actor's stance and follower state directly.
    SetActorStance {
        actor_id: String,
        stance: ActorStance,
        #[serde(default)]
        follows_player: bool,
        #[serde(default)]
        messages: Vec<String>,
    },
    /// Sets the stance of matching living actors carrying `tag`.
    SetStanceByTag(SetStanceByTagEffect),
    /// Sets a content-authored standing order for any actor, driving
    /// AI-controlled parties without requiring the actor to be allied.
    SetPartyOrder { actor_id: String, order: String },
    /// Sets a content-authored standing order for every living actor carrying
    /// `tag` (e.g. advance a whole role group reactively).
    SetPartyOrderByTag { tag: String, order: String },
    /// Moves one or more actors to a designated room.
    #[serde(alias = "move_actor")]
    MoveActors {
        #[serde(default)]
        actor_id: Option<String>,
        #[serde(default)]
        tag: Option<String>,
        #[serde(default)]
        stance: Option<ActorStance>,
        room_id: String,
    },
    /// Sets a story variable (e.g. a flag marking a boss as defeated).
    SetStoryVar { key: String, value: String },
    /// Grants a declared skill to an actor. Omitted actor id targets the player.
    GrantSkill {
        skill_id: String,
        #[serde(default)]
        actor_id: Option<String>,
    },
    AcquireItem {
        item_id: String,
        #[serde(default)]
        from_actor_id: Option<String>,
    },
    /// Defeats every living actor carrying `tag`.
    DefeatActorsByTag { tag: String },
    /// Plot beat rendered through the pack's message table.
    NarrateMessage {
        key: String,
        #[serde(default)]
        vars: Vec<(String, String)>,
        #[serde(default)]
        generate_commentary: bool,
    },
    /// Instantiates a new runtime actor from an authored template.
    SpawnActor {
        template_id: String,
        #[serde(default)]
        room_id: Option<String>,
        #[serde(default)]
        stance: Option<ActorStance>,
        #[serde(default)]
        follows_player: bool,
        #[serde(default)]
        messages: Vec<String>,
        #[serde(default)]
        scale_with_actor_id: Option<String>,
        #[serde(default)]
        scale_stat: Option<String>,
        #[serde(default)]
        max_active_instances: Option<usize>,
        #[serde(default)]
        max_instances_messages: Vec<String>,
    },
    /// In-room character speech rendered in dialogue style and tracked in conversation memory.
    ActorSpeech(ActorSpeechEffect),
}

/// Payload for [`WorldHookEffect::SetStanceByTag`].
#[derive(Debug, Deserialize)]
pub(crate) struct SetStanceByTagEffect {
    pub tag: String,
    pub stance: ActorStance,
    #[serde(default)]
    pub from_stances: Vec<ActorStance>,
    #[serde(default)]
    pub follows_player: bool,
    #[serde(default)]
    pub messages: Vec<String>,
}

/// Payload for [`WorldHookEffect::ActorSpeech`].
#[derive(Debug, Deserialize)]
pub(crate) struct ActorSpeechEffect {
    pub actor_id: String,
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub vars: Vec<(String, String)>,
}

impl WorldHookEffect {
    pub fn apply(
        &self,
        state: &mut WorldState,
        content: &ContentPack,
        input: &Value,
        mut lines: Option<&mut NarrativeLines>,
    ) -> Result<(), String> {
        match self {
            WorldHookEffect::AdjustPairStat {
                participant_a_id,
                participant_b_id,
                stat,
                delta,
            } => {
                WorldMutationApplier::new(state, content).apply(
                    &WorldMutation::AdjustPairStat {
                        participant_a_id: participant_a_id.clone(),
                        participant_b_id: participant_b_id.clone(),
                        stat: stat.clone(),
                        delta: *delta,
                    },
                )?;
            }
            WorldHookEffect::AdjustActorStat {
                actor_id,
                stat,
                delta,
            } => {
                WorldMutationApplier::new(state, content).apply(
                    &WorldMutation::AdjustActorStat {
                        actor_id: actor_id.clone(),
                        stat: stat.clone(),
                        delta: *delta,
                    },
                )?;
            }
            WorldHookEffect::ConvertActorToAlly {
                actor_id,
                follows_player,
                messages,
            } => {
                self.apply_convert_actor_to_ally(
                    state,
                    content,
                    actor_id,
                    *follows_player,
                    messages,
                    lines.as_deref_mut(),
                );
            }
            WorldHookEffect::SetActorStance {
                actor_id,
                stance,
                follows_player,
                messages,
            } => {
                self.apply_set_actor_stance(
                    state,
                    content,
                    actor_id,
                    *stance,
                    *follows_player,
                    messages,
                    lines.as_deref_mut(),
                );
            }
            WorldHookEffect::SetStanceByTag(effect) => {
                self.apply_set_stance_by_tag(state, content, effect, lines.as_deref_mut());
            }
            WorldHookEffect::SetPartyOrder { actor_id, order } => {
                state.set_authored_party_order(actor_id, order.clone());
            }
            WorldHookEffect::SetPartyOrderByTag { tag, order } => {
                state.set_party_order_by_tag(content, tag, order);
            }
            WorldHookEffect::MoveActors {
                actor_id,
                tag,
                stance,
                room_id,
            } => {
                self.apply_move_actors(
                    state,
                    content,
                    actor_id.as_deref(),
                    tag.as_deref(),
                    *stance,
                    room_id,
                    lines.as_deref_mut(),
                );
            }
            WorldHookEffect::SetStoryVar { key, value } => {
                self.apply_set_story_var(state, content, key, value, lines.as_deref_mut());
            }
            WorldHookEffect::GrantSkill { skill_id, actor_id } => {
                self.apply_grant_skill(state, content, skill_id, actor_id.as_deref())?;
            }
            WorldHookEffect::AcquireItem {
                item_id,
                from_actor_id,
            } => {
                self.apply_acquire_item(
                    state,
                    content,
                    item_id,
                    from_actor_id.as_deref(),
                    lines.as_deref_mut(),
                );
            }
            WorldHookEffect::NarrateMessage {
                key,
                vars,
                generate_commentary,
            } => {
                self.apply_narrate_message(
                    content,
                    input,
                    key,
                    vars,
                    *generate_commentary,
                    lines.as_deref_mut(),
                );
            }
            WorldHookEffect::DefeatActorsByTag { tag } => {
                state.defeat_actors_by_tag(content, tag);
            }
            WorldHookEffect::SpawnActor { .. } => {
                self.apply_spawn_actor(state, content, lines.as_deref_mut());
            }
            WorldHookEffect::ActorSpeech(effect) => {
                self.apply_actor_speech(state, content, effect, lines);
            }
        }
        Ok(())
    }

    fn apply_convert_actor_to_ally(
        &self,
        state: &mut WorldState,
        content: &ContentPack,
        actor_id: &str,
        follows_player: bool,
        messages: &[String],
        lines: Option<&mut NarrativeLines>,
    ) {
        self.apply_set_actor_stance(
            state,
            content,
            actor_id,
            ActorStance::Allied,
            follows_player,
            messages,
            lines,
        );
    }

    fn apply_set_actor_stance(
        &self,
        state: &mut WorldState,
        content: &ContentPack,
        actor_id: &str,
        stance: ActorStance,
        follows_player: bool,
        messages: &[String],
        lines: Option<&mut NarrativeLines>,
    ) {
        state.set_actor_stance(content, actor_id, stance, follows_player);
        if let Some(lines) = lines {
            let actor_name = content
                .actor(actor_id)
                .map(|actor| actor.name.as_str())
                .unwrap_or(actor_id);
            for key in messages {
                if let Some(line) = content.render_message(key, &[("actor", actor_name)]) {
                    lines.narration(line);
                }
            }
        }
    }

    fn apply_set_stance_by_tag(
        &self,
        state: &mut WorldState,
        content: &ContentPack,
        effect: &SetStanceByTagEffect,
        lines: Option<&mut NarrativeLines>,
    ) {
        let SetStanceByTagEffect {
            tag,
            stance,
            from_stances,
            follows_player,
            messages,
        } = effect;
        let changed =
            state.set_actor_stance_by_tag(content, tag, *stance, from_stances, *follows_player);
        if let Some(lines) = lines {
            for (_actor_id, actor_name) in changed {
                for key in messages {
                    if let Some(line) =
                        content.render_message(key, &[("actor", actor_name.as_str())])
                    {
                        lines.narration(line);
                    }
                }
            }
        }
    }

    fn apply_move_actors(
        &self,
        state: &mut WorldState,
        content: &ContentPack,
        actor_id: Option<&str>,
        tag: Option<&str>,
        stance: Option<ActorStance>,
        room_id: &str,
        lines: Option<&mut NarrativeLines>,
    ) {
        let target_room_id = if room_id == "player" || room_id == "current_room" {
            state.current_room_id.clone()
        } else {
            room_id.to_string()
        };

        if let Some(tag) = tag {
            Self::apply_move_tagged_actors(state, content, tag, stance, &target_room_id);
        }

        if let Some(actor_id) = actor_id {
            self.apply_move_single_actor(state, content, actor_id, &target_room_id, lines);
        }
    }

    fn apply_move_tagged_actors(
        state: &mut WorldState,
        content: &ContentPack,
        tag: &str,
        stance: Option<ActorStance>,
        target_room_id: &str,
    ) {
        let health_stat_id = &content.settings.combat.health_stat_id;
        let matching_actors: Vec<String> = state
            .actors(content)
            .filter(|actor| {
                !content.is_player_actor(&actor.id)
                    && actor.tags.iter().any(|actor_tag| actor_tag.as_str() == tag)
                    && !state.actor_is_defeated(&actor.id, health_stat_id)
                    && !state.actor_is_offstage(content, &actor.id)
                    && !state.actor_is_imprisoned(content, &actor.id)
                    && stance.map_or(true, |s| state.stance(&actor.id) == s)
            })
            .map(|actor| actor.id.clone())
            .collect();

        for id in matching_actors {
            state.mark_actor_room_visited(&id, target_room_id);
            state.set_actor_room(&id, target_room_id);
        }
    }

    fn apply_move_single_actor(
        &self,
        state: &mut WorldState,
        content: &ContentPack,
        actor_id: &str,
        target_room_id: &str,
        lines: Option<&mut NarrativeLines>,
    ) {
        let is_player =
            actor_id == "player" || actor_id == content.settings.combat.player_actor_id.as_str();
        if is_player {
            self.apply_move_player(state, content, target_room_id, lines);
        } else {
            state.mark_actor_room_visited(actor_id, target_room_id);
            state.set_actor_room(actor_id, target_room_id);
        }
    }

    fn apply_move_player(
        &self,
        state: &mut WorldState,
        content: &ContentPack,
        target_room_id: &str,
        lines: Option<&mut NarrativeLines>,
    ) {
        let from_room_id = state.current_room_id.clone();
        if from_room_id == target_room_id {
            return;
        }
        let first_visit =
            !state.actor_has_visited_room(&content.settings.combat.player_actor_id, target_room_id);
        state.current_room_id = target_room_id.to_string();
        state.mark_actor_room_visited(&content.settings.combat.player_actor_id, target_room_id);
        state.set_actor_room(&content.settings.combat.player_actor_id, target_room_id);
        state.set_actor_room("player", target_room_id);
        if let Some(lines) = lines {
            lines.extend_lines(
                crate::engine::reducer::beat_advance::advance_objective_for_signal(
                    state,
                    content,
                    &format!("room_left:{from_room_id}"),
                ),
            );
            lines.extend_lines(
                crate::engine::reducer::beat_advance::advance_objective_for_signal(
                    state,
                    content,
                    &format!("room_entered:{target_room_id}"),
                ),
            );
            lines.extend_lines(
                crate::engine::reducer::beat_advance::advance_objective_for_signal(
                    state,
                    content,
                    &format!("player_moved:{target_room_id}"),
                ),
            );
            lines.extend_lines(
                crate::engine::reducer::beat_advance::advance_objective_for_signal(
                    state,
                    content,
                    "player_moved",
                ),
            );
            let _ = crate::engine::hooks::apply_narrating_world_hook_effects(
                state,
                content,
                crate::engine::hook_ids::PLAYER_MOVED,
                serde_json::json!({
                    "from_room_id": from_room_id,
                    "to_room_id": target_room_id,
                    "first_visit": first_visit,
                    "story_vars": state.story_vars.to_map(),
                }),
                lines,
            );
        }
    }

    fn apply_grant_skill(
        &self,
        state: &mut WorldState,
        content: &ContentPack,
        skill_id: &str,
        actor_id: Option<&str>,
    ) -> Result<(), String> {
        if content.skill(skill_id).is_none() {
            return Err(format!("hook grants unknown skill '{skill_id}'"));
        }
        let actor_id = actor_id.unwrap_or(&content.settings.combat.player_actor_id);
        if state.actor(content, actor_id).is_none() {
            return Err(format!("hook grants skill to unknown actor '{actor_id}'"));
        }
        state.grant_actor_skill(actor_id, skill_id);
        Ok(())
    }

    fn apply_set_story_var(
        &self,
        state: &mut WorldState,
        content: &ContentPack,
        key: &str,
        value: &str,
        lines: Option<&mut NarrativeLines>,
    ) {
        state.story_vars.set_unchecked(key, value);
        if let Some(lines) = lines {
            lines.extend_lines(
                crate::engine::reducer::beat_advance::advance_objective_for_signal(
                    state, content, key,
                ),
            );
        }
    }

    fn apply_acquire_item(
        &self,
        state: &mut WorldState,
        content: &ContentPack,
        item_id: &str,
        from_actor_id: Option<&str>,
        lines: Option<&mut NarrativeLines>,
    ) {
        if let Some(actor_id) = from_actor_id {
            state.actor_remove_item(actor_id, item_id);
        }
        state.acquire_player_item(content, item_id);
        if let Some(lines) = lines {
            let label = content.item_label(item_id);
            let line = content
                .render_message(
                    "item.acquired_inventory",
                    &[("label", label), ("item", label)],
                )
                .unwrap_or_else(|| format!("You received the {label}."));
            push_rendered_message(
                lines,
                content,
                line,
                content.message_voice("item.acquired_inventory"),
            );
            lines.extend_lines(
                crate::engine::reducer::beat_advance::advance_objective_for_signal(
                    state,
                    content,
                    &format!("item_acquired:{item_id}"),
                ),
            );
            lines.extend_lines(
                crate::engine::reducer::beat_advance::advance_objective_for_signal(
                    state,
                    content,
                    "item_acquired",
                ),
            );
        }
    }

    fn apply_narrate_message(
        &self,
        content: &ContentPack,
        input: &Value,
        key: &str,
        vars: &[(String, String)],
        generate_commentary: bool,
        lines: Option<&mut NarrativeLines>,
    ) {
        let Some(lines) = lines else {
            return;
        };
        let replacements: Vec<(&str, &str)> =
            vars.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let Some(line) = content.render_message(key, &replacements) else {
            return;
        };
        push_rendered_message(lines, content, line.clone(), content.message_voice(key));
        if generate_commentary {
            let from_room_id = input
                .get("from_room_id")
                .and_then(Value::as_str)
                .map(str::to_string);
            let to_room_id = input
                .get("to_room_id")
                .and_then(Value::as_str)
                .map(str::to_string);
            if let (Some(from_room_id), Some(to_room_id)) = (from_room_id, to_room_id)
                && let Some(last) = lines.0.last_mut()
            {
                last.pending_commentary_upgrade = Some(PendingCommentaryUpgrade {
                    from_room_id,
                    to_room_id,
                    fallback_text: line,
                });
            }
        }
    }

    fn apply_spawn_actor(
        &self,
        state: &mut WorldState,
        content: &ContentPack,
        lines: Option<&mut NarrativeLines>,
    ) {
        let WorldHookEffect::SpawnActor {
            template_id,
            room_id,
            stance,
            follows_player,
            messages,
            scale_with_actor_id,
            scale_stat,
            max_active_instances,
            max_instances_messages,
        } = self
        else {
            return;
        };

        let config = SpawnActorConfig {
            template_id,
            room_id: room_id.as_deref(),
            stance: *stance,
            follows_player: *follows_player,
            scale_with_actor_id: scale_with_actor_id.as_deref(),
            scale_stat: scale_stat.as_deref(),
            max_active_instances: *max_active_instances,
        };

        match state.spawn_actor(content, config) {
            SpawnActorOutcome::Success(info) => {
                if let Some(lines) = lines {
                    let hp_str = info.scaled_hp.to_string();
                    let str_str = info.scaled_str.to_string();
                    let intel_str = info.scaled_intel.to_string();
                    let scaler_val_str = info.scaler_val.to_string();
                    for key in messages {
                        if let Some(line) = content.render_message(
                            key,
                            &[
                                ("actor", info.actor_name.as_str()),
                                ("instance_id", info.instance_id.as_str()),
                                ("room_id", info.target_room_id.as_str()),
                                ("hp", hp_str.as_str()),
                                ("strength", str_str.as_str()),
                                ("intelligence", intel_str.as_str()),
                                ("intel", scaler_val_str.as_str()),
                            ],
                        ) {
                            push_rendered_message(lines, content, line, content.message_voice(key));
                        }
                    }
                }
            }
            SpawnActorOutcome::CapacityExceeded { max, template_name } => {
                if let Some(lines) = lines {
                    let max_str = max.to_string();
                    for key in max_instances_messages {
                        if let Some(line) = content.render_message(
                            key,
                            &[
                                ("actor", template_name.as_str()),
                                ("template_id", template_id.as_str()),
                                ("max", max_str.as_str()),
                            ],
                        ) {
                            push_rendered_message(lines, content, line, content.message_voice(key));
                        }
                    }
                }
            }
            SpawnActorOutcome::TemplateNotFound => {}
        }
    }

    fn apply_actor_speech(
        &self,
        state: &mut WorldState,
        content: &ContentPack,
        effect: &ActorSpeechEffect,
        lines: Option<&mut NarrativeLines>,
    ) {
        let ActorSpeechEffect {
            actor_id,
            target_id,
            text,
            key,
            vars,
        } = effect;
        let spoken_text = if let Some(key) = key.as_deref() {
            let replacements: Vec<(&str, &str)> =
                vars.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
            content
                .render_message(key, &replacements)
                .unwrap_or_default()
        } else if let Some(text) = text.as_deref() {
            text.to_string()
        } else {
            String::new()
        };
        if spoken_text.is_empty() {
            return;
        }

        let target_id = target_id.as_deref();
        let actor_name = content
            .actor(actor_id)
            .map(|actor| actor.name.as_str())
            .unwrap_or(actor_id);
        let target_name =
            target_id.and_then(|target| content.actor(target).map(|actor| actor.name.as_str()));
        let speech_line = crate::engine::reducer::render_actor_speech_line(
            content,
            actor_name,
            target_name,
            &spoken_text,
        );
        if let Some(lines) = lines {
            lines.narration(speech_line);
        }
        let target_recipient_id = target_id.unwrap_or(&content.settings.combat.player_actor_id);
        let target_recipient_name = content
            .actor(target_recipient_id)
            .map(|actor| actor.name.as_str())
            .unwrap_or(target_recipient_id);
        state.push_conversation_line(
            actor_id,
            target_recipient_id,
            ConversationMemoryLine {
                turn_number: state.clock.turn_number,
                event_sequence: 0,
                speaker_id: actor_id.to_string(),
                speaker_name: actor_name.to_string(),
                kind: ConversationMemoryKind::Speech,
                target_label: Some(target_recipient_name.to_string()),
                text: spoken_text,
            },
        );
    }
}

pub(crate) fn deserialize_i32ish<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Number(number) => number
            .as_i64()
            .ok_or_else(|| serde::de::Error::custom("delta must be an integer"))
            .and_then(|value| {
                i32::try_from(value).map_err(|_| serde::de::Error::custom("delta out of range"))
            }),
        Value::String(text) => text
            .parse::<i32>()
            .map_err(|_| serde::de::Error::custom("delta string must parse as i32")),
        other => Err(serde::de::Error::custom(format!(
            "delta must be an integer or integer string, got {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::test_fixtures::{minimal_test_pack, rebuild_test_pack_indexes};

    fn effect(value: serde_json::Value) -> WorldHookEffect {
        serde_json::from_value(value).expect("deserialize hook effect")
    }

    #[test]
    fn set_party_order_effect_drives_non_allied_ai_party_members() {
        let mut content = minimal_test_pack();
        content.settings.combat.health_stat_id = "stamina".to_string();
        content
            .actors
            .iter_mut()
            .find(|actor| actor.id == "casey")
            .expect("casey fixture")
            .tags = vec!["pawn".to_string()];
        rebuild_test_pack_indexes(&mut content);
        let mut state = WorldState::new(&content);
        state.set_stance("casey", ActorStance::Hostile);

        let single = effect(serde_json::json!({
            "kind": "set_party_order",
            "actor_id": "casey",
            "order": "deploy d7c1",
        }));
        single
            .apply(&mut state, &content, &serde_json::json!({}), None)
            .expect("apply single order");
        assert_eq!(
            state.party_order(&content, "casey"),
            Some("deploy d7c1".to_string())
        );

        let by_tag = effect(serde_json::json!({
            "kind": "set_party_order_by_tag",
            "tag": "pawn",
            "order": "advance",
        }));
        by_tag
            .apply(&mut state, &content, &serde_json::json!({}), None)
            .expect("apply order by tag");
        assert_eq!(
            state.party_order(&content, "casey"),
            Some("advance".to_string())
        );
    }

    #[test]
    fn move_actors_by_tag_and_alias_moves_actors() {
        let mut content = minimal_test_pack();
        content.settings.combat.health_stat_id = "stamina".to_string();
        content
            .actors
            .iter_mut()
            .find(|actor| actor.id == "casey")
            .expect("casey fixture")
            .tags = vec!["minion".to_string()];
        rebuild_test_pack_indexes(&mut content);
        let mut state = WorldState::new(&content);
        state.current_room_id = "room_b".to_string();
        state.set_actor_room("casey", "room_a");
        state.set_stance("casey", ActorStance::Allied);

        // move_actors by tag to "player"
        let move_tag = effect(serde_json::json!({
            "kind": "move_actors",
            "tag": "minion",
            "stance": "allied",
            "room_id": "player",
        }));
        move_tag
            .apply(&mut state, &content, &serde_json::json!({}), None)
            .expect("apply move_actors by tag");
        assert_eq!(state.actor_room_id("casey", ""), "room_b");

        // legacy alias "move_actor" for single actor to explicit room
        let move_single = effect(serde_json::json!({
            "kind": "move_actor",
            "actor_id": "casey",
            "room_id": "garden",
        }));
        move_single
            .apply(&mut state, &content, &serde_json::json!({}), None)
            .expect("apply move_actor alias");
        assert_eq!(state.actor_room_id("casey", ""), "garden");
    }
}
