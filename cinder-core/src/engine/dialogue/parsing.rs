use super::{
    ActorTurnActionDecision, ActorTurnAffordanceOption, DirectSpeechIntentDecision,
    MenuIntentDecision,
};

pub(super) struct ActorTurnActionParseContext<'a> {
    pub(super) affordances: &'a [ActorTurnAffordanceOption],
}

pub(super) fn parse_menu_intent_label(label: &str) -> Result<MenuIntentDecision, String> {
    let normalized = label.trim().to_ascii_uppercase();
    match normalized.as_str() {
        "OPEN" => Ok(MenuIntentDecision {
            should_open: true,
            label: "OPEN".to_string(),
        }),
        "PASS" => Ok(MenuIntentDecision {
            should_open: false,
            label: "PASS".to_string(),
        }),
        _ => Err(format!(
            "menu intent backend returned '{}'; expected OPEN or PASS",
            label.trim()
        )),
    }
}

pub(super) fn parse_direct_speech_intent_label(
    label: &str,
) -> Result<DirectSpeechIntentDecision, String> {
    match label.trim().to_ascii_uppercase().as_str() {
        "NONE" => Ok(DirectSpeechIntentDecision::None),
        "WARM" => Ok(DirectSpeechIntentDecision::Warm),
        "FLIRTY" => Ok(DirectSpeechIntentDecision::Flirty),
        "VALIDATING" => Ok(DirectSpeechIntentDecision::Validating),
        "CHALLENGING" => Ok(DirectSpeechIntentDecision::Challenging),
        _ => Err(format!(
            "direct speech attraction intent backend returned '{}'; expected NONE, WARM, FLIRTY, VALIDATING, or CHALLENGING",
            label.trim()
        )),
    }
}

pub(super) fn parse_actor_turn_action(
    label: &str,
    context: &ActorTurnActionParseContext<'_>,
) -> Result<ActorTurnActionDecision, String> {
    let trimmed = label.trim();
    let normalized = strip_actor_turn_annotation(trimmed);
    for affordance in context.affordances {
        if strip_actor_turn_annotation(&affordance.decision_label).eq_ignore_ascii_case(normalized)
        {
            return affordance.invocation.clone().into_decision(None);
        }
        if let Some(prefix) = affordance.decision_prefix.as_deref()
            && normalized
                .get(..prefix.len())
                .is_some_and(|candidate| candidate.eq_ignore_ascii_case(prefix))
        {
            let text = normalized[prefix.len()..]
                .trim_start_matches([' ', ':', '-', '—'])
                .trim();
            return affordance.invocation.clone().into_decision(Some(text));
        }
    }
    Err(format!(
        "actor turn decider returned '{}'; expected one of the authored affordance commands from the prompt's Decision section",
        trimmed
    ))
}

fn strip_actor_turn_annotation(label: &str) -> &str {
    label
        .split_once(" — ")
        .map(|(head, _)| head.trim_end())
        .or_else(|| label.split_once(" - ").map(|(head, _)| head.trim_end()))
        .unwrap_or(label)
}
