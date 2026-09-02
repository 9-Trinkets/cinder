use super::CinderRuntime;
use crate::content::types::ContentPack;
use crate::engine::dialogue::{ScriptedDialogueGenerator, StageAssignment, StageAssignmentScore};
use crate::engine::neuron::load_workflow;
use crate::engine::state::{TurnOutcome, WorldState};
use crate::engine::test_fixtures::{TestDir, load_test_pack_with_files};
use crate::engine::workflows::{cinder_npc_tick_workflow_path, workflow_path_for_id};
use std::sync::Arc;

pub(super) fn stage_assignment_test_pack() -> ContentPack {
    load_test_pack_with_files(&[
        ("locales/en/rooms.json", STAGE_ASSIGNMENT_ROOMS_JSON),
        ("locales/en/actors.json", STAGE_ASSIGNMENT_ACTORS_JSON),
        ("locales/en/beats.json", STAGE_ASSIGNMENT_BEATS_JSON),
    ])
}

pub(super) fn stage_assignment_runtime(
    content: ContentPack,
    state: WorldState,
    stage_id: &str,
    assignment: StageAssignment,
) -> (CinderRuntime, TestDir) {
    let dialogue =
        Arc::new(ScriptedDialogueGenerator::new().with_stage_assignment(stage_id, assignment));
    let trace_dir = TestDir::new("runtime-traces");
    let runtime = CinderRuntime::new_with_dialogue_generator_and_workflows(
        content,
        state,
        false,
        dialogue,
        load_workflow(&workflow_path_for_id("cinder_turn")).expect("load turn workflow"),
        load_workflow(&cinder_npc_tick_workflow_path()).expect("load npc tick workflow"),
        trace_dir.path().to_path_buf(),
    )
    .expect("build runtime");
    (runtime, trace_dir)
}

pub(super) fn dinner_prep_assignment() -> StageAssignment {
    StageAssignment {
        assignments: vec![
            StageAssignmentScore {
                actor_id: "blair".to_string(),
                selection_score: 90,
                rationale: "already leaning toward Alex".to_string(),
            },
            StageAssignmentScore {
                actor_id: "casey".to_string(),
                selection_score: 82,
                rationale: "likes the energy in the kitchen".to_string(),
            },
            StageAssignmentScore {
                actor_id: "devon".to_string(),
                selection_score: 15,
                rationale: "hangs back in the lounge".to_string(),
            },
        ],
    }
}

pub(super) fn activity_split_state(content: &ContentPack) -> WorldState {
    let mut state = WorldState::new(content);
    state.active_objective_stage_ids = vec!["activity-split".to_string()];
    state.story_vars.set_unchecked("activity_room_a", "patio");
    state.story_vars.set_unchecked("activity_room_b", "studio");
    state.story_vars.set_unchecked("activity_host_a", "devon");
    state.story_vars.set_unchecked("activity_host_b", "alex");
    state
}

pub(super) fn activity_split_assignment() -> StageAssignment {
    StageAssignment {
        assignments: vec![
            StageAssignmentScore {
                actor_id: "casey".to_string(),
                selection_score: 90,
                rationale: "joins the patio activity".to_string(),
            },
            StageAssignmentScore {
                actor_id: "blair".to_string(),
                selection_score: 20,
                rationale: "stays with the studio activity".to_string(),
            },
        ],
    }
}

pub(super) fn assert_dinner_prep_assignment(outcome: &TurnOutcome, state: &WorldState) {
    assert!(
        outcome
            .text
            .contains("Alex starts pulling the house toward the Kitchen for dinner prep.")
    );
    assert_eq!(state.story_vars.get("alex_assigned_room"), Some("kitchen"));
    assert_eq!(state.story_vars.get("blair_assigned_room"), Some("kitchen"));
    assert_eq!(state.story_vars.get("casey_assigned_room"), Some("kitchen"));
    assert_eq!(state.story_vars.get("devon_assigned_room"), Some("lounge"));
    assert_eq!(
        state.story_vars.get("stage_assignment_applied:dinner-prep"),
        Some("true")
    );
    assert!(state.actor_room_overrides.is_empty());
}

pub(super) fn assert_activity_split_assignment(state: &WorldState) {
    assert_eq!(
        state.story_vars.get("activity_group_a"),
        Some("casey,devon"),
        "selected group must include the anchored host"
    );
    assert_eq!(
        state.story_vars.get("activity_group_b"),
        Some("alex,blair"),
        "remaining group must include the anchored host"
    );
    assert_eq!(state.story_vars.get("devon_assigned_room"), Some("patio"));
    assert_eq!(state.story_vars.get("alex_assigned_room"), Some("studio"));
    assert!(state.actor_room_overrides.is_empty());
}

