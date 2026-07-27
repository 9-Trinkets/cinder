use super::{MINUTES_PER_DAY, WorldState};
use crate::content::types::{
    ActorDefinition, ActorPromptContext, ActCastMember, ContentPack,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActSeriesState {
    pub current_act_number: u32,
    pub current_patient_id: String,
    #[serde(default)]
    pub next_seed_index: usize,
    #[serde(default)]
    pub patients: BTreeMap<String, CastMemberRecord>,
    #[serde(default)]
    pub act_history: Vec<ActHistoryEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CastMemberRecord {
    pub id: String,
    pub name: String,
    pub actor_id: String,
    pub inspect_blurb: String,
    pub intro_blurb: String,
    pub return_blurb: String,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    #[serde(default)]
    pub act_count: u32,
    #[serde(default)]
    pub last_seen_act: Option<u32>,
    #[serde(default)]
    pub last_feedback_rating: Option<u32>,
    #[serde(default)]
    pub last_feedback_review: Option<String>,
    #[serde(default)]
    pub actor_stats: BTreeMap<String, i32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActHistoryEntry {
    pub act_number: u32,
    pub patient_id: String,
    pub patient_name: String,
    pub feedback_rating: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ActFeedbackSummary {
    pub rating: u32,
    pub review_text: String,
}

const ACT_CAST_NAME_VAR: &str = "act_cast_name";
const ACT_CAST_ACTOR_ID_VAR: &str = "act_cast_actor_id";
const ACT_CAST_TEMPLATE_ACTOR_ID_VAR: &str = "act_cast_template_actor_id";
const ACT_CAST_SLOT_BASE_NAME_VAR: &str = "act_cast_slot_base_name";
const ACT_OFFSTAGE_ROOM_ID: &str = "acts-offstage";

pub fn initialize_act_state(content: &ContentPack, state: &mut WorldState) {
    if content.act_cast.is_empty() {
        return;
    }
    if state.act_series.is_none() {
        state.act_series = Some(ActSeriesState::default());
    }
    let needs_bootstrap = state.act_series.as_ref().is_some_and(|series| {
        series.current_act_number == 0 || series.current_patient_id.is_empty()
    });
    if needs_bootstrap {
        bootstrap_first_act(content, state);
    } else {
        inject_act_cast_vars(content, state);
    }
}

pub fn advance_to_next_act(
    content: &ContentPack,
    state: &mut WorldState,
    feedback: Option<&ActFeedbackSummary>,
) -> Option<String> {
    if content.act_cast.is_empty() {
        return None;
    }
    let Some(mut series) = state.act_series.clone() else {
        return None;
    };
    if series.current_act_number == 0 || series.current_patient_id.is_empty() {
        bootstrap_first_act(content, state);
        return Some(
            current_act_intro(state).unwrap_or_else(|| content.opening.intro_text.clone()),
        );
    }

    if let Some(current) = series.patients.get_mut(&series.current_patient_id) {
        current.act_count = current.act_count.saturating_add(1);
        current.last_seen_act = Some(series.current_act_number);
        current.last_feedback_rating = feedback.map(|summary| summary.rating);
        current.last_feedback_review = feedback.map(|summary| summary.review_text.clone());
        current.actor_stats = state.actor_stats_snapshot(&current.actor_id);
    }
    if let Some(current) = series.patients.get(&series.current_patient_id) {
        series.act_history.push(ActHistoryEntry {
            act_number: series.current_act_number,
            patient_id: current.id.clone(),
            patient_name: current.name.clone(),
            feedback_rating: feedback.map(|summary| summary.rating),
        });
    }

    series.current_act_number = series.current_act_number.saturating_add(1);
    let next_patient_id = choose_next_cast_member_id(content, &series);
    if !series.patients.contains_key(&next_patient_id) {
        let patient_definition = content
            .act_cast
            .iter()
            .find(|definition| definition.id == next_patient_id)
            .unwrap_or_else(|| {
                panic!(
                    "missing act_cast member definition '{}'",
                    next_patient_id
                )
            });
        let patient = build_patient_record(content, state, patient_definition);
        series.next_seed_index = series.next_seed_index.saturating_add(1);
        series.patients.insert(patient.id.clone(), patient);
    }
    series.current_patient_id = next_patient_id;

    let mut next_state = WorldState::new(content);
    next_state.act_series = Some(series);
    next_state.last_transcript_line = state.last_transcript_line.clone();
    if let Some(series) = next_state.act_series.as_ref() {
        next_state.current_time_minutes = content.opening.start_time_minutes
            + (series.current_act_number.saturating_sub(1) * MINUTES_PER_DAY);
    }
    inject_act_cast_vars(content, &mut next_state);
    *state = next_state;
    Some(current_act_intro(state).unwrap_or_else(|| content.opening.intro_text.clone()))
}

pub fn display_actor_name(state: &WorldState, actor: &ActorDefinition) -> String {
    if is_current_patient_reference(state, &actor.id)
        && let Some(name) = state.story_vars.get(ACT_CAST_NAME_VAR)
    {
        return name.to_string();
    }
    actor.name.clone()
}

pub fn resolved_actor_prompt_context(
    content: &ContentPack,
    state: &WorldState,
    actor: &ActorDefinition,
) -> ActorPromptContext {
    if !is_current_patient_reference(state, &actor.id) {
        return actor.prompt_context.clone();
    }
    let Some(patient) = current_patient(state) else {
        return actor.prompt_context.clone();
    };
    let behavior_actor = content.actor(&patient.actor_id).unwrap_or(actor);
    let act_number = current_act_number(state);
    let mut character_notes = behavior_actor.prompt_context.character_notes.clone();
    character_notes.push(format!(
        "You are {}.",
        patient.name
    ));
    for (key, value) in &patient.metadata {
        if key.starts_with("char_") {
            let label = key.strip_prefix("char_").unwrap_or(key);
            character_notes.push(format!("{}: {}.", label, value));
        }
    }
    let mut subtext_notes = behavior_actor.prompt_context.subtext_notes.clone();
    subtext_notes.push(format!("Carry the emotional residue of {}.", patient.intro_blurb));
    for (key, value) in &patient.metadata {
        if key.starts_with("sub_") {
            let label = key.strip_prefix("sub_").unwrap_or(key);
            subtext_notes.push(format!("{}: {}.", label, value));
        }
    }
    let mut response_notes = behavior_actor.prompt_context.response_notes.clone();
    response_notes.push(format!(
        "You are in act {act_number}. Respond as {} would, without narrating future sessions.",
        patient.name
    ));
    if let Some(review) = patient.last_feedback_review.as_deref() {
        response_notes.push(format!("Last act takeaway: {review}"));
    }
    ActorPromptContext {
        character_notes,
        subtext_notes,
        response_notes,
        behavior_examples: behavior_actor.prompt_context.behavior_examples.clone(),
    }
}

pub fn current_act_intro(state: &WorldState) -> Option<String> {
    let patient = current_patient(state)?;
    if current_act_number(state) == 1 && patient.act_count == 0 {
        return None;
    }
    let act_number = current_act_number(state);
    let returning = patient.act_count > 0;
    let header = format!(
        "━━━━━━━━━━━━━━━━━━━━\nAct {act_number}: {}\n━━━━━━━━━━━━━━━━━━━━",
        patient.name
    );
    Some(if returning {
        let prior_note = patient
            .last_feedback_review
            .as_deref()
            .unwrap_or("they are still sorting through what happened last time");
        format!(
            "{header}\n\n{} returns. {} They arrive carrying the aftertaste of last time: {}",
            patient.name, patient.return_blurb, prior_note
        )
    } else {
        format!(
            "{header}\n\n{} arrives. {}",
            patient.name, patient.intro_blurb
        )
    })
}

pub fn current_patient_name(state: &WorldState) -> Option<String> {
    current_patient(state).map(|patient| patient.name.clone())
}

pub fn current_patient_actor_id(state: &WorldState) -> Option<&str> {
    current_patient(state).map(|patient| patient.actor_id.as_str())
}

pub fn remap_story_actor_id<'a>(state: &'a WorldState, actor_id: &'a str) -> &'a str {
    match (
        state.story_vars.get(ACT_CAST_TEMPLATE_ACTOR_ID_VAR),
        state.story_vars.get(ACT_CAST_ACTOR_ID_VAR),
    ) {
        (Some(template_actor_id), Some(current_actor_id)) if template_actor_id == actor_id => {
            current_actor_id
        }
        _ => actor_id,
    }
}

pub fn story_actor_matches(
    state: &WorldState,
    runtime_actor_id: &str,
    authored_actor_id: &str,
) -> bool {
    remap_story_actor_id(state, authored_actor_id) == runtime_actor_id
}

pub fn render_dynamic_story_text(template: &str, state: &WorldState) -> String {
    let mut rendered = state.story_vars.render_template(template);
    if let (Some(base_name), Some(current_name)) = (
        state.story_vars.get(ACT_CAST_SLOT_BASE_NAME_VAR),
        state.story_vars.get(ACT_CAST_NAME_VAR),
    ) && base_name != current_name
    {
        rendered = rendered.replace(base_name, current_name);
    }
    for (actor_id, stats) in &state.actor_stats {
        for (stat_key, stat_value) in stats {
            rendered = rendered.replace(
                &format!("{{actor.{actor_id}.{stat_key}}}"),
                &stat_value.to_string(),
            );
        }
    }
    rendered
}

fn bootstrap_first_act(content: &ContentPack, state: &mut WorldState) {
    let patient = build_patient_record(
        content,
        state,
        content
            .act_cast
            .first()
            .unwrap_or_else(|| panic!("missing act_cast member definitions")),
    );
    let Some(series) = state.act_series.as_mut() else {
        return;
    };
    series.current_act_number = 1;
    series.current_patient_id = patient.id.clone();
    series.next_seed_index = 1;
    series.patients.insert(patient.id.clone(), patient);
    inject_act_cast_vars(content, state);
}

fn inject_act_cast_vars(content: &ContentPack, state: &mut WorldState) {
    let Some(series) = state.act_series.as_ref() else {
        return;
    };
    let Some(patient) = series.patients.get(&series.current_patient_id) else {
        return;
    };
    let template_actor_id = act_template_actor_id(content).unwrap_or(&patient.actor_id);
    let base_name = content
        .actor(template_actor_id)
        .map(|actor| actor.name.clone())
        .unwrap_or_else(|| patient.name.clone());
    state
        .story_vars
        .set_unchecked(ACT_CAST_ACTOR_ID_VAR, &patient.actor_id);
    state.story_vars.set_unchecked(
        ACT_CAST_TEMPLATE_ACTOR_ID_VAR,
        template_actor_id,
    );
    state
        .story_vars
        .set_unchecked(ACT_CAST_SLOT_BASE_NAME_VAR, &base_name);
    state
        .story_vars
        .set_unchecked(ACT_CAST_NAME_VAR, &patient.name);
    state
        .story_vars
        .set_unchecked("act_number", &series.current_act_number.to_string());
    for (key, value) in &patient.metadata {
        state.story_vars.set_unchecked(&format!("patient_{key}"), value);
    }
    state.story_vars.set_unchecked(
        "patient_returning",
        if patient.act_count > 0 {
            "true"
        } else {
            "false"
        },
    );
    if let Some(stats) = state.actor_stats.get_mut(&patient.actor_id) {
        *stats = patient.actor_stats.clone();
    }
    if let Some(initial_stats) = state.initial_actor_stats.get_mut(&patient.actor_id) {
        *initial_stats = patient.actor_stats.clone();
    }
    let patient_actor_ids = content
        .act_cast
        .iter()
        .map(|definition| definition.actor_id.as_str())
        .collect::<Vec<_>>();
    for actor_id in patient_actor_ids {
        if actor_id == patient.actor_id {
            state.actor_room_overrides.remove(actor_id);
        } else {
            state.actor_room_overrides.insert(
                actor_id.to_string(),
                ACT_OFFSTAGE_ROOM_ID.to_string(),
            );
        }
    }
}

fn current_patient(state: &WorldState) -> Option<&CastMemberRecord> {
    let series = state.act_series.as_ref()?;
    series.patients.get(&series.current_patient_id)
}

fn current_act_number(state: &WorldState) -> u32 {
    state
        .act_series
        .as_ref()
        .map(|series| series.current_act_number)
        .unwrap_or(1)
}

fn is_current_patient_reference(state: &WorldState, actor_id: &str) -> bool {
    current_patient_actor_id(state).is_some_and(|current_actor_id| {
        current_actor_id == actor_id
            || state
                .story_vars
                .get(ACT_CAST_TEMPLATE_ACTOR_ID_VAR)
                .is_some_and(|template_actor_id| template_actor_id == actor_id)
    })
}

fn choose_next_cast_member_id(content: &ContentPack, series: &ActSeriesState) -> String {
    if series.current_act_number >= 3 && series.current_act_number % 2 == 1 {
        if let Some((patient_id, _)) = series
            .patients
            .iter()
            .filter(|(patient_id, _)| **patient_id != series.current_patient_id)
            .min_by_key(|(_, patient)| patient.last_seen_act.unwrap_or(0))
        {
            return patient_id.clone();
        }
    }
    if let Some(definition) = content.act_cast.get(series.next_seed_index) {
        return definition.id.clone();
    }
    series
        .patients
        .iter()
        .filter(|(patient_id, _)| **patient_id != series.current_patient_id)
        .min_by_key(|(_, patient)| patient.last_seen_act.unwrap_or(0))
        .map(|(patient_id, _)| patient_id.clone())
        .unwrap_or_else(|| series.current_patient_id.clone())
}

fn build_patient_record(
    content: &ContentPack,
    state: &WorldState,
    definition: &ActCastMember,
) -> CastMemberRecord {
    let mut actor_stats = content
        .actor(&definition.actor_id)
        .map(|actor| actor.initial_stats.clone())
        .unwrap_or_default();
    for (key, value) in &definition.actor_stats {
        actor_stats.insert(key.clone(), *value);
    }
    for (stat_key, definition) in &state.actor_stat_defs {
        actor_stats
            .entry(stat_key.clone())
            .and_modify(|value| *value = definition.clamp(*value))
            .or_insert(definition.default);
    }
    CastMemberRecord {
        id: definition.id.clone(),
        name: definition.name.clone(),
        actor_id: definition.actor_id.clone(),
        inspect_blurb: definition.inspect_blurb.clone(),
        intro_blurb: definition.intro_blurb.clone(),
        return_blurb: definition.return_blurb.clone(),
        metadata: definition.metadata.clone(),
        act_count: 0,
        last_seen_act: None,
        last_feedback_rating: None,
        last_feedback_review: None,
        actor_stats,
    }
}

fn act_template_actor_id(content: &ContentPack) -> Option<&str> {
    content
        .act_cast
        .first()
        .map(|patient| patient.actor_id.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::loader::load_named_pack;

    #[test]
    fn first_appointment_preserves_authored_patient() {
        let content = load_named_pack("isla", None).expect("load isla");
        let mut state = WorldState::new(&content);
        initialize_act_state(&content, &mut state);

        assert_eq!(current_patient_name(&state).as_deref(), Some("Noa"));
        assert_eq!(current_act_intro(&state), None);
    }

    #[test]
    fn second_appointment_uses_first_generated_patient() {
        let content = load_named_pack("isla", None).expect("load isla");
        let mut state = WorldState::new(&content);
        initialize_act_state(&content, &mut state);

        let _ = advance_to_next_act(&content, &mut state, None);

        assert_eq!(current_patient_name(&state).as_deref(), Some("Awa"));
        assert_eq!(current_patient_actor_id(&state), Some("awa"));
        let intro = current_act_intro(&state).expect("act intro");
        assert!(intro.contains("Act 2: Awa"));
        assert!(intro.contains("━━━━━━━━━━━━━━━━━━━━"));
    }

    #[test]
    fn current_patient_reference_uses_distinct_actor_prompt_context() {
        let content = load_named_pack("isla", None).expect("load isla");
        let mut state = WorldState::new(&content);
        initialize_act_state(&content, &mut state);
        let _ = advance_to_next_act(&content, &mut state, None);

        let proxy_actor = content.actor("noa").expect("proxy actor");
        let prompt_context = resolved_actor_prompt_context(&content, &state, proxy_actor);
        let joined = prompt_context
            .character_notes
            .iter()
            .chain(prompt_context.subtext_notes.iter())
            .chain(prompt_context.response_notes.iter())
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");

        assert!(joined.contains("You are Awa"));
        assert!(joined.contains("home health aide"));
        assert!(joined.contains("dry, sharp, and defensive"));
    }
}
