use super::format_bullets;
use super::format_memory;
use super::join_non_empty_sections;
use super::render_prompt_template;
use super::{
    ActorTurnActionRequest, ActorTurnAffordanceOption, ActorTurnAffordanceTarget,
    ActorTurnCommandInvocation, ActorTurnSpeakCandidate, ChapterRelationshipSummaryRequest,
    ChapterScriptSummaryRequest, ConversationMemorySummaryRequest, DialogueRequest,
    DirectSpeechIntentRequest, MenuIntentRequest,
};
use crate::content::types::{CommandDefinition, SystemTextDefinition};

pub(crate) fn build_actor_turn_action_prompt(request: &ActorTurnActionRequest) -> String {
    let text = &request.system_text;
    let relationship_context = format_relationship_context(request);
    let available_actions = join_non_empty_sections(
        &request
            .affordances
            .iter()
            .map(|affordance| format!("- {}", affordance.available_text))
            .collect::<Vec<_>>(),
    );
    let decision_lines = join_non_empty_sections(
        &request
            .affordances
            .iter()
            .map(|affordance| format!("- {}", affordance.decision_label))
            .collect::<Vec<_>>(),
    );
    let template = &text.actor_turn_prompt_template;
    let decision_instruction = &text.actor_turn_decision_instruction;
    render_prompt_template(
        template,
        &[
            (
                "character",
                &format_bullets(&request.character_notes, &text.dialogue_no_character_facts),
            ),
            (
                "setting",
                &format_bullets(&request.setting_notes, &text.dialogue_no_setting_facts),
            ),
            (
                "current_beat",
                &format_bullets(
                    &request.current_beat_notes,
                    &text.dialogue_no_current_beat_facts,
                ),
            ),
            (
                "subtext",
                &format_bullets(&request.subtext_notes, &text.dialogue_no_subtext_facts),
            ),
            (
                "behavior_examples",
                &format_bullets(
                    &request.behavior_examples,
                    &text.dialogue_no_behavior_examples,
                ),
            ),
            (
                "recent_memory_note",
                &text.actor_turn_prompt_recent_memory_note,
            ),
            (
                "recent_memory",
                &format_memory(
                    &request.recent_memory,
                    &text.dialogue_no_recent_memory,
                    Some(request.actor_name.as_str()),
                ),
            ),
            (
                "relationship_status_label",
                &text.actor_turn_relationship_status_label,
            ),
            ("relationship_context", &relationship_context),
            (
                "available_actions_label",
                &text.actor_turn_available_actions_label,
            ),
            ("available_actions", &available_actions),
            ("decision_label", &text.menu_decision_label),
            ("decision_instruction", decision_instruction),
            ("decision_lines", &decision_lines),
        ],
    )
}

