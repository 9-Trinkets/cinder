use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemTextDefinition {
    pub dialogue_system_prompt: String,
    pub dialogue_section_character: String,
    pub dialogue_section_setting: String,
    pub dialogue_section_current_beat: String,
    pub dialogue_section_subtext: String,
    #[serde(default = "default_dialogue_section_behavior_examples")]
    pub dialogue_section_behavior_examples: String,
    pub dialogue_section_recent_memory: String,
    pub dialogue_latest_line_label: String,
    pub dialogue_section_response: String,
    pub dialogue_no_direct_question: String,
    pub dialogue_no_character_facts: String,
    pub dialogue_no_setting_facts: String,
    pub dialogue_no_current_beat_facts: String,
    pub dialogue_no_subtext_facts: String,
    #[serde(default = "default_dialogue_no_behavior_examples")]
    pub dialogue_no_behavior_examples: String,
    pub dialogue_no_recent_memory: String,
    pub dialogue_response_fallback: String,
    pub menu_intent_system_prompt: String,
    pub menu_section_title: String,
    pub menu_id_label: String,
    pub menu_offered_by_label: String,
    pub menu_intent_guidance_label: String,
    pub menu_available_options_label: String,
    pub menu_section_setting: String,
    pub menu_section_current_beat: String,
    pub menu_section_recent_memory: String,
    pub menu_latest_line_label: String,
    pub menu_decision_label: String,
    pub menu_no_direct_request: String,
    pub menu_no_authored_options: String,
    pub menu_decision_instruction: String,
    pub prompt_time_note: String,
    pub prompt_current_room_note: String,
    pub prompt_visible_features_note: String,
    pub prompt_people_here_note: String,
    pub prompt_exits_note: String,
    pub prompt_current_speaker_note: String,
    pub prompt_shared_room_note: String,
    pub prompt_latest_words_note: String,
    pub prompt_address_other_person_note: String,
    #[serde(default = "default_actor_action_response_notes")]
    pub actor_action_response_notes: Vec<String>,
    #[serde(default = "default_conversation_memory_summary_label")]
    pub conversation_memory_summary_label: String,
    #[serde(default = "default_conversation_memory_summary_empty")]
    pub conversation_memory_summary_empty: String,
    #[serde(default = "default_conversation_memory_summary_prompt_template")]
    pub conversation_memory_summary_prompt_template: String,
    #[serde(default = "default_chapter_script_summary_empty")]
    pub chapter_script_summary_empty: String,
    #[serde(default = "default_chapter_script_summary_prompt_template")]
    pub chapter_script_summary_prompt_template: String,
    #[serde(default = "default_chapter_relationship_summary_empty")]
    pub chapter_relationship_summary_empty: String,
    #[serde(default = "default_chapter_relationship_summary_prompt_template")]
    pub chapter_relationship_summary_prompt_template: String,
    #[serde(default = "default_direct_speech_intent_no_current_beat")]
    pub direct_speech_intent_no_current_beat: String,
    #[serde(default = "default_direct_speech_intent_no_subtext")]
    pub direct_speech_intent_no_subtext: String,
    #[serde(default = "default_direct_speech_intent_no_recent_memory")]
    pub direct_speech_intent_no_recent_memory: String,
    #[serde(default = "default_direct_speech_intent_no_reply")]
    pub direct_speech_intent_no_reply: String,
    #[serde(default = "default_direct_speech_intent_prompt_template")]
    pub direct_speech_intent_prompt_template: String,
    #[serde(default = "default_direct_speech_intent_system_prompt")]
    pub direct_speech_intent_system_prompt: String,
    #[serde(default = "default_actor_turn_prompt_recent_memory_note")]
    pub actor_turn_prompt_recent_memory_note: String,
    #[serde(default = "default_actor_turn_relationship_status_label")]
    pub actor_turn_relationship_status_label: String,
    #[serde(default = "default_actor_turn_available_actions_label")]
    pub actor_turn_available_actions_label: String,
    #[serde(default = "default_actor_turn_decision_instruction")]
    pub actor_turn_decision_instruction: String,
    #[serde(default = "default_actor_turn_prompt_template")]
    pub actor_turn_prompt_template: String,
    #[serde(default = "default_actor_turn_decider_system_prompt")]
    pub actor_turn_decider_system_prompt: String,
    #[serde(default = "default_actor_turn_no_social_context")]
    pub actor_turn_no_social_context: String,
    #[serde(default = "default_actor_turn_move_option_template")]
    pub actor_turn_move_option_template: String,
    #[serde(default = "default_actor_turn_move_option_with_actor_template")]
    pub actor_turn_move_option_with_actor_template: String,
    #[serde(default = "default_actor_turn_speak_option_template")]
    pub actor_turn_speak_option_template: String,
    #[serde(default = "default_actor_turn_reply_option_template")]
    pub actor_turn_reply_option_template: String,
    #[serde(default = "default_actor_turn_speak_room_option_template")]
    pub actor_turn_speak_room_option_template: String,
    #[serde(default = "default_actor_turn_hug_option_template")]
    pub actor_turn_hug_option_template: String,
    #[serde(default = "default_actor_turn_rest_option_template")]
    pub actor_turn_rest_option_template: String,
    #[serde(default = "default_actor_turn_consume_option_template")]
    pub actor_turn_consume_option_template: String,
    #[serde(default = "default_actor_turn_inspect_feature_option_template")]
    pub actor_turn_inspect_feature_option_template: String,
    #[serde(default = "default_actor_turn_inspect_actor_option_template")]
    pub actor_turn_inspect_actor_option_template: String,
    #[serde(default = "default_actor_turn_act_option_template")]
    pub actor_turn_act_option_template: String,
    #[serde(default = "default_actor_turn_act_decision_template")]
    pub actor_turn_act_decision_template: String,
    #[serde(default = "default_exploration_unvisited_room_note_template")]
    pub exploration_unvisited_room_note_template: String,
}

