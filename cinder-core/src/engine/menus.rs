use crate::content::types::{ContentPack, OpeningMenuDefinition};
use crate::engine::dialogue::{DialogueGenerator, MenuIntentRequest};
use crate::engine::dialogue_grounding::{
    build_current_beat_notes, build_setting_notes, current_objective_beat_notes,
    viewer_participant_id,
};
use crate::engine::events::WorldEvent;
use crate::engine::reducer::render_actor_speech_line;
use crate::engine::state::{WorldState, display_actor_name, render_dynamic_story_text};

pub(crate) struct PendingMenuDialogue<'a> {
    pub actor_id: &'a str,
    pub current_room_id: &'a str,
    pub other_person_message: Option<&'a str>,
}

pub(crate) fn render_menu_prompt(
    content: &ContentPack,
    menu: &OpeningMenuDefinition,
    state: &WorldState,
) -> String {
    let actor_line = if menu.proposal_line.is_empty() {
        String::new()
    } else {
        let actor_name = content
            .actor(&menu.actor_id)
            .map(|actor| display_actor_name(state, actor))
            .unwrap_or_else(|| menu.actor_id.clone());
        render_actor_speech_line(
            content,
            &actor_name,
            Some(&content.opening.title),
            &menu.proposal_line,
        )
    };
    match menu.selection_prompt.is_empty() {
        false => render_dynamic_story_text(&menu.selection_prompt, state),
        true => render_dynamic_story_text(&actor_line, state),
    }
}

pub(crate) fn menu_to_offer_for_pending_dialogue<'a, F>(
    content: &'a ContentPack,
    state: &WorldState,
    dialogue: &dyn DialogueGenerator,
    pending: PendingMenuDialogue<'_>,
    mut emit_trace: F,
) -> Result<Option<&'a OpeningMenuDefinition>, String>
where
    F: FnMut(&str, &str, serde_json::Value) -> Result<(), String>,
{
    let candidate_menus = content
        .menus
        .iter()
        .filter(|menu| {
            state
                .active_objective_stage_ids
                .iter()
                .any(|stage_id| stage_id == &menu.stage_id)
                && pending.actor_id == menu.actor_id
        })
        .collect::<Vec<_>>();
    let objective_notes = current_objective_beat_notes(content, state);
    let current_time_note = content.render_template(
        &content.system_text.prompt_time_note,
        &[("current_time", state.current_time_label().as_str())],
    );
    let actor = content
        .actor(pending.actor_id)
        .ok_or_else(|| format!("missing actor '{}'", pending.actor_id))?;
    let room = content
        .room(pending.current_room_id)
        .ok_or_else(|| format!("missing room '{}'", pending.current_room_id))?;
    let viewer_id = viewer_participant_id(content);
    let recent_memory = state
        .conversation_history(pending.actor_id, &viewer_id)
        .iter()
        .rev()
        .filter(|line| {
            !pending
                .other_person_message
                .is_some_and(|message| line.speaker_id == viewer_id && line.text == message)
        })
        .take(6)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let setting_notes = build_setting_notes(content, state, actor, room, &current_time_note);
    let current_beat_notes = build_current_beat_notes(
        content,
        room,
        &viewer_id,
        &content.opening.title,
        pending.other_person_message,
        &objective_notes,
    );

    for menu in candidate_menus {
        let should_open = match menu.trigger_mode {
            crate::content::types::MenuTriggerMode::Agreement => {
                is_movie_agreement_message(pending.other_person_message)
            }
            crate::content::types::MenuTriggerMode::AnySpeak => true,
            crate::content::types::MenuTriggerMode::IntentClarified => {
                let request = MenuIntentRequest {
                    menu_id: menu.id.clone(),
                    actor_id: actor.id.clone(),
                    actor_name: actor.name.clone(),
                    other_person_id: viewer_id.clone(),
                    other_person_name: content.opening.title.clone(),
                    locale: content.locale.clone(),
                    system_text: content.system_text.clone(),
                    setting_notes: setting_notes.clone(),
                    current_beat_notes: current_beat_notes.clone(),
                    recent_memory: recent_memory.clone(),
                    other_person_message: pending.other_person_message.map(str::to_string),
                    intent_guidance: menu.intent_guidance.clone(),
                    option_titles: menu
                        .options
                        .iter()
                        .map(|option| option.title.clone())
                        .collect(),
                };
                let trace_backend = dialogue.trace_metadata("menu_intent_clarifier");
                emit_trace(
                    "menu_intent_clarifier",
                    "model.request",
                    serde_json::json!({
                        "menu_id": request.menu_id,
                        "actor_id": request.actor_id,
                        "actor_name": request.actor_name,
                        "dialogue_request": request.clone(),
                        "prompt": dialogue.build_menu_intent_prompt(&request),
                        "backend": trace_backend.clone(),
                    }),
                )?;
                match dialogue.clarify_menu_intent(&request) {
                    Ok(decision) => {
                        emit_trace(
                            "menu_intent_clarifier",
                            "model.response",
                            serde_json::json!({
                                "menu_id": request.menu_id,
                                "actor_id": request.actor_id,
                                "actor_name": request.actor_name,
                                "decision": decision.label,
                                "should_open": decision.should_open,
                                "backend": trace_backend,
                            }),
                        )?;
                        decision.should_open
                    }
                    Err(error) => {
                        emit_trace(
                            "menu_intent_clarifier",
                            "model.response",
                            serde_json::json!({
                                "menu_id": request.menu_id,
                                "actor_id": request.actor_id,
                                "actor_name": request.actor_name,
                                "error": error.clone(),
                                "backend": trace_backend,
                            }),
                        )?;
                        false
                    }
                }
            }
        };
        if should_open {
            return Ok(Some(menu));
        }
    }
    Ok(None)
}