pub(crate) fn build_actor_turn_affordance_option(
    system_text: &SystemTextDefinition,
    affordance_id: &str,
    group: &str,
    prompt_verb: &str,
    _prompt_reply_verb: Option<&str>,
    command: &CommandDefinition,
    target: ActorTurnAffordanceTarget<'_>,
) -> ActorTurnAffordanceOption {
    let (available_text, decision_label, decision_prefix, invocation) = match target {
        ActorTurnAffordanceTarget::Move {
            room_id,
            room_title,
            actor_name,
        } => (
            match actor_name {
                Some(actor_name) => render_prompt_template(
                    &system_text.actor_turn_move_option_with_actor_template,
                    &[
                        ("prompt_verb", prompt_verb),
                        ("room_title", room_title),
                        ("actor_name", actor_name),
                    ],
                ),
                None => render_prompt_template(
                    &system_text.actor_turn_move_option_template,
                    &[("prompt_verb", prompt_verb), ("room_title", room_title)],
                ),
            },
            format!("{} {room_id}", command.command),
            None,
            ActorTurnCommandInvocation::Command {
                command_id: command.id.clone(),
                target_room_id: Some(room_id.to_string()),
                target_actor_id: None,
                feature_id: None,
                consumable_id: None,
                context_label: None,
                input_mode: command.input_mode,
            },
        ),
        ActorTurnAffordanceTarget::Speak {
            actor_id,
            actor_name,
            reply_now,
        } => (
            if reply_now {
                render_prompt_template(
                    &system_text.actor_turn_reply_option_template,
                    &[("actor_name", actor_name)],
                )
            } else {
                render_prompt_template(
                    &system_text.actor_turn_speak_option_template,
                    &[("actor_name", actor_name)],
                )
            },
            format!("SPEAK {actor_id}"),
            None,
            ActorTurnCommandInvocation::Command {
                command_id: command.id.clone(),
                target_room_id: None,
                target_actor_id: Some(actor_id.to_string()),
                feature_id: None,
                consumable_id: None,
                context_label: None,
                input_mode: command.input_mode,
            },
        ),
        ActorTurnAffordanceTarget::SpeakRoom { audience_label } => (
            render_prompt_template(
                &system_text.actor_turn_speak_room_option_template,
                &[("audience_label", audience_label)],
            ),
            format!("SPEAK ROOM — address {audience_label} at once"),
            None,
            ActorTurnCommandInvocation::Command {
                command_id: command.id.clone(),
                target_room_id: None,
                target_actor_id: None,
                feature_id: None,
                consumable_id: None,
                context_label: None,
                input_mode: command.input_mode,
            },
        ),
        ActorTurnAffordanceTarget::Hug {
            actor_id,
            actor_name,
        } => (
            render_prompt_template(
                &system_text.actor_turn_hug_option_template,
                &[("prompt_verb", prompt_verb), ("actor_name", actor_name)],
            ),
            format!("{} {actor_id}", command.command),
            None,
            ActorTurnCommandInvocation::Command {
                command_id: command.id.clone(),
                target_room_id: None,
                target_actor_id: Some(actor_id.to_string()),
                feature_id: None,
                consumable_id: None,
                context_label: None,
                input_mode: command.input_mode,
            },
        ),
        ActorTurnAffordanceTarget::Rest { context_label } => (
            render_prompt_template(
                &system_text.actor_turn_rest_option_template,
                &[
                    ("prompt_verb", prompt_verb),
                    ("context_label", context_label),
                ],
            ),
            command.command.clone(),
            None,
            ActorTurnCommandInvocation::Command {
                command_id: command.id.clone(),
                target_room_id: None,
                target_actor_id: None,
                feature_id: None,
                consumable_id: None,
                context_label: Some(context_label.to_string()),
                input_mode: command.input_mode,
            },
        ),
        ActorTurnAffordanceTarget::Consume {
            item_id,
            item_label,
            feature_label,
            kind: _,
        } => (
            render_prompt_template(
                &system_text.actor_turn_consume_option_template,
                &[
                    ("prompt_verb", prompt_verb),
                    ("item_label", item_label),
                    ("feature_label", feature_label),
                ],
            ),
            format!("{} {item_id}", command.command),
            None,
            ActorTurnCommandInvocation::Command {
                command_id: command.id.clone(),
                target_room_id: None,
                target_actor_id: None,
                feature_id: None,
                consumable_id: Some(item_id.to_string()),
                context_label: None,
                input_mode: command.input_mode,
            },
        ),
        ActorTurnAffordanceTarget::InspectFeature {
            feature_id,
            feature_label,
        } => (
            render_prompt_template(
                &system_text.actor_turn_inspect_feature_option_template,
                &[
                    ("prompt_verb", prompt_verb),
                    ("feature_label", feature_label),
                ],
            ),
            format!("{} {feature_id}", command.command),
            None,
            ActorTurnCommandInvocation::Command {
                command_id: command.id.clone(),
                target_room_id: None,
                target_actor_id: None,
                feature_id: Some(feature_id.to_string()),
                consumable_id: None,
                context_label: None,
                input_mode: command.input_mode,
            },
        ),
        ActorTurnAffordanceTarget::InspectActor {
            actor_id,
            actor_name,
        } => (
            render_prompt_template(
                &system_text.actor_turn_inspect_actor_option_template,
                &[("prompt_verb", prompt_verb), ("actor_name", actor_name)],
            ),
            format!("{} {actor_id}", command.command),
            None,
            ActorTurnCommandInvocation::Command {
                command_id: command.id.clone(),
                target_room_id: None,
                target_actor_id: Some(actor_id.to_string()),
                feature_id: None,
                consumable_id: None,
                context_label: None,
                input_mode: command.input_mode,
            },
        ),
        ActorTurnAffordanceTarget::Act => (
            system_text.actor_turn_act_option_template.clone(),
            render_prompt_template(
                &system_text.actor_turn_act_decision_template,
                &[("command", command.command.as_str())],
            ),
            Some(command.command.clone()),
            ActorTurnCommandInvocation::Command {
                command_id: command.id.clone(),
                target_room_id: None,
                target_actor_id: None,
                feature_id: None,
                consumable_id: None,
                context_label: None,
                input_mode: command.input_mode,
            },
        ),
    };
    ActorTurnAffordanceOption {
        affordance_id: affordance_id.to_string(),
        command_id: command.id.clone(),
        group: group.to_string(),
        available_text,
        decision_label,
        decision_prefix,
        invocation,
    }
}

pub(crate) fn dialogue_system_prompt(request: &DialogueRequest) -> &str {
    request.system_text.dialogue_system_prompt.as_str()
}

pub(crate) fn menu_intent_system_prompt(request: &MenuIntentRequest) -> &str {
    request.system_text.menu_intent_system_prompt.as_str()
}

