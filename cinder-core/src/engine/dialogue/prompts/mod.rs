use super::{
    ActorTurnActionRequest, ActorTurnAffordanceOption, ActorTurnAffordanceTarget,
    ActorTurnCommandInvocation, ActorTurnSpeakCandidate, ChapterRelationshipSummaryRequest,
    ChapterScriptSummaryRequest, ConversationMemorySummaryRequest, DialogueRequest,
    DirectSpeechIntentRequest, MenuIntentRequest, StageAssignmentRequest,
};
use crate::content::types::SpeechIntentLabel;
use crate::engine::state::{ConversationMemoryKind, ConversationMemoryLine};

pub(crate) fn build_scene_brief_dialogue_prompt(request: &DialogueRequest) -> String {
    let text = &request.system_text;
    let mut prompt_sections = vec![
        format!(
            "{character_label}\n{character}",
            character_label = text.dialogue_section_character,
            character = format_bullets(&request.character_notes, &text.dialogue_no_character_facts),
        ),
        format!(
            "{setting_label}\n{setting}",
            setting_label = text.dialogue_section_setting,
            setting = format_bullets(&request.setting_notes, &text.dialogue_no_setting_facts),
        ),
        format!(
            "{current_beat_label}\n{current_beat}",
            current_beat_label = text.dialogue_section_current_beat,
            current_beat = format_bullets(
                &request.current_beat_notes,
                &text.dialogue_no_current_beat_facts,
            ),
        ),
        format!(
            "{subtext_label}\n{subtext}",
            subtext_label = text.dialogue_section_subtext,
            subtext = format_bullets(&request.subtext_notes, &text.dialogue_no_subtext_facts),
        ),
        format!(
            "{behavior_examples_label}\n{behavior_examples}",
            behavior_examples_label = text.dialogue_section_behavior_examples,
            behavior_examples = format_bullets(
                &request.behavior_examples,
                &text.dialogue_no_behavior_examples,
            ),
        ),
    ];

    if request.include_conversation_summary_section {
        prompt_sections.push(format!(
            "{memory_summary_label}\n{memory_summary}",
            memory_summary_label = text.conversation_memory_summary_label,
            memory_summary = format_optional_summary(
                request.recent_memory_summary.as_deref(),
                &text.conversation_memory_summary_empty,
            ),
        ));
    }

    prompt_sections.push(format!(
        "{recent_memory_label}\n{recent_memory}",
        recent_memory_label = text.dialogue_section_recent_memory,
        recent_memory =
            format_dialogue_memory(request, &request.system_text.dialogue_no_recent_memory),
    ));

    if request.include_latest_line_section {
        let other_person_message = request
            .other_person_message
            .clone()
            .unwrap_or_else(|| text.dialogue_no_direct_question.clone());
        prompt_sections.push(format!(
            "{latest_line_label}\n{other_person_message}",
            latest_line_label = render_prompt_template(
                &text.dialogue_latest_line_label,
                &[("other_person_name", request.other_person_name.as_str())],
            ),
            other_person_message = sanitize_statement(&other_person_message),
        ));
    }

    prompt_sections.push(format!(
        "{response_label}\n{response}",
        response_label = text.dialogue_section_response,
        response = format_bullets(&request.response_notes, &text.dialogue_response_fallback),
    ));

    format!("{}\n", prompt_sections.join("\n\n"))
}

pub(crate) fn build_menu_intent_prompt(request: &MenuIntentRequest) -> String {
    let text = &request.system_text;
    let other_person_message = request
        .other_person_message
        .clone()
        .unwrap_or_else(|| text.menu_no_direct_request.clone());
    format!(
        "{menu_title}\n- {menu_id_label}: {menu_id}\n- {offered_by_label}: {actor_name}\n- {intent_guidance_label}: {intent_guidance}\n- {options_label}: {options}\n\n{setting_label}\n{setting}\n\n{current_beat_label}\n{current_beat}\n\n{recent_memory_label}\n{recent_memory}\n\n{latest_line_label}\n\"\"\"\n{other_person_name}: {other_person_message}\n\"\"\"\n\n{decision_label}\n{decision_instruction}",
        menu_title = text.menu_section_title,
        menu_id_label = text.menu_id_label,
        menu_id = request.menu_id,
        offered_by_label = text.menu_offered_by_label,
        actor_name = request.actor_name,
        intent_guidance_label = text.menu_intent_guidance_label,
        intent_guidance = sanitize_statement(&request.intent_guidance),
        options_label = text.menu_available_options_label,
        options = if request.option_titles.is_empty() {
            text.menu_no_authored_options.clone()
        } else if request.locale == "zh-TW" {
            request.option_titles.join("、")
        } else {
            request.option_titles.join(", ")
        },
        setting_label = text.menu_section_setting,
        setting = format_bullets(&request.setting_notes, &text.dialogue_no_setting_facts),
        current_beat_label = text.menu_section_current_beat,
        current_beat = format_bullets(
            &request.current_beat_notes,
            &text.dialogue_no_current_beat_facts,
        ),
        recent_memory_label = text.menu_section_recent_memory,
        recent_memory = format_memory(
            &request.recent_memory,
            &request.system_text.dialogue_no_recent_memory,
            Some(request.actor_name.as_str()),
        ),
        latest_line_label = render_prompt_template(
            &text.menu_latest_line_label,
            &[("other_person_name", request.other_person_name.as_str())],
        ),
        other_person_name = request.other_person_name,
        other_person_message = sanitize_statement(&other_person_message),
        decision_label = text.menu_decision_label,
        decision_instruction = text.menu_decision_instruction,
    )
}