pub(crate) fn resolve_menu_choice<'a>(
    menu: &'a OpeningMenuDefinition,
    raw_input: &str,
) -> Option<&'a crate::content::types::OpeningMenuOptionDefinition> {
    resolve_menu_choice_in_options(&menu.options, raw_input)
}

pub(crate) fn resolve_menu_choice_in_options<'a>(
    options: &'a [crate::content::types::OpeningMenuOptionDefinition],
    raw_input: &str,
) -> Option<&'a crate::content::types::OpeningMenuOptionDefinition> {
    let trimmed = raw_input.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(index) = trimmed.parse::<usize>() {
        return options.get(index.saturating_sub(1));
    }
    let lower = trimmed.to_ascii_lowercase();
    options.iter().find(|option| {
        let title = option.title.to_ascii_lowercase();
        let id = option.id.to_ascii_lowercase();
        lower == title || lower == id || title.contains(&lower) || lower.contains(&title)
    })
}

pub(crate) fn build_menu_choice_events(
    content: &ContentPack,
    state: &WorldState,
    menu: &OpeningMenuDefinition,
    option: &crate::content::types::OpeningMenuOptionDefinition,
) -> Vec<WorldEvent> {
    let mut events = vec![WorldEvent::MenuChoiceMade {
        menu_id: menu.id.clone(),
        option_id: option.id.clone(),
        title: option.title.clone(),
    }];
    events.extend(
        menu.actor_relocations
            .iter()
            .map(|relocation| WorldEvent::ActorRelocated {
                actor_id: relocation.actor_id.clone(),
                to_room_id: relocation.to_room_id.clone(),
            }),
    );
    events.extend(
        menu.narrative_lines
            .iter()
            .map(|line| WorldEvent::NarrativeLine {
                text: render_dynamic_story_text(
                    &content.render_template(
                        line,
                        &[
                            ("selection_title", option.title.as_str()),
                            (menu.selection_var_key.as_str(), option.title.as_str()),
                        ],
                    ),
                    state,
                ),
            }),
    );
    events.extend(
        option
            .narrative_lines
            .iter()
            .map(|line| WorldEvent::NarrativeLine {
                text: render_dynamic_story_text(
                    &content.render_template(
                        line,
                        &[
                            ("selection_title", option.title.as_str()),
                            (menu.selection_var_key.as_str(), option.title.as_str()),
                        ],
                    ),
                    state,
                ),
            }),
    );
    events
}

fn is_movie_agreement_message(message: Option<&str>) -> bool {
    let Some(message) = message else {
        return false;
    };
    let lower = message.to_ascii_lowercase();
    lower.contains("watch")
        || lower.contains("movie")
        || lower.contains("film")
        || lower.contains("sounds good")
        || lower.contains("sure")
        || lower.contains("yeah")
        || lower.contains("yes")
        || lower.contains("okay")
        || lower.contains("ok")
}