// ── UiTextDefinition default helpers ─────────────────────────────────────────

fn default_dialogue_section_behavior_examples() -> String {
    "Behavior Examples".to_string()
}

fn default_dialogue_no_behavior_examples() -> String {
    "No behavior examples.".to_string()
}

fn default_actor_action_response_notes() -> Vec<String> {
    vec![
        "Describe one brief, noticeable in-room action in bare third-person present tense as this actor."
            .to_string(),
        "Start with a present-tense verb phrase (for example: adjusts, glances, rests)."
            .to_string(),
        "Do not include dialogue, quotes, or any person's name.".to_string(),
    ]
}

fn default_conversation_memory_summary_label() -> String {
    "Conversation Summary".to_string()
}

fn default_conversation_memory_summary_empty() -> String {
    "No longer-term interaction summary yet.".to_string()
}

fn default_conversation_memory_summary_prompt_template() -> String {
    "Participants\n- {participant_a_name}\n- {participant_b_name}\n\nExisting Summary\n{existing_summary}\n\nRecent Lines\n{recent_lines}\n\nTask\nWrite one short relationship-memory summary for future prompt grounding.\n- Keep it under 240 characters.\n- Focus on what changed between them: trust, tension, warmth, recurring topics, promises, distance, or revealing visible actions.\n- Do not invent anything.\n- Write in plain third-person prose, not bullets.\n- Use the same language as the source lines.".to_string()
}

fn default_chapter_script_summary_empty() -> String {
    "No chapter transcript available.".to_string()
}

fn default_chapter_script_summary_prompt_template() -> String {
    "Chapter Transcript\n{transcript}\n\nTask\nWrite one short end-of-chapter recap.\n- Use only the transcript.\n- Sound like a sharp reality TV episode recap: specific, vivid, socially observant, and a little juicy without turning campy.\n- Keep it to one tight paragraph of 2 to 4 sentences.\n- Lead with the most important turn of the night, then name the strongest connection, the sharpest awkward beat, or the mood that settled over the house.\n- End on what tension or possibility is still hanging in the air.\n- Do not invent off-screen events, inner thoughts, or future outcomes.\n- Use the same language as the transcript.".to_string()
}

fn default_chapter_relationship_summary_empty() -> String {
    "No relationship shifts were recorded.".to_string()
}

fn default_chapter_relationship_summary_prompt_template() -> String {
    "Final Relationship Stats\n{pair_stats}\n\nTask\nWrite the relationship-status segment for the end of the chapter.\n- Base it only on these stat lines.\n- Return 2 to 4 short lines, with one pair per line when possible.\n- Start each line with the pair name exactly as given, then an em dash, then the status update.\n- Keep the tone sharp and readable, like a reality TV host giving the latest status board.\n- Mention uncertainty when the numbers are mild instead of overselling them.\n- Do not invent scenes or promises that are not supported by the stats.\n- Use the same language as the stat labels.".to_string()
}

