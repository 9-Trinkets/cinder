use super::{WorldState, MINUTES_PER_DAY};
use crate::content::types::{
    ActorDefinition, ActorPromptContext, AppointmentPatientDefinition, ContentPack,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppointmentSeriesState {
    pub patient_actor_id: String,
    pub current_appointment_number: u32,
    pub current_patient_id: String,
    #[serde(default)]
    pub next_seed_index: usize,
    #[serde(default)]
    pub patients: BTreeMap<String, PatientRecord>,
    #[serde(default)]
    pub appointment_history: Vec<AppointmentHistoryEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PatientRecord {
    pub id: String,
    pub name: String,
    pub age: u32,
    pub profession: String,
    pub presenting_issue: String,
    pub relational_pattern: String,
    pub formative_memory: String,
    pub coping_style: String,
    pub desired_change: String,
    pub bibliotherapy_fit: String,
    pub inspect_blurb: String,
    pub intro_blurb: String,
    pub return_blurb: String,
    #[serde(default)]
    pub secret_notes: Vec<String>,
    #[serde(default)]
    pub appointment_count: u32,
    #[serde(default)]
    pub last_seen_appointment: Option<u32>,
    #[serde(default)]
    pub last_feedback_rating: Option<u32>,
    #[serde(default)]
    pub last_feedback_review: Option<String>,
    #[serde(default)]
    pub actor_stats: BTreeMap<String, i32>,
    #[serde(default)]
    pub uses_authored_profile: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppointmentHistoryEntry {
    pub appointment_number: u32,
    pub patient_id: String,
    pub patient_name: String,
    pub feedback_rating: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct AppointmentFeedbackSummary {
    pub rating: u32,
    pub review_text: String,
}

const PATIENT_NAME_VAR: &str = "patient_name";
const PATIENT_ACTOR_ID_VAR: &str = "patient_actor_id";
const PATIENT_SLOT_BASE_NAME_VAR: &str = "patient_slot_base_name";

pub fn initialize_appointment_state(content: &ContentPack, state: &mut WorldState) {
    if !content.settings.multi_appointment || content.settings.appointment_patient_actor_id.is_empty() {
        return;
    }
    if state.appointment_series.is_none() {
        state.appointment_series = Some(AppointmentSeriesState {
            patient_actor_id: content.settings.appointment_patient_actor_id.clone(),
            ..AppointmentSeriesState::default()
        });
    }
    let needs_bootstrap = state
        .appointment_series
        .as_ref()
        .is_some_and(|series| series.current_appointment_number == 0 || series.current_patient_id.is_empty());
    if needs_bootstrap {
        bootstrap_first_appointment(content, state);
    } else {
        sync_current_patient_story_vars(content, state);
    }
}

pub fn advance_to_next_appointment(
    content: &ContentPack,
    state: &mut WorldState,
    feedback: Option<&AppointmentFeedbackSummary>,
) -> Option<String> {
    if !content.settings.multi_appointment {
        return None;
    }
    let Some(mut series) = state.appointment_series.clone() else {
        return None;
    };
    if series.current_appointment_number == 0 || series.current_patient_id.is_empty() {
        bootstrap_first_appointment(content, state);
        return Some(current_appointment_intro(state).unwrap_or_else(|| content.opening.intro_text.clone()));
    }

    if let Some(current) = series.patients.get_mut(&series.current_patient_id) {
        current.appointment_count = current.appointment_count.saturating_add(1);
        current.last_seen_appointment = Some(series.current_appointment_number);
        current.last_feedback_rating = feedback.map(|summary| summary.rating);
        current.last_feedback_review = feedback.map(|summary| summary.review_text.clone());
        current.actor_stats = state.actor_stats_snapshot(&series.patient_actor_id);
    }
    if let Some(current) = series.patients.get(&series.current_patient_id) {
        series.appointment_history.push(AppointmentHistoryEntry {
            appointment_number: series.current_appointment_number,
            patient_id: current.id.clone(),
            patient_name: current.name.clone(),
            feedback_rating: feedback.map(|summary| summary.rating),
        });
    }

    series.current_appointment_number = series.current_appointment_number.saturating_add(1);
    let next_patient_id = choose_next_patient_id(content, &series);
    if !series.patients.contains_key(&next_patient_id) {
        let patient_definition = content
            .appointment_patients
            .iter()
            .find(|definition| definition.id == next_patient_id)
            .unwrap_or_else(|| panic!("missing appointment patient definition '{}'", next_patient_id));
        let patient = build_patient_record(content, state, patient_definition);
        series.next_seed_index = series.next_seed_index.saturating_add(1);
        series.patients.insert(patient.id.clone(), patient);
    }
    series.current_patient_id = next_patient_id;

    let mut next_state = WorldState::new(content);
    next_state.appointment_series = Some(series);
    next_state.transcript = state.transcript.clone();
    if let Some(series) = next_state.appointment_series.as_ref() {
        next_state.current_time_minutes =
            content.opening.start_time_minutes + (series.current_appointment_number.saturating_sub(1) * MINUTES_PER_DAY);
    }
    sync_current_patient_story_vars(content, &mut next_state);
    *state = next_state;
    Some(current_appointment_intro(state).unwrap_or_else(|| content.opening.intro_text.clone()))
}

pub fn display_actor_name(state: &WorldState, actor: &ActorDefinition) -> String {
    if is_patient_actor(state, &actor.id)
        && let Some(name) = state.story_vars.get(PATIENT_NAME_VAR)
    {
        return name.clone();
    }
    actor.name.clone()
}

pub fn resolved_actor_prompt_context(state: &WorldState, actor: &ActorDefinition) -> ActorPromptContext {
    if !is_patient_actor(state, &actor.id) {
        return actor.prompt_context.clone();
    }
    let Some(patient) = current_patient(state) else {
        return actor.prompt_context.clone();
    };
    if patient.uses_authored_profile {
        return actor.prompt_context.clone();
    }
    let appointment_number = current_appointment_number(state);
    let mut response_notes = actor.prompt_context.response_notes.clone();
    response_notes.push(format!(
        "You are in appointment {appointment_number}. Respond as {} would in therapy, without narrating future sessions.",
        patient.name
    ));
    if let Some(review) = patient.last_feedback_review.as_deref() {
        response_notes.push(format!("Last appointment takeaway: {review}"));
    }
    ActorPromptContext {
        character_notes: vec![
            format!("You are {}, a {}-year-old {}.", patient.name, patient.age, patient.profession),
            format!("Presenting issue: {}.", patient.presenting_issue),
            format!("Relational pattern: {}.", patient.relational_pattern),
            format!("Formative memory: {}.", patient.formative_memory),
            format!("Coping style: {}.", patient.coping_style),
            format!("Desired change: {}.", patient.desired_change),
            format!("Bibliotherapy fit: {}.", patient.bibliotherapy_fit),
        ],
        subtext_notes: {
            let mut notes = vec![
                format!("Carry the emotional residue of {}.", patient.intro_blurb),
                format!("Your tendency under pressure: {}.", patient.coping_style),
            ];
            notes.extend(
                patient
                    .secret_notes
                    .iter()
                    .map(|secret| format!("Hidden truth: {secret}")),
            );
            notes
        },
        response_notes,
        behavior_examples: actor.prompt_context.behavior_examples.clone(),
    }
}

pub fn current_appointment_intro(state: &WorldState) -> Option<String> {
    let patient = current_patient(state)?;
    if patient.uses_authored_profile && patient.appointment_count == 0 {
        return None;
    }
    let appointment_number = current_appointment_number(state);
    let returning = patient.appointment_count > 0;
    Some(if returning {
        let prior_note = patient
            .last_feedback_review
            .as_deref()
            .unwrap_or("they are still sorting through what happened last time");
        format!(
            "Day {appointment_number}.\n\n{} returns for another appointment. {} They arrive carrying the aftertaste of last time: {}",
            patient.name, patient.return_blurb, prior_note
        )
    } else {
        format!(
            "Day {appointment_number}.\n\n{} arrives for a first appointment. {}",
            patient.name, patient.intro_blurb
        )
    })
}

pub fn current_patient_name(state: &WorldState) -> Option<String> {
    current_patient(state).map(|patient| patient.name.clone())
}

pub fn render_dynamic_story_text(template: &str, state: &WorldState) -> String {
    let mut rendered = template.to_string();
    for (key, value) in &state.story_vars {
        rendered = rendered.replace(&format!("{{{key}}}"), value);
    }
    if let (Some(base_name), Some(current_name)) = (
        state.story_vars.get(PATIENT_SLOT_BASE_NAME_VAR),
        state.story_vars.get(PATIENT_NAME_VAR),
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

fn bootstrap_first_appointment(content: &ContentPack, state: &mut WorldState) {
    let patient = build_patient_record(
        content,
        state,
        content
            .appointment_patients
            .first()
            .unwrap_or_else(|| panic!("missing appointment patient definitions")),
    );
    let Some(series) = state.appointment_series.as_mut() else {
        return;
    };
    series.current_appointment_number = 1;
    series.current_patient_id = patient.id.clone();
    series.next_seed_index = 1;
    series.patients.insert(patient.id.clone(), patient);
    sync_current_patient_story_vars(content, state);
}

fn sync_current_patient_story_vars(content: &ContentPack, state: &mut WorldState) {
    let Some(series) = state.appointment_series.as_ref() else {
        return;
    };
    let Some(patient) = series.patients.get(&series.current_patient_id) else {
        return;
    };
    let base_name = content
        .actor(&series.patient_actor_id)
        .map(|actor| actor.name.clone())
        .unwrap_or_else(|| "Patient".to_string());
    state.story_vars.insert(PATIENT_ACTOR_ID_VAR.to_string(), series.patient_actor_id.clone());
    state.story_vars.insert(PATIENT_SLOT_BASE_NAME_VAR.to_string(), base_name);
    state.story_vars.insert(PATIENT_NAME_VAR.to_string(), patient.name.clone());
    state.story_vars.insert("appointment_number".to_string(), series.current_appointment_number.to_string());
    state.story_vars.insert("patient_age".to_string(), patient.age.to_string());
    state.story_vars.insert("patient_profession".to_string(), patient.profession.clone());
    state.story_vars.insert("patient_presenting_issue".to_string(), patient.presenting_issue.clone());
    state.story_vars.insert("patient_relational_pattern".to_string(), patient.relational_pattern.clone());
    state.story_vars.insert("patient_formative_memory".to_string(), patient.formative_memory.clone());
    state.story_vars.insert("patient_coping_style".to_string(), patient.coping_style.clone());
    state.story_vars.insert("patient_desired_change".to_string(), patient.desired_change.clone());
    state.story_vars.insert("patient_bibliotherapy_fit".to_string(), patient.bibliotherapy_fit.clone());
    state.story_vars.insert(
        "patient_returning".to_string(),
        if patient.appointment_count > 0 { "true" } else { "false" }.to_string(),
    );
    if let Some(stats) = state.actor_stats.get_mut(&series.patient_actor_id) {
        *stats = patient.actor_stats.clone();
    }
    if let Some(initial_stats) = state.initial_actor_stats.get_mut(&series.patient_actor_id) {
        *initial_stats = patient.actor_stats.clone();
    }
}

fn current_patient(state: &WorldState) -> Option<&PatientRecord> {
    let series = state.appointment_series.as_ref()?;
    series.patients.get(&series.current_patient_id)
}

fn current_appointment_number(state: &WorldState) -> u32 {
    state
        .appointment_series
        .as_ref()
        .map(|series| series.current_appointment_number)
        .unwrap_or(1)
}

fn is_patient_actor(state: &WorldState, actor_id: &str) -> bool {
    state
        .appointment_series
        .as_ref()
        .is_some_and(|series| series.patient_actor_id == actor_id)
}

fn choose_next_patient_id(content: &ContentPack, series: &AppointmentSeriesState) -> String {
    if series.current_appointment_number >= 3 && series.current_appointment_number % 2 == 1 {
        if let Some((patient_id, _)) = series
            .patients
            .iter()
            .filter(|(patient_id, patient)| {
                **patient_id != series.current_patient_id && !patient.uses_authored_profile
            })
            .min_by_key(|(_, patient)| patient.last_seen_appointment.unwrap_or(0))
        {
            return patient_id.clone();
        }
    }
    if let Some(definition) = content.appointment_patients.get(series.next_seed_index) {
        return definition.id.clone();
    }
    series
        .patients
        .iter()
        .filter(|(patient_id, patient)| {
            **patient_id != series.current_patient_id && !patient.uses_authored_profile
        })
        .min_by_key(|(_, patient)| patient.last_seen_appointment.unwrap_or(0))
        .map(|(patient_id, _)| patient_id.clone())
        .unwrap_or_else(|| series.current_patient_id.clone())
}

fn build_patient_record(
    content: &ContentPack,
    state: &WorldState,
    definition: &AppointmentPatientDefinition,
) -> PatientRecord {
    let mut actor_stats = content
        .actor(&content.settings.appointment_patient_actor_id)
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
    PatientRecord {
        id: definition.id.clone(),
        name: definition.name.clone(),
        age: definition.age,
        profession: definition.profession.clone(),
        presenting_issue: definition.presenting_issue.clone(),
        relational_pattern: definition.relational_pattern.clone(),
        formative_memory: definition.formative_memory.clone(),
        coping_style: definition.coping_style.clone(),
        desired_change: definition.desired_change.clone(),
        bibliotherapy_fit: definition.bibliotherapy_fit.clone(),
        inspect_blurb: definition.inspect_blurb.clone(),
        intro_blurb: definition.intro_blurb.clone(),
        return_blurb: definition.return_blurb.clone(),
        secret_notes: definition.secret_notes.clone(),
        appointment_count: 0,
        last_seen_appointment: None,
        last_feedback_rating: None,
        last_feedback_review: None,
        actor_stats,
        uses_authored_profile: definition.uses_authored_profile,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::loader::load_named_pack;

    #[test]
    fn first_appointment_preserves_authored_patient() {
        let content = load_named_pack("isla", None).expect("load isla");
        let mut state = WorldState::new(&content);
        initialize_appointment_state(&content, &mut state);

        assert_eq!(current_patient_name(&state).as_deref(), Some("Noa"));
        assert_eq!(current_appointment_intro(&state), None);
    }

    #[test]
    fn second_appointment_uses_first_generated_patient() {
        let content = load_named_pack("isla", None).expect("load isla");
        let mut state = WorldState::new(&content);
        initialize_appointment_state(&content, &mut state);

        let _ = advance_to_next_appointment(&content, &mut state, None);

        assert_eq!(current_patient_name(&state).as_deref(), Some("Awa"));
    }
}