pub(crate) fn build_conversation_memory_summary_prompt(
    request: &ConversationMemorySummaryRequest,
) -> String {
    let text = &request.system_text;
    let template = &text.conversation_memory_summary_prompt_template;
    render_prompt_template(
        template,
        &[
            ("participant_a_name", request.participant_a_name.as_str()),
            ("participant_b_name", request.participant_b_name.as_str()),
            (
                "existing_summary",
                request
                    .existing_summary
                    .as_deref()
                    .filter(|summary| !summary.is_empty())
                    .unwrap_or("(none)"),
            ),
            (
                "recent_lines",
                &format_memory(
                    &request.recent_lines,
                    &text.conversation_memory_summary_empty,
                    None,
                ),
            ),
        ],
    )
}

pub(crate) fn build_chapter_script_summary_prompt(request: &ChapterScriptSummaryRequest) -> String {
    let text = &request.system_text;
    let template = &text.chapter_script_summary_prompt_template;
    render_prompt_template(
        template,
        &[(
            "transcript",
            &format_chapter_lines(
                &request.transcript_lines,
                &text.chapter_script_summary_empty,
            ),
        )],
    )
}

pub(crate) fn build_chapter_relationship_summary_prompt(
    request: &ChapterRelationshipSummaryRequest,
) -> String {
    let text = &request.system_text;
    let template = &text.chapter_relationship_summary_prompt_template;
    render_prompt_template(
        template,
        &[(
            "pair_stats",
            &format_chapter_lines(
                &request.pair_stat_lines,
                &text.chapter_relationship_summary_empty,
            ),
        )],
    )
}

