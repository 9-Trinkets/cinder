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
use crate::content::types::{ActionDefinition, CommandInputMode, SystemTextDefinition};

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
    action: &ActionDefinition,
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
            format!("{} {room_id}", action.command),
            None,
            ActorTurnCommandInvocation::Command {
                command_id: action.id.clone(),
                target_room_id: Some(room_id.to_string()),
                target_actor_id: None,
                feature_id: None,
                consumable_id: None,
                context_label: None,
                input_mode: action.input_mode,
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
                command_id: action.id.clone(),
                target_room_id: None,
                target_actor_id: Some(actor_id.to_string()),
                feature_id: None,
                consumable_id: None,
                context_label: None,
                input_mode: action.input_mode,
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
                command_id: action.id.clone(),
                target_room_id: None,
                target_actor_id: None,
                feature_id: None,
                consumable_id: None,
                context_label: None,
                input_mode: action.input_mode,
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
            format!("{} {actor_id}", action.command),
            None,
            ActorTurnCommandInvocation::Command {
                command_id: action.id.clone(),
                target_room_id: None,
                target_actor_id: Some(actor_id.to_string()),
                feature_id: None,
                consumable_id: None,
                context_label: None,
                input_mode: action.input_mode,
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
            action.command.clone(),
            None,
            ActorTurnCommandInvocation::Command {
                command_id: action.id.clone(),
                target_room_id: None,
                target_actor_id: None,
                feature_id: None,
                consumable_id: None,
                context_label: Some(context_label.to_string()),
                input_mode: action.input_mode,
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
            format!("{} {item_id}", action.command),
            None,
            ActorTurnCommandInvocation::Command {
                command_id: action.id.clone(),
                target_room_id: None,
                target_actor_id: None,
                feature_id: None,
                consumable_id: Some(item_id.to_string()),
                context_label: None,
                input_mode: action.input_mode,
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
            format!("{} {feature_id}", action.command),
            None,
            ActorTurnCommandInvocation::Command {
                command_id: action.id.clone(),
                target_room_id: None,
                target_actor_id: None,
                feature_id: Some(feature_id.to_string()),
                consumable_id: None,
                context_label: None,
                input_mode: action.input_mode,
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
            format!("{} {actor_id}", action.command),
            None,
            ActorTurnCommandInvocation::Command {
                command_id: action.id.clone(),
                target_room_id: None,
                target_actor_id: Some(actor_id.to_string()),
                feature_id: None,
                consumable_id: None,
                context_label: None,
                input_mode: action.input_mode,
            },
        ),
        ActorTurnAffordanceTarget::Act => match action.input_mode {
            CommandInputMode::FreeformText => (
                system_text.actor_turn_act_option_template.clone(),
                render_prompt_template(
                    &system_text.actor_turn_act_decision_template,
                    &[("command", action.command.as_str())],
                ),
                Some(action.command.clone()),
                ActorTurnCommandInvocation::Command {
                    command_id: action.id.clone(),
                    target_room_id: None,
                    target_actor_id: None,
                    feature_id: None,
                    consumable_id: None,
                    context_label: None,
                    input_mode: action.input_mode,
                },
            ),
            CommandInputMode::None => (
                format!("You could {prompt_verb}."),
                action.command.clone(),
                None,
                ActorTurnCommandInvocation::Command {
                    command_id: action.id.clone(),
                    target_room_id: None,
                    target_actor_id: None,
                    feature_id: None,
                    consumable_id: None,
                    context_label: None,
                    input_mode: action.input_mode,
                },
            ),
        },
    };
    ActorTurnAffordanceOption {
        affordance_id: affordance_id.to_string(),
        command_id: action.id.clone(),
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
) -> &str {
    request
        .system_text
        .conversation_memory_summarizer_system_prompt
        .as_str()
}

pub(crate) fn chapter_script_summarizer_system_prompt(
    request: &ChapterScriptSummaryRequest,
) -> &str {
    request
        .system_text
        .chapter_script_summarizer_system_prompt
        .as_str()
}

pub(crate) fn chapter_relationship_summarizer_system_prompt(
    request: &ChapterRelationshipSummaryRequest,
) -> &str {
    request
        .system_text
        .chapter_relationship_summarizer_system_prompt
        .as_str()
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

#[cfg(test)]
mod tests {
    use super::build_actor_turn_affordance_option;
    use crate::content::types::{ActionDefinition, CommandInputMode, CommandTargetMode};
    use crate::engine::dialogue::ActorTurnAffordanceTarget;
    use crate::engine::test_fixtures::minimal_test_pack;

    #[test]
    fn targetless_none_input_commands_render_explicit_action_text() {
        let content = minimal_test_pack();
        let option = build_actor_turn_affordance_option(
            &content.system_text,
            "cook",
            "kitchen",
            "cook now",
            None,
            &ActionDefinition {
                id: "cook".to_string(),
                command: "COOK".to_string(),
                input_mode: CommandInputMode::None,
                target_mode: CommandTargetMode::None,
                ..ActionDefinition::default()
            },
            ActorTurnAffordanceTarget::Act,
        );

        assert_eq!(option.available_text, "You could cook now.");
        assert_eq!(option.decision_label, "COOK");
        assert_eq!(option.decision_prefix, None);
    }
}
