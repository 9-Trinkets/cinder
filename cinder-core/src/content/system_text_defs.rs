use serde::{Deserialize, Serialize};

/// System-level prompt and label text for the dialogue engine.
///
/// Fields without a default are required in every pack's `system.json`. The
/// defaulted fields fall back to values in `system_defaults.json` (bundled
/// with the engine); the loader merges a pack's `system.json` over that base,
/// so packs only declare the keys they want to override.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemTextDefinition {
    pub dialogue_system_prompt: String,
    pub dialogue_section_character: String,
    pub dialogue_section_setting: String,
    pub dialogue_section_current_beat: String,
    pub dialogue_section_subtext: String,
    #[serde(default)]
    pub dialogue_section_behavior_examples: String,
    pub dialogue_section_recent_memory: String,
    pub dialogue_latest_line_label: String,
    pub dialogue_section_response: String,
    pub dialogue_no_direct_question: String,
    pub dialogue_no_character_facts: String,
    pub dialogue_no_setting_facts: String,
    pub dialogue_no_current_beat_facts: String,
    pub dialogue_no_subtext_facts: String,
    #[serde(default)]
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
    #[serde(default)]
    pub actor_action_response_notes: Vec<String>,
    #[serde(default)]
    pub conversation_memory_summary_label: String,
    #[serde(default)]
    pub conversation_memory_summary_empty: String,
    #[serde(default)]
    pub conversation_memory_summary_prompt_template: String,
    #[serde(default)]
    pub chapter_script_summary_empty: String,
    #[serde(default)]
    pub chapter_script_summary_prompt_template: String,
    #[serde(default)]
    pub chapter_relationship_summary_empty: String,
    #[serde(default)]
    pub chapter_relationship_summary_prompt_template: String,
    #[serde(default)]
    pub direct_speech_intent_no_current_beat: String,
    #[serde(default)]
    pub direct_speech_intent_no_subtext: String,
    #[serde(default)]
    pub direct_speech_intent_no_recent_memory: String,
    #[serde(default)]
    pub direct_speech_intent_no_reply: String,
    #[serde(default)]
    pub direct_speech_intent_prompt_template: String,
    #[serde(default)]
    pub direct_speech_intent_system_prompt: String,
    #[serde(default)]
    pub actor_turn_prompt_recent_memory_note: String,
    #[serde(default)]
    pub actor_turn_relationship_status_label: String,
    #[serde(default)]
    pub actor_turn_available_actions_label: String,
    #[serde(default)]
    pub actor_turn_decision_instruction: String,
    #[serde(default)]
    pub actor_turn_prompt_template: String,
    #[serde(default)]
    pub actor_turn_decider_system_prompt: String,
    #[serde(default)]
    pub actor_turn_no_social_context: String,
    #[serde(default)]
    pub actor_turn_move_option_template: String,
    #[serde(default)]
    pub actor_turn_move_option_with_actor_template: String,
    #[serde(default)]
    pub actor_turn_speak_option_template: String,
    #[serde(default)]
    pub actor_turn_reply_option_template: String,
    #[serde(default)]
    pub actor_turn_speak_room_option_template: String,
    #[serde(default)]
    pub actor_turn_hug_option_template: String,
    #[serde(default)]
    pub actor_turn_rest_option_template: String,
    #[serde(default)]
    pub actor_turn_consume_option_template: String,
    #[serde(default)]
    pub actor_turn_inspect_feature_option_template: String,
    #[serde(default)]
    pub actor_turn_inspect_actor_option_template: String,
    #[serde(default)]
    pub actor_turn_act_option_template: String,
    #[serde(default)]
    pub actor_turn_act_decision_template: String,
    #[serde(default)]
    pub exploration_unvisited_room_note_template: String,
    #[serde(default)]
    pub conversation_memory_summarizer_system_prompt: String,
    #[serde(default)]
    pub chapter_script_summarizer_system_prompt: String,
    #[serde(default)]
    pub chapter_relationship_summarizer_system_prompt: String,
    #[serde(default)]
    pub act_cast_character_note_template: String,
    #[serde(default)]
    pub act_cast_subtext_template: String,
    #[serde(default)]
    pub act_cast_response_note_template: String,
    #[serde(default)]
    pub stage_assignment_system_prompt: String,
    #[serde(default)]
    pub dynamic_menu_system_prompt: String,
    #[serde(default)]
    pub hostility_planner_system_prompt: String,
}