pub(crate) fn actor_turn_decider_system_prompt(_request: &ActorTurnActionRequest) -> &str {
    _request
        .system_text
        .actor_turn_decider_system_prompt
        .as_str()
}

pub(crate) fn conversation_memory_summarizer_system_prompt(
    request: &ConversationMemorySummaryRequest,
) -> &'static str {
    match request.locale.as_str() {
        "zh-TW" => {
            "你要把兩個角色最近互動的重點濃縮成一小段後續可重用的記憶摘要。只能根據提供的既有摘要與最近互動來寫，不要杜撰。只回傳一小段自然語句，不要條列，不要分析，不要超過 240 個字元。"
        }
        _ => {
            "You compress recent interaction history into one short reusable relationship-memory summary for future prompts. Use only the provided summary and recent lines. Return one plain prose sentence or two very short clauses, under 240 characters, with no bullets or analysis."
        }
    }
}

pub(crate) fn chapter_script_summarizer_system_prompt(
    request: &ChapterScriptSummaryRequest,
) -> &'static str {
    match request.locale.as_str() {
        "zh-TW" => {
            "你要閱讀整章逐字稿，寫成一小段章節結尾回顧。只能根據提供的逐字稿，不要杜撰。語氣像戀愛實境節目的集數回顧：具體、有畫面、懂人際張力，但不要浮誇。只回傳一段 3 到 5 句的自然文字。"
        }
        _ => {
            "You turn a full chapter transcript into one compact end-of-episode recap. Use only the provided transcript. Sound like a sharp reality TV recap: specific, vivid, socially observant, and lightly juicy without becoming campy. Return one tight paragraph of 2 to 4 sentences."
        }
    }
}

pub(crate) fn chapter_relationship_summarizer_system_prompt(
    request: &ChapterRelationshipSummaryRequest,
) -> &'static str {
    match request.locale.as_str() {
        "zh-TW" => {
            "你要根據章節結尾的人際數值，寫出關係現況更新。只能依據提供的數值，不要杜撰事件。每行簡短自然，像戀愛實境節目主持人在更新配對看板。回傳 2 到 4 行，不要加條列符號。"
        }
        _ => {
            "You turn end-of-chapter pair stats into relationship-status updates. Use only the provided stat lines. Return 2 to 4 short lines with no bullets, each starting with the pair name exactly as given, like a reality TV host giving a sharp relationship board update. Do not invent scenes or promises."
        }
    }
}

pub(crate) fn direct_speech_intent_system_prompt(request: &DirectSpeechIntentRequest) -> &str {
    request
        .system_text
        .direct_speech_intent_system_prompt
        .as_str()
}

pub(crate) fn sanitize_statement(statement: &str) -> String {
    statement
        .replace('|', "/")
        .replace('\n', " ")
        .trim()
        .to_string()
}

fn format_relationship_context(request: &ActorTurnActionRequest) -> String {
    let mut lines = Vec::new();
    if let Some(actor_name) = request.move_target_actor_name.as_deref() {
        lines.push(format_move_target_context_line(
            actor_name,
            request.move_target_social_note.as_deref(),
        ));
    }
    lines.extend(
        request
            .speak_candidates
            .iter()
            .map(format_talk_relationship_context),
    );
    if lines.is_empty() {
        format!("- {}", request.system_text.actor_turn_no_social_context)
    } else {
        lines
            .into_iter()
            .map(|line| format!("- {line}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn format_move_target_context_line(actor_name: &str, social_note: Option<&str>) -> String {
    match social_note
        .map(sanitize_statement)
        .filter(|note| !note.is_empty())
    {
        Some(note) => format!("{actor_name} — {note} — currently elsewhere"),
        None => format!("{actor_name} — no strong social pull yet — currently elsewhere"),
    }
}

fn format_talk_relationship_context(candidate: &ActorTurnSpeakCandidate) -> String {
    let social_context = candidate
        .interaction_note
        .as_deref()
        .map(sanitize_statement)
        .unwrap_or_else(|| "No strong interaction note yet.".to_string());
    let reply_marker = if candidate.reply_now {
        " Reply now."
    } else {
        ""
    };
    let summary = candidate
        .recent_summary
        .as_deref()
        .map(sanitize_statement)
        .filter(|summary| !summary.is_empty());
    match summary {
        Some(summary) => format!(
            "{name} — {social_context}{reply_marker}\n  Summary: {summary}",
            name = candidate.actor_name,
            social_context = social_context,
            reply_marker = reply_marker,
            summary = summary,
        ),
        None => format!(
            "{name} — {social_context}{reply_marker}",
            name = candidate.actor_name,
            social_context = social_context,
            reply_marker = reply_marker,
        ),
    }
}
