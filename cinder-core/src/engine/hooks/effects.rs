use crate::content::types::ContentPack;
use crate::engine::narrative::{NarrativeLines, PendingCommentaryUpgrade};
use crate::engine::reducer::handlers::push_rendered_message;
use crate::engine::state::{
    ActorStance, ConversationMemoryKind, ConversationMemoryLine, SpawnActorConfig,
    SpawnActorOutcome, WorldState,
};
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
    /// Moves an actor to a designated room.
    MoveActor { actor_id: String, room_id: String },
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
                state.adjust_pair_stat(participant_a_id, participant_b_id, stat, *delta)?;
            }
            WorldHookEffect::AdjustActorStat {
                actor_id,
                stat,
                delta,
            } => {
                state.adjust_actor_stat(content, actor_id, stat, *delta)?;
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
            WorldHookEffect::MoveActor { actor_id, room_id } => {
                state.set_actor_room(actor_id, room_id);
            }
            WorldHookEffect::SetStoryVar { key, value } => {
                self.apply_set_story_var(state, content, key, value, lines.as_deref_mut());
            }
            WorldHookEffect::GrantSkill { skill_id, actor_id } => {
                if content.skill(skill_id).is_none() {
                    return Err(format!("hook grants unknown skill '{skill_id}'"));
                }
                let actor_id = actor_id
                    .as_deref()
                    .unwrap_or(&content.settings.combat.player_actor_id);
                if state.actor(content, actor_id).is_none() {
                    return Err(format!("hook grants skill to unknown actor '{actor_id}'"));
                }
                state.grant_actor_skill(actor_id, skill_id);
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
                turn_number: state.turn_number,
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
