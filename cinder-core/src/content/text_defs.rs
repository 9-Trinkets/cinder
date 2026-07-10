use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShellMenuDefinition {
    #[serde(default)]
    pub items: Vec<ShellMenuItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellMenuItem {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub children: Vec<ShellMenuItem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionBarDefinition {
    #[serde(default)]
    pub actions: Vec<ActionBarItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionBarItem {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiTextDefinition {
    #[serde(default = "default_language_name")]
    pub language_name: String,
    #[serde(default = "default_menu_button_label")]
    pub menu_button_label: String,
    #[serde(default = "default_session_ended_title")]
    pub session_ended_title: String,
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
    #[serde(default = "default_day_summary_title")]
    pub day_summary_title: String,
    #[serde(default = "default_day_summary_current_focus_label")]
    pub day_summary_current_focus_label: String,
    #[serde(default = "default_day_summary_highlights_label")]
    pub day_summary_highlights_label: String,
    #[serde(default = "default_day_summary_relationships_label")]
    pub day_summary_relationships_label: String,
    #[serde(default = "default_day_summary_empty_highlights")]
    pub day_summary_empty_highlights: String,
    #[serde(default = "default_day_summary_empty_relationships")]
    pub day_summary_empty_relationships: String,
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
    #[serde(default = "default_commands_modal_title")]
    pub commands_modal_title: String,
    #[serde(default = "default_commands_modal_empty")]
    pub commands_modal_empty: String,
    #[serde(default = "default_commands_group_other")]
    pub commands_group_other: String,
    #[serde(default = "default_commands_group_support")]
    pub commands_group_support: String,
    #[serde(default = "default_look_modal_title")]
    pub look_modal_title: String,
    #[serde(default = "default_look_group_room")]
    pub look_group_room: String,
    #[serde(default = "default_look_group_things")]
    pub look_group_things: String,
    #[serde(default = "default_look_group_people")]
    pub look_group_people: String,
    #[serde(default = "default_talk_modal_title")]
    pub talk_modal_title: String,
    #[serde(default = "default_talk_modal_prompt")]
    pub talk_modal_prompt: String,
    #[serde(default)]
    pub shell_menu: ShellMenuDefinition,
    #[serde(default)]
    pub action_bar: ActionBarDefinition,
}

pub use crate::content::system_text_defs::SystemTextDefinition;

fn default_language_name() -> String {
    "English".to_string()
}

fn default_menu_button_label() -> String {
    "? Menu".to_string()
}

fn default_session_ended_title() -> String {
    "Session Ended".to_string()
}

fn default_game_over_hint() -> String {
    "↑/↓ or PageUp/PageDown scroll • q exits".to_string()
}

fn default_menu_option_list_title() -> String {
    "Choose an option".to_string()
}

fn default_menu_choice_hint() -> String {
    "Use ↑/↓ and Enter to choose.".to_string()
}

fn default_menu_choice_transcript() -> String {
    "chose {title}".to_string()
}

fn default_shell_menu_title() -> String {
    "Menu".to_string()
}

fn default_shell_menu_close_hint() -> String {
    "Use ↑/↓ and Enter to choose. Esc closes.".to_string()
}

fn default_help_label() -> String {
    "Help".to_string()
}

fn default_resume_label() -> String {
    "Resume".to_string()
}

fn default_things_to_do_label() -> String {
    "Things to do".to_string()
}

fn default_about_label() -> String {
    "About".to_string()
}

fn default_exit_label() -> String {
    "Exit".to_string()
}

fn default_language_menu_label() -> String {
    "Language".to_string()
}

fn default_room_switcher_label() -> String {
    "Rooms".to_string()
}

fn default_room_switcher_title() -> String {
    "Switch rooms".to_string()
}

fn default_room_switch_transcript() -> String {
    "switched to {title}".to_string()
}

fn default_follow_actor_title() -> String {
    "Follow someone".to_string()
}

fn default_follow_actor_transcript() -> String {
    "following {title}".to_string()
}

fn default_follow_actor_stop_transcript() -> String {
    "stopped following anyone".to_string()
}

fn default_things_to_do_empty() -> String {
    "Nothing pressing right now.".to_string()
}

fn default_about_body() -> String {
    "Made with love by 9 Trinkets\nwww.9trinkets.com".to_string()
}

fn default_language_modal_title() -> String {
    "Display language".to_string()
}

fn default_language_changed_text() -> String {
    "Display language changed to {language_name}.".to_string()
}

fn default_modal_close_hint() -> String {
    "Press Enter or Esc to close.".to_string()
}

fn default_day_summary_title() -> String {
    "End of Day {day_number}".to_string()
}

fn default_day_summary_current_focus_label() -> String {
    "Current focus".to_string()
}

fn default_day_summary_highlights_label() -> String {
    "Highlights".to_string()
}

fn default_day_summary_relationships_label() -> String {
    "Relationship status".to_string()
}

fn default_day_summary_empty_highlights() -> String {
    "A quiet stretch, mostly observation and drift.".to_string()
}

fn default_day_summary_empty_relationships() -> String {
    "No clear relationship shifts yet.".to_string()
}

fn default_final_summary_title() -> String {
    "Chapter Recap".to_string()
}

fn default_final_summary_highlights_label() -> String {
    "What happened".to_string()
}

fn default_final_summary_relationships_label() -> String {
    "Relationship status".to_string()
}

fn default_final_summary_preview_label() -> String {
    "Next chapter".to_string()
}

fn default_final_summary_empty_preview() -> String {
    "The next chapter is still taking shape.".to_string()
}

fn default_exit_confirm_title() -> String {
    "Exit session?".to_string()
}

fn default_exit_confirm_body() -> String {
    "Press Enter to exit to the terminal, or Esc to keep playing.".to_string()
}

fn default_projector_skip_hint() -> String {
    "Press Enter to skip ahead. Esc closes.".to_string()
}

fn default_projector_title_prefix() -> String {
    "Projector".to_string()
}

fn default_error_prefix() -> String {
    "Error:".to_string()
}

fn default_response_worker_disconnected() -> String {
    "response worker disconnected.".to_string()
}

fn default_menu_unavailable() -> String {
    "That menu is no longer available.".to_string()
}

fn default_npc_tick_soft_error() -> String {
    "{actor_name} blanks for a second, looking briefly confused.".to_string()
}

fn default_follow_actor_prompt() -> String {
    "Choose someone to follow.".to_string()
}

fn default_follow_nobody_option() -> String {
    "Nobody".to_string()
}

fn default_follow_unknown_actor_name() -> String {
    "Someone".to_string()
}

fn default_room_switch_prompt() -> String {
    "Switch channels from {}.".to_string()
}

fn default_commands_modal_title() -> String {
    "Commands".to_string()
}

fn default_commands_modal_empty() -> String {
    "No additional commands available.".to_string()
}

fn default_commands_group_other() -> String {
    "Other".to_string()
}

fn default_commands_group_support() -> String {
    "Support".to_string()
}

fn default_look_modal_title() -> String {
    "Look".to_string()
}

fn default_look_group_room() -> String {
    "Room".to_string()
}

fn default_look_group_things() -> String {
    "Things".to_string()
}

fn default_look_group_people() -> String {
    "People".to_string()
}

fn default_talk_modal_title() -> String {
    "Talk".to_string()
}

fn default_talk_modal_prompt() -> String {
    "Who do you want to talk to?".to_string()
}

impl Default for UiTextDefinition {
    fn default() -> Self {
        Self {
            language_name: default_language_name(),
            menu_button_label: default_menu_button_label(),
            session_ended_title: default_session_ended_title(),
            game_over_hint: default_game_over_hint(),
            menu_option_list_title: default_menu_option_list_title(),
            menu_choice_hint: default_menu_choice_hint(),
            menu_choice_transcript: default_menu_choice_transcript(),
            shell_menu_title: default_shell_menu_title(),
            shell_menu_close_hint: default_shell_menu_close_hint(),
            help_label: default_help_label(),
            resume_label: default_resume_label(),
            things_to_do_label: default_things_to_do_label(),
            about_label: default_about_label(),
            exit_label: default_exit_label(),
            language_menu_label: default_language_menu_label(),
            room_switcher_label: default_room_switcher_label(),
            room_switcher_title: default_room_switcher_title(),
            room_switch_transcript: default_room_switch_transcript(),
            follow_actor_title: default_follow_actor_title(),
            follow_actor_transcript: default_follow_actor_transcript(),
            follow_actor_stop_transcript: default_follow_actor_stop_transcript(),
            things_to_do_empty: default_things_to_do_empty(),
            about_body: default_about_body(),
            language_modal_title: default_language_modal_title(),
            language_changed_text: default_language_changed_text(),
            modal_close_hint: default_modal_close_hint(),
            day_summary_title: default_day_summary_title(),
            day_summary_current_focus_label: default_day_summary_current_focus_label(),
            day_summary_highlights_label: default_day_summary_highlights_label(),
            day_summary_relationships_label: default_day_summary_relationships_label(),
            day_summary_empty_highlights: default_day_summary_empty_highlights(),
            day_summary_empty_relationships: default_day_summary_empty_relationships(),
            final_summary_title: default_final_summary_title(),
            final_summary_highlights_label: default_final_summary_highlights_label(),
            final_summary_relationships_label: default_final_summary_relationships_label(),
            final_summary_preview_label: default_final_summary_preview_label(),
            final_summary_empty_preview: default_final_summary_empty_preview(),
            exit_confirm_title: default_exit_confirm_title(),
            exit_confirm_body: default_exit_confirm_body(),
            projector_skip_hint: default_projector_skip_hint(),
            projector_title_prefix: default_projector_title_prefix(),
            error_prefix: default_error_prefix(),
            response_worker_disconnected: default_response_worker_disconnected(),
            menu_unavailable: default_menu_unavailable(),
            npc_tick_soft_error: default_npc_tick_soft_error(),
            follow_actor_prompt: default_follow_actor_prompt(),
            follow_nobody_option: default_follow_nobody_option(),
            follow_unknown_actor_name: default_follow_unknown_actor_name(),
            room_switch_prompt: default_room_switch_prompt(),
            commands_modal_title: default_commands_modal_title(),
            commands_modal_empty: default_commands_modal_empty(),
            commands_group_other: default_commands_group_other(),
            commands_group_support: default_commands_group_support(),
            look_modal_title: default_look_modal_title(),
            look_group_room: default_look_group_room(),
            look_group_things: default_look_group_things(),
            look_group_people: default_look_group_people(),
            talk_modal_title: default_talk_modal_title(),
            talk_modal_prompt: default_talk_modal_prompt(),
            shell_menu: ShellMenuDefinition::default(),
            action_bar: ActionBarDefinition::default(),
        }
    }
}

// ── SystemTextDefinition default helpers ─────────────────────────────────────