fn default_direct_speech_intent_no_current_beat() -> String {
    "No current beat guidance.".to_string()
}

fn default_direct_speech_intent_no_subtext() -> String {
    "No subtext notes.".to_string()
}

fn default_direct_speech_intent_no_recent_memory() -> String {
    "No recent memory.".to_string()
}

fn default_direct_speech_intent_no_reply() -> String {
    "No immediate reply from the other person.".to_string()
}

fn default_direct_speech_intent_prompt_template() -> String {
    "Speaker\n- {actor_name}\n\nTarget\n- {other_person_name}\n\nCurrent Beat\n{current_beat}\n\nSubtext\n{subtext}\n\nRecent Memory\n{recent_memory}\n\nLatest Line From Target\n{other_person_message}\n\nSpoken Line\n- {spoken_line}\n\nAvailable Intents\n{available_intents}\n\nTask\nChoose the intent that best describes the spoken line.\n- Base the choice on the spoken line first, using the other context only to interpret it.\n- Return exactly one label from the Available Intents list.".to_string()
}

fn default_direct_speech_intent_system_prompt() -> String {
    "You classify the relational intent of a spoken line. Return exactly one label from the available intents list. Use the descriptions to guide your choice.".to_string()
}

fn default_actor_turn_prompt_recent_memory_note() -> String {
    "(Chronological room context, oldest to newest.)".to_string()
}

fn default_actor_turn_relationship_status_label() -> String {
    "Relationship Status".to_string()
}

fn default_actor_turn_available_actions_label() -> String {
    "Available Actions".to_string()
}

fn default_actor_turn_decision_instruction() -> String {
    "Return exactly one command from this list.\nDo not return explanations or descriptions."
        .to_string()
}

fn default_actor_turn_prompt_template() -> String {
    "Character\n{character}\n\nSetting\n{setting}\n\nCurrent Beat\n{current_beat}\n\nSubtext\n{subtext}\n\nBehavior Examples\n{behavior_examples}\n\nRecent Memory\n{recent_memory_note}\n{recent_memory}\n\n{relationship_status_label}\n{relationship_context}\n\n{available_actions_label}\n{available_actions}\n\n{decision_label}\n{decision_instruction}\n{decision_lines}".to_string()
}

fn default_actor_turn_decider_system_prompt() -> String {
    "Use Available Actions to understand what is possible. Return exactly one command from the prompt's Decision section and do not include explanations or descriptions. If a listed command includes a placeholder like <...>, repeat the command prefix followed by a short bare third-person present-tense action phrase. Do not include the actor name or any leading pronoun. Never use first-person or second-person wording.".to_string()
}

fn default_actor_turn_no_social_context() -> String {
    "No active social context yet.".to_string()
}

fn default_actor_turn_move_option_template() -> String {
    "You could {prompt_verb} to {room_title}.".to_string()
}

fn default_actor_turn_move_option_with_actor_template() -> String {
    "You could {prompt_verb} to {room_title} next to get closer to {actor_name}.".to_string()
}

fn default_actor_turn_speak_option_template() -> String {
    "You could speak to {actor_name}.".to_string()
}

fn default_actor_turn_reply_option_template() -> String {
    "You could speak to {actor_name} now.".to_string()
}

fn default_actor_turn_speak_room_option_template() -> String {
    "You could speak to {audience_label} by addressing the whole room at once.".to_string()
}

fn default_actor_turn_hug_option_template() -> String {
    "You could {prompt_verb} {actor_name}.".to_string()
}

fn default_actor_turn_rest_option_template() -> String {
    "You could {prompt_verb} on the {context_label} to recover a little.".to_string()
}

fn default_actor_turn_consume_option_template() -> String {
    "You could {prompt_verb} the {item_label} from the {feature_label}.".to_string()
}

fn default_actor_turn_inspect_feature_option_template() -> String {
    "You could {prompt_verb} {feature_label} more closely.".to_string()
}

fn default_actor_turn_inspect_actor_option_template() -> String {
    "You could {prompt_verb} {actor_name} more closely.".to_string()
}

fn default_actor_turn_act_option_template() -> String {
    "You could do one brief, noticeable in-room action without speaking.".to_string()
}

fn default_actor_turn_act_decision_template() -> String {
    "{command} <third-person present-tense action>".to_string()
}

fn default_exploration_unvisited_room_note_template() -> String {
    "You have not really gotten a feel for the {room_title} yet.".to_string()
}