pub(crate) fn build_direct_speech_intent_prompt(
    request: &DirectSpeechIntentRequest,
    intents: &[SpeechIntentLabel],
) -> String {
    let text = &request.system_text;
    let other_person_message = request
        .other_person_message
        .as_deref()
        .filter(|message| !message.trim().is_empty())
        .map(|message| format!("- {}", sanitize_statement(message)))
        .unwrap_or_else(|| format!("- {}", text.direct_speech_intent_no_reply));
    let available_intents = if intents.is_empty() {
        String::new()
    } else {
        intents
            .iter()
            .map(|label| format!("  - {}: {}", label.label, label.description))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let template = &text.direct_speech_intent_prompt_template;
    render_prompt_template(
        template,
        &[
            ("actor_name", request.actor_name.as_str()),
            ("other_person_name", request.other_person_name.as_str()),
            (
                "current_beat",
                &format_bullets(
                    &request.current_beat_notes,
                    &text.direct_speech_intent_no_current_beat,
                ),
            ),
            (
                "subtext",
                &format_bullets(
                    &request.subtext_notes,
                    &text.direct_speech_intent_no_subtext,
                ),
            ),
            (
                "recent_memory",
                &format_memory(
                    &request.recent_memory,
                    &text.direct_speech_intent_no_recent_memory,
                    Some(&request.actor_name),
                ),
            ),
            ("other_person_message", &other_person_message),
            ("spoken_line", &sanitize_statement(&request.spoken_line)),
            ("available_intents", &available_intents),
        ],
    )
}

pub(crate) fn build_stage_assignment_prompt(request: &StageAssignmentRequest) -> String {
    let candidate_lines = request
        .candidates
        .iter()
        .map(|candidate| {
            let actor_stats = format_stat_map(&candidate.actor_stats);
            let pair_stats = format_stat_map(&candidate.pair_stats_with_initiator);
            format!(
                "- actor_id: {actor_id}\n  name: {actor_name}\n  current_room: {current_room_title}\n  actor_stats: {actor_stats}\n  pair_stats_with_{initiator_actor_id}: {pair_stats}",
                actor_id = candidate.actor_id,
                actor_name = candidate.actor_name,
                current_room_title = candidate.current_room_title,
                actor_stats = actor_stats,
                initiator_actor_id = request.initiator_actor_id,
                pair_stats = pair_stats,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "You are deciding how to split a stage's characters between two possible room outcomes.\n\nStage: {stage_id}\nSelection: {selection_label}\nInitiator: {initiator_actor_name}\nSelected room: {selected_room_title}\nRemaining room: {remaining_room_title}\n\nCurrent beat notes:\n{beat_note}\n\nContent-specific instructions:\n{prompt_instructions}\n\nFor each candidate below, assign an integer selection_score from 0 to 100 for how likely they are to join {initiator_actor_name} in {selected_room_title} for this stage split right now.\nUse only the candidate's stats, their pair stats with the initiator, and the content instructions above. Higher score means more likely to join the selected room.\n\nReturn JSON in exactly this shape:\n{{\"assignments\":[{{\"actor_id\":\"...\",\"selection_score\":72,\"rationale\":\"short reason\"}}]}}\n\nCandidates:\n{candidate_lines}\n",
        stage_id = request.stage_id,
        selection_label = request.selection_label,
        initiator_actor_name = request.initiator_actor_name,
        selected_room_title = request.selected_room_title,
        remaining_room_title = request.remaining_room_title,
        beat_note = if request.beat_note.trim().is_empty() {
            "(none)"
        } else {
            request.beat_note.trim()
        },
        prompt_instructions = if request.prompt_instructions.trim().is_empty() {
            "(none)"
        } else {
            request.prompt_instructions.trim()
        },
        candidate_lines = candidate_lines,
    )
}

mod actor_turn;
pub(crate) use actor_turn::{
    actor_turn_decider_system_prompt, chapter_relationship_summarizer_system_prompt,
    chapter_script_summarizer_system_prompt, conversation_memory_summarizer_system_prompt,
    dialogue_system_prompt, direct_speech_intent_system_prompt, menu_intent_system_prompt,
    sanitize_statement,
};
pub(crate) use actor_turn::{build_actor_turn_action_prompt, build_actor_turn_affordance_option};

pub(super) fn join_non_empty_sections(sections: &[String]) -> String {
    sections
        .iter()
        .filter(|section| !section.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn format_bullets(lines: &[String], empty_message: &str) -> String {
    if lines.is_empty() {
        return format!("- {empty_message}");
    }
    lines
        .iter()
        .map(|line| format!("- {}", sanitize_statement(line)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_stat_map(stats: &std::collections::BTreeMap<String, i32>) -> String {
    if stats.is_empty() {
        return "(none)".to_string();
    }
    stats
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_chapter_lines(lines: &[String], empty_message: &str) -> String {
    if lines.is_empty() {
        return format!("- {empty_message}");
    }
    lines
        .iter()
        .map(|line| format!("- {}", sanitize_statement(line)))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn format_memory(
    memory: &[ConversationMemoryLine],
    empty_message: &str,
    viewer_name: Option<&str>,
) -> String {
    if memory.is_empty() {
        return format!("- {empty_message}");
    }

    memory
        .iter()
        .map(|line| match line.kind {
            ConversationMemoryKind::Speech => {
                let marker = match line.target_label.as_deref() {
                    Some("room") => "[to everyone] ".to_string(),
                    Some(target) if viewer_name.is_some_and(|viewer| viewer == target) => {
                        "[to you] ".to_string()
                    }
                    Some(target) => format!("[to {}] ", sanitize_statement(target)),
                    None => String::new(),
                };
                format!(
                    "{marker}{}: {}",
                    line.speaker_name,
                    sanitize_statement(&line.text)
                )
            }
            ConversationMemoryKind::Action => {
                let marker = match line.target_label.as_deref() {
                    Some("room") | None => "[action] ".to_string(),
                    Some(target) => format!("[action to {}] ", sanitize_statement(target)),
                };
                format!("{marker}{}", sanitize_statement(&line.text))
            }
        })
        .map(|line| format!("- {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_optional_summary(summary: Option<&str>, empty_message: &str) -> String {
    summary
        .filter(|summary| !summary.is_empty())
        .map(|summary| format!("- {}", sanitize_statement(summary)))
        .unwrap_or_else(|| format!("- {}", empty_message))
}

fn format_dialogue_memory(request: &DialogueRequest, empty_message: &str) -> String {
    if request.recent_memory.is_empty() {
        return format!("- {empty_message}");
    }

    request
        .recent_memory
        .iter()
        .map(|line| match line.kind {
            ConversationMemoryKind::Speech => {
                let speaker = if line.speaker_id == request.actor_id {
                    "You".to_string()
                } else if line.speaker_id == request.other_person_id {
                    request.other_person_name.clone()
                } else {
                    line.speaker_name.clone()
                };
                format!("{speaker} said {}", sanitize_statement(&line.text))
            }
            ConversationMemoryKind::Action => sanitize_statement(&line.text),
        })
        .map(|line| format!("- {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn render_prompt_template(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut rendered = template.to_string();
    for (key, value) in replacements {
        rendered = rendered.replace(&format!("{{{key}}}"), value);
    }
    rendered
}
