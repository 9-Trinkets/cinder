use serde::{Deserialize, Serialize};

use super::closure::{ActClosureDefinition, ShellMenuDefinition};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiTextDefinition {
    #[serde(default = "default_language_name")]
    pub language_name: String,
    #[serde(default = "default_menu_button_label")]
    pub menu_button_label: String,
    #[serde(default = "default_act_ended_title")]
    pub act_ended_title: String,
    #[serde(default = "default_game_over_hint")]
    pub game_over_hint: String,
    #[serde(default = "default_menu_option_list_title")]
    pub menu_option_list_title: String,
    #[serde(default = "default_menu_choice_hint")]
    pub menu_choice_hint: String,
    #[serde(default = "default_menu_choice_transcript")]
    pub menu_choice_transcript: String,
    #[serde(default = "default_shell_menu_title")]
    pub shell_menu_title: String,
    #[serde(default = "default_shell_menu_close_hint")]
    pub shell_menu_close_hint: String,
    #[serde(default = "default_help_label")]
    pub help_label: String,
    #[serde(default = "default_resume_label")]
    pub resume_label: String,
    #[serde(default = "default_things_to_do_label")]
    pub things_to_do_label: String,
    #[serde(default = "default_about_label")]
    pub about_label: String,
    #[serde(default = "default_exit_label")]
    pub exit_label: String,
    #[serde(default = "default_language_menu_label")]
    pub language_menu_label: String,
    #[serde(default = "default_room_switcher_label")]
    pub room_switcher_label: String,
    #[serde(default = "default_room_switcher_title")]
    pub room_switcher_title: String,
    #[serde(default = "default_room_switch_transcript")]
    pub room_switch_transcript: String,
    #[serde(default = "default_follow_actor_title")]
    pub follow_actor_title: String,
    #[serde(default = "default_follow_actor_transcript")]
    pub follow_actor_transcript: String,
    #[serde(default = "default_follow_actor_stop_transcript")]
    pub follow_actor_stop_transcript: String,
    #[serde(default = "default_things_to_do_empty")]
    pub things_to_do_empty: String,
    #[serde(default = "default_about_body")]
    pub about_body: String,
    #[serde(default = "default_language_modal_title")]
    pub language_modal_title: String,
    #[serde(default = "default_language_changed_text")]
    pub language_changed_text: String,
    #[serde(default = "default_modal_close_hint")]
    pub modal_close_hint: String,
    #[serde(default = "default_act_closure_title")]
    pub act_closure_title: String,
    #[serde(default = "default_act_closure_current_focus_label")]
    pub act_closure_current_focus_label: String,
    #[serde(default = "default_act_closure_highlights_label")]
    pub act_closure_highlights_label: String,
    #[serde(default = "default_act_closure_relationships_label")]
    pub act_closure_relationships_label: String,
    #[serde(default = "default_act_closure_empty_highlights")]
    pub act_closure_empty_highlights: String,
    #[serde(default = "default_act_closure_empty_relationships")]
    pub act_closure_empty_relationships: String,
    #[serde(default = "default_final_summary_title")]
    pub final_summary_title: String,
    #[serde(default = "default_final_summary_highlights_label")]
    pub final_summary_highlights_label: String,
    #[serde(default = "default_final_summary_relationships_label")]
    pub final_summary_relationships_label: String,
    #[serde(default = "default_final_summary_preview_label")]
    pub final_summary_preview_label: String,
    #[serde(default = "default_final_summary_empty_preview")]
    pub final_summary_empty_preview: String,
    #[serde(default = "default_exit_confirm_title")]
    pub exit_confirm_title: String,
    #[serde(default = "default_exit_confirm_body")]
    pub exit_confirm_body: String,
    #[serde(default = "default_projector_skip_hint")]
    pub projector_skip_hint: String,
    #[serde(default = "default_projector_title_prefix")]
    pub projector_title_prefix: String,
    #[serde(default = "default_error_prefix")]
    pub error_prefix: String,
    #[serde(default = "default_response_worker_disconnected")]
    pub response_worker_disconnected: String,
    #[serde(default = "default_menu_unavailable")]
    pub menu_unavailable: String,
    #[serde(default = "default_npc_tick_soft_error")]
    pub npc_tick_soft_error: String,
    #[serde(default = "default_follow_actor_prompt")]
    pub follow_actor_prompt: String,
    #[serde(default = "default_follow_nobody_option")]
    pub follow_nobody_option: String,
    #[serde(default = "default_follow_unknown_actor_name")]
    pub follow_unknown_actor_name: String,
    #[serde(default = "default_room_switch_prompt")]
    pub room_switch_prompt: String,
    #[serde(default = "default_commands_panel_title")]
    pub commands_panel_title: String,
    #[serde(default = "default_commands_panel_empty")]
    pub commands_panel_empty: String,
    #[serde(default = "default_commands_group_other")]
    pub commands_group_other: String,
    #[serde(default = "default_commands_group_support")]
    pub commands_group_support: String,
    #[serde(default = "default_commands_group_act")]
    pub commands_group_act: String,
    #[serde(default = "default_look_panel_title")]
    pub look_panel_title: String,
    #[serde(default = "default_look_group_room")]
    pub look_group_room: String,
    #[serde(default = "default_look_group_things")]
    pub look_group_things: String,
    #[serde(default = "default_look_group_people")]
    pub look_group_people: String,
    #[serde(default = "default_perspective_review_prompt")]
    pub perspective_review_prompt: String,
    #[serde(default = "default_perspective_review_system")]
    pub perspective_review_system: String,
    #[serde(default = "default_book_recommender_instructions")]
    pub book_recommender_instructions: String,
    #[serde(default = "default_dynamic_menu_context_label")]
    pub dynamic_menu_context_label: String,
    /// Section title for loose items lying in the current room, shown in the
    /// sidebar. Layla calls this "On the ground"; story packs that don't drop
    /// items in rooms can rename or repurpose it.
    #[serde(default = "default_room_items_sidebar_label")]
    pub room_items_sidebar_label: String,
    /// Verb label for the generic `take <item>` action-bar button/panel.
    #[serde(default = "default_take_label")]
    pub take_label: String,
    /// Verb label for the generic `drop <item>` action/panel.
    #[serde(default = "default_drop_label")]
    pub drop_label: String,
    #[serde(default)]
    pub act_closure: ActClosureDefinition,
    #[serde(default)]
    pub game_closure: ActClosureDefinition,
    #[serde(default)]
    pub shell_menu: ShellMenuDefinition,
}