const STAGE_ASSIGNMENT_ROOMS_JSON: &str = r#"[
  {
    "id": "lounge",
    "title": "Lounge",
    "summary": "A shared lounge.",
    "inspect_text": "A shared lounge.",
    "features": [],
    "exits": []
  },
  {
    "id": "kitchen",
    "title": "Kitchen",
    "summary": "A warm kitchen.",
    "inspect_text": "A warm kitchen.",
    "features": [],
    "exits": []
  },
  {
    "id": "patio",
    "title": "Patio",
    "summary": "A quiet patio.",
    "inspect_text": "A quiet patio.",
    "features": [],
    "exits": []
  },
  {
    "id": "studio",
    "title": "Studio",
    "summary": "A focused studio.",
    "inspect_text": "A focused studio.",
    "features": [],
    "exits": []
  }
]"#;

const STAGE_ASSIGNMENT_ACTORS_JSON: &str = r#"[
  {
    "id": "alex",
    "name": "Alex",
    "room_id": "lounge",
    "initial_stats": { "confidence": 5, "stamina": 8, "hunger": 3 },
    "prompt_context": {}
  },
  {
    "id": "blair",
    "name": "Blair",
    "room_id": "lounge",
    "initial_stats": { "confidence": 3, "stamina": 6, "hunger": 5 },
    "initial_pair_stats": { "alex": { "connection": 4, "attraction": 2, "safety": 3 } },
    "prompt_context": {}
  },
  {
    "id": "casey",
    "name": "Casey",
    "room_id": "lounge",
    "initial_stats": { "confidence": 7, "stamina": 7, "hunger": 4 },
    "initial_pair_stats": { "alex": { "connection": 2, "attraction": 3, "safety": 1 } },
    "prompt_context": {}
  },
  {
    "id": "devon",
    "name": "Devon",
    "room_id": "lounge",
    "initial_stats": { "confidence": 1, "stamina": 9, "hunger": 2 },
    "initial_pair_stats": { "alex": { "connection": 1, "attraction": 0, "safety": 2 } },
    "prompt_context": {}
  }
]"#;

const STAGE_ASSIGNMENT_BEATS_JSON: &str = r#"{
  "initial_stage_ids": ["dinner-prep"],
  "stages": [
    {
      "id": "dinner-prep",
      "summary": "Prep",
      "update_message": "Prep starts.",
      "beat_note": "Split the house.",
      "stage_assignment": {
        "selection_label": "dinner prep",
        "prompt_instructions": "Prefer the kitchen when someone would want to be near Alex through useful, practical closeness.",
        "initiator_actor_id": "alex",
        "selected_room_id": "kitchen",
        "remaining_room_id": "lounge",
        "max_selected_actors": 2,
        "min_selected_actors": 1,
        "score_threshold": 50,
        "initiator_line_template": "{initiator_name} starts pulling the house toward the {selected_room_title} for {selection_label}.",
        "selected_line_template": "{selected_names} fall in with the plan.",
        "remaining_line_template": "{remaining_names} lag a step behind for the moment."
      },
      "next_stage_ids": ["dinner"]
    },
    {
      "id": "dinner",
      "summary": "Dinner",
      "update_message": "Dinner starts."
    },
    {
      "id": "activity-split",
      "summary": "Split",
      "update_message": "Split starts.",
      "beat_note": "Split the house into two activities.",
      "stage_assignment": {
        "selection_label": "activity split",
        "prompt_instructions": "Divide the cast between two activity rooms.",
        "selected_room_id": "patio",
        "remaining_room_id": "studio",
        "selected_room_story_var": "activity_room_a",
        "remaining_room_story_var": "activity_room_b",
        "selected_host_story_var": "activity_host_a",
        "remaining_host_story_var": "activity_host_b",
        "max_selected_actors": 1,
        "min_selected_actors": 1,
        "score_threshold": 0,
        "group_story_var_key": "activity_group_a",
        "remaining_group_story_var_key": "activity_group_b"
      },
      "next_stage_ids": ["after-split"]
    },
    {
      "id": "after-split",
      "summary": "After split",
      "update_message": "After split."
    }
  ]
}"#;
