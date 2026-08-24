use super::{
    ActorTurnActionDecision, ActorTurnAffordanceOption, DirectSpeechIntentDecision,
    HostilityPlanDecision, MenuIntentDecision,
};
use serde_json::Value;

pub(super) struct ActorTurnActionParseContext<'a> {
    pub(super) affordances: &'a [ActorTurnAffordanceOption],
}

pub(super) fn parse_menu_intent_label(label: &str) -> Result<MenuIntentDecision, String> {
    let normalized = normalize_enum_label(label);
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
    let trimmed = label.trim().to_ascii_uppercase();
    if trimmed.is_empty() {
        return Err("direct speech intent backend returned empty label".to_string());
    }
    Ok(DirectSpeechIntentDecision(trimmed))
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

pub(super) fn parse_hostility_plan(
    text: &str,
    candidate_ids: &[String],
) -> Result<HostilityPlanDecision, String> {
    let trimmed = text.trim();
    let json_start = trimmed.find('{');
    let json_end = trimmed.rfind('}');
    if let (Some(start), Some(end)) = (json_start, json_end)
        && start < end
    {
        let payload = &trimmed[start..=end];
        let value: Value = serde_json::from_str(payload)
            .map_err(|error| format!("hostility planner returned invalid JSON: {error}"))?;
        let Some(strikes) = value.get("strikes") else {
            return Err("hostility planner JSON is missing the 'strikes' field".to_string());
        };
        let Value::Array(items) = strikes else {
            return Err("hostility planner 'strikes' field must be an array".to_string());
        };
        let mut decision = HostilityPlanDecision::default();
        for item in items {
            let Some(actor_id) = item.as_str() else {
                return Err("hostility planner strike entries must be actor id strings".to_string());
            };
            if !candidate_ids.iter().any(|candidate| candidate == actor_id) {
                return Err(format!(
                    "hostility planner returned unknown or ineligible actor '{actor_id}'"
                ));
            }
            if !decision.strikes.contains(&actor_id.to_string()) {
                decision.strikes.push(actor_id.to_string());
            }
        }
        return Ok(decision);
    }
    let normalized = normalize_enum_label(trimmed);
    match normalized.as_str() {
        "WAIT" | "HOLD" | "" => Ok(HostilityPlanDecision::default()),
        _ => Err(format!(
            "hostility planner returned '{}'; expected a JSON object with a 'strikes' array or WAIT",
            trimmed
        )),
    }
}

fn normalize_enum_label(label: &str) -> String {
    label
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_end_matches([':', '-', '—'])
        .to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::{parse_hostility_plan, parse_menu_intent_label};

    #[test]
    fn parses_annotated_open_menu_intent() {
        let decision = parse_menu_intent_label("OPEN — the player clearly wants a snack")
            .expect("parse annotated OPEN");
        assert!(decision.should_open);
        assert_eq!(decision.label, "OPEN");
    }

    #[test]
    fn parses_explained_pass_menu_intent() {
        let decision =
            parse_menu_intent_label("PASS because this is only small talk").expect("parse PASS");
        assert!(!decision.should_open);
        assert_eq!(decision.label, "PASS");
    }

    fn candidate_ids() -> Vec<String> {
        vec!["dark_golem".to_string(), "pale_golem".to_string()]
    }

    #[test]
    fn parses_hostility_plan_json_with_fencing() {
        let text = "Sure!\n```json\n{\"strikes\": [\"dark_golem\", \"dark_golem\"]}\n```";
        let decision = parse_hostility_plan(text, &candidate_ids()).expect("parse plan");
        assert_eq!(decision.strikes, vec!["dark_golem".to_string()]);
    }

    #[test]
    fn parses_hostility_plan_wait() {
        let decision = parse_hostility_plan("WAIT", &candidate_ids()).expect("parse WAIT");
        assert!(decision.strikes.is_empty());
    }

    #[test]
    fn rejects_hostility_plan_unknown_actor() {
        let error = parse_hostility_plan("{\"strikes\": [\"ghost\"]}", &candidate_ids())
            .expect_err("unknown actor rejected");
        assert!(error.contains("unknown or ineligible"));
    }
}