pub(super) fn default_language_name() -> String {
    "English".to_string()
}

pub(super) fn default_menu_button_label() -> String {
    "? Menu".to_string()
}

pub(super) fn default_act_ended_title() -> String {
    "Session Ended".to_string()
}

pub(super) fn default_game_over_hint() -> String {
    "↑/↓ or PageUp/PageDown scroll • q exits".to_string()
}

pub(super) fn default_menu_option_list_title() -> String {
    "Choose an option".to_string()
}

pub(super) fn default_menu_choice_hint() -> String {
    "Use ↑/↓ and Enter to choose.".to_string()
}

pub(super) fn default_menu_choice_transcript() -> String {
    "chose {title}".to_string()
}

pub(super) fn default_shell_menu_title() -> String {
    "Menu".to_string()
}

pub(super) fn default_shell_menu_close_hint() -> String {
    "Use ↑/↓ and Enter to choose. Esc closes.".to_string()
}

pub(super) fn default_help_label() -> String {
    "Help".to_string()
}

pub(super) fn default_resume_label() -> String {
    "Resume".to_string()
}

pub(super) fn default_things_to_do_label() -> String {
    "Things to do".to_string()
}

pub(super) fn default_about_label() -> String {
    "About".to_string()
}

pub(super) fn default_exit_label() -> String {
    "Exit".to_string()
}

pub(super) fn default_language_menu_label() -> String {
    "Language".to_string()
}

pub(super) fn default_room_switcher_label() -> String {
    "Rooms".to_string()
}

pub(super) fn default_room_switcher_title() -> String {
    "Switch rooms".to_string()
}

pub(super) fn default_room_switch_transcript() -> String {
    "switched to {title}".to_string()
}

pub(super) fn default_follow_actor_title() -> String {
    "Follow someone".to_string()
}

pub(super) fn default_follow_actor_transcript() -> String {
    "following {title}".to_string()
}

pub(super) fn default_follow_actor_stop_transcript() -> String {
    "stopped following anyone".to_string()
}

pub(super) fn default_things_to_do_empty() -> String {
    "Nothing pressing right now.".to_string()
}

pub(super) fn default_about_body() -> String {
    "Made with love by 9 Trinkets\nwww.9trinkets.com".to_string()
}

pub(super) fn default_language_modal_title() -> String {
    "Display language".to_string()
}

pub(super) fn default_language_changed_text() -> String {
    "Display language changed to {language_name}.".to_string()
}

pub(super) fn default_modal_close_hint() -> String {
    "Press Enter or Esc to close.".to_string()
}