#[cfg(test)]
mod tests {
    use super::{
        PendingMenuDialogue, build_menu_choice_events, menu_to_offer_for_pending_dialogue,
        render_menu_prompt,
    };
    use crate::content::loader::load_named_pack;
    use crate::content::types::SpeechIntentLabel;
    use crate::engine::dialogue::DialogueGenerator;
    use crate::engine::dialogue::types::{
        ActorTurnActionDecision, ActorTurnActionRequest, ConversationMemorySummaryRequest,
        DialogueRequest, DirectSpeechIntentDecision, DirectSpeechIntentRequest,
        DynamicMenuOptionOutput, DynamicMenuRequest, MenuIntentDecision, MenuIntentRequest,
        PerspectiveReview, PerspectiveReviewRequest, StageAssignment, StageAssignmentRequest,
    };
    use crate::engine::events::WorldEvent;
    use crate::engine::state::{
        WorldState, advance_to_next_appointment, initialize_appointment_state,
    };

    struct FailingMenuIntentDialogue;

    impl DialogueGenerator for FailingMenuIntentDialogue {
        fn generate(&self, _request: &DialogueRequest) -> Result<String, String> {
            Err("not used in test".to_string())
        }

        fn clarify_menu_intent(
            &self,
            _request: &MenuIntentRequest,
        ) -> Result<MenuIntentDecision, String> {
            Err("planning rejected too many times".to_string())
        }

        fn choose_actor_turn_action(
            &self,
            _request: &ActorTurnActionRequest,
        ) -> Result<ActorTurnActionDecision, String> {
            Err("not used in test".to_string())
        }

        fn summarize_conversation_memory(
            &self,
            _request: &ConversationMemorySummaryRequest,
        ) -> Result<String, String> {
            Err("not used in test".to_string())
        }

        fn extract_direct_speech_intent(
            &self,
            _request: &DirectSpeechIntentRequest,
            _intents: &[SpeechIntentLabel],
        ) -> Result<DirectSpeechIntentDecision, String> {
            Err("not used in test".to_string())
        }

        fn generate_dynamic_menu_options(
            &self,
            _request: &DynamicMenuRequest,
        ) -> Result<Vec<DynamicMenuOptionOutput>, String> {
            Err("not used in test".to_string())
        }

        fn generate_perspective_review(
            &self,
            _request: &PerspectiveReviewRequest,
        ) -> Result<PerspectiveReview, String> {
            Err("not used in test".to_string())
        }

        fn assign_stage_participants(
            &self,
            _request: &StageAssignmentRequest,
        ) -> Result<StageAssignment, String> {
            Err("not used in test".to_string())
        }
    }

    #[test]
    fn recommendation_prompt_uses_current_patient_name() {
        let content = load_named_pack("isla", None).expect("load isla");
        let mut state = WorldState::new(&content);
        initialize_appointment_state(&content, &mut state);
        advance_to_next_appointment(&content, &mut state, None);
        let menu = content
            .menu("book-recommendation")
            .expect("book recommendation menu");

        let prompt = render_menu_prompt(&content, menu, &state);

        assert!(prompt.contains("Awa"));
        assert!(!prompt.contains("Noa"));
    }

    #[test]
    fn menu_narrative_lines_use_current_patient_name() {
        let content = load_named_pack("isla", None).expect("load isla");
        let mut state = WorldState::new(&content);
        initialize_appointment_state(&content, &mut state);
        advance_to_next_appointment(&content, &mut state, None);
        let menu = content.menu("request-quarter").expect("quarter menu");
        let option = menu
            .options
            .iter()
            .find(|option| option.id == "quarter-coffee")
            .expect("quarter coffee option");

        let events = build_menu_choice_events(&content, &state, menu, option);
        let narrative_lines = events
            .into_iter()
            .filter_map(|event| match event {
                WorldEvent::NarrativeLine { text } => Some(text),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(narrative_lines.iter().any(|line| line.contains("Awa")));
        assert!(narrative_lines.iter().all(|line| !line.contains("Noa")));
    }

    #[test]
    fn menu_intent_failure_falls_back_without_aborting_turn() {
        let content = load_named_pack("ella", None).expect("load ella");
        let mut state = WorldState::new(&content);
        state.current_room_id = "kitchen".to_string();
        state.active_objective_stage_ids = vec!["talk-with-dad".to_string()];

        let offered = menu_to_offer_for_pending_dialogue(
            &content,
            &state,
            &FailingMenuIntentDialogue,
            PendingMenuDialogue {
                actor_id: "dad",
                current_room_id: "kitchen",
                other_person_message: Some("Hey Dad."),
            },
            |_role, _topic, _payload| Ok(()),
        )
        .expect("menu intent failure should not abort");

        assert!(offered.is_none());
    }
}