pub(super) fn default_act_closure_title() -> String {
    "End of Day {day_number}".to_string()
}

pub(super) fn default_act_closure_current_focus_label() -> String {
    "Current focus".to_string()
}

pub(super) fn default_act_closure_highlights_label() -> String {
    "Highlights".to_string()
}

pub(super) fn default_act_closure_relationships_label() -> String {
    "Relationship status".to_string()
}

pub(super) fn default_act_closure_empty_highlights() -> String {
    "A quiet stretch, mostly observation and drift.".to_string()
}

pub(super) fn default_act_closure_empty_relationships() -> String {
    "No clear relationship shifts yet.".to_string()
}

pub(super) fn default_final_summary_title() -> String {
    "Chapter Recap".to_string()
}

pub(super) fn default_final_summary_highlights_label() -> String {
    "What happened".to_string()
}

pub(super) fn default_final_summary_relationships_label() -> String {
    "Relationship status".to_string()
}

pub(super) fn default_final_summary_preview_label() -> String {
    "Next chapter".to_string()
}

pub(super) fn default_final_summary_empty_preview() -> String {
    "The next chapter is still taking shape.".to_string()
}

pub(super) fn default_exit_confirm_title() -> String {
    "Exit act?".to_string()
}

pub(super) fn default_exit_confirm_body() -> String {
    "Press Enter to exit to the terminal, or Esc to keep playing.".to_string()
}

pub(super) fn default_projector_skip_hint() -> String {
    "Press Enter to skip ahead. Esc closes.".to_string()
}

pub(super) fn default_projector_title_prefix() -> String {
    "Projector".to_string()
}

pub(super) fn default_error_prefix() -> String {
    "Error:".to_string()
}

pub(super) fn default_response_worker_disconnected() -> String {
    "response worker disconnected.".to_string()
}

pub(super) fn default_menu_unavailable() -> String {
    "That menu is no longer available.".to_string()
}

pub(super) fn default_npc_tick_soft_error() -> String {
    "{actor_name} blanks for a second, looking briefly confused.".to_string()
}

pub(super) fn default_follow_actor_prompt() -> String {
    "Choose someone to follow.".to_string()
}

pub(super) fn default_follow_nobody_option() -> String {
    "Nobody".to_string()
}

pub(super) fn default_follow_unknown_actor_name() -> String {
    "Someone".to_string()
}

pub(super) fn default_room_switch_prompt() -> String {
    "Switch channels from {}.".to_string()
}

pub(super) fn default_commands_panel_title() -> String {
    "Commands".to_string()
}

pub(super) fn default_commands_panel_empty() -> String {
    "No additional commands available.".to_string()
}

pub(super) fn default_commands_group_other() -> String {
    "Other".to_string()
}

pub(super) fn default_commands_group_support() -> String {
    "Support".to_string()
}

pub(super) fn default_commands_group_act() -> String {
    "Act".to_string()
}

pub(super) fn default_look_panel_title() -> String {
    "Look".to_string()
}

pub(super) fn default_look_group_room() -> String {
    "Room".to_string()
}

pub(super) fn default_look_group_things() -> String {
    "Things".to_string()
}

pub(super) fn default_look_group_people() -> String {
    "People".to_string()
}

pub(super) fn default_perspective_review_prompt() -> String {
    r#"Cast Member: {actor_name}
Other: {other_person_name}

Outcome
{stats_context}

Summary
{act_summary}

Relationship Notes
{relationship_lines}

Write a short review from {actor_name}'s perspective about their experience with {other_person_name}. Be specific and in character.

Return ONLY valid JSON (no markdown, no backticks) with a rating field (1-5 integer) and a review_text field (string).
The rating should reflect the cast member's genuine experience based on the outcome.
The review text should be 2-5 sentences in the cast member's voice — honest and specific."#
        .to_string()
}

pub(super) fn default_perspective_review_system() -> String {
    "You write a short review from a cast member's perspective. Respond only with valid JSON."
        .to_string()
}

pub(super) fn default_book_recommender_instructions() -> String {
    "Generate exactly 3 fictional book recommendations. Each option must be a plausible novel title paired with a one-line thematic blurb that fits this specific character and this specific conversation. None of the options should be framed as the correct answer."
        .to_string()
}

pub(super) fn default_dynamic_menu_context_label() -> String {
    "Context".to_string()
}

pub(super) fn default_room_items_sidebar_label() -> String {
    "On the ground".to_string()
}

pub(super) fn default_take_label() -> String {
    "Take".to_string()
}

pub(super) fn default_drop_label() -> String {
    "Drop".to_string()
}

