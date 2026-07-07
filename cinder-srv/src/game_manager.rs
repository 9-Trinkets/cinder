use cinder_core::content::loader;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{TurnOutcome, WorldState};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

mod response;
mod ui;

pub use self::response::{CommandResponse, SessionFeedbackData, consume_projector_sequence};
pub use self::ui::UiSnapshot;

#[derive(Clone)]
#[allow(dead_code)]
pub struct ActiveSession {
    pub runtime: CinderRuntime,
    pub pack_id: String,
    pub session_id: String,
    pub player_id: String,
}

pub type SessionMap = Arc<Mutex<HashMap<String, ActiveSession>>>;

pub fn new_session_map() -> SessionMap {
    Arc::new(Mutex::new(HashMap::new()))
}

pub async fn ensure_session(
    sessions: &SessionMap,
    pool: &PgPool,
    session_id: &str,
    player_id: &str,
) -> Result<(), String> {
    {
        let guard = sessions.lock().map_err(|_| "lock poisoned".to_string())?;
        if guard.contains_key(session_id) {
            return Ok(());
        }
    }
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT pack_id, state_json::text FROM game_sessions WHERE id = $1 AND player_id = $2",
    )
    .bind(session_id)
    .bind(player_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("db query error: {e}"))?
    .ok_or_else(|| "session not found".to_string())?;
    let (pack_id, state_json) = row;
    let content = loader::load_named_pack(&pack_id, None)
        .map_err(|e| format!("failed to load pack '{pack_id}': {e}"))?;
    let runtime = if state_json.is_empty() || state_json == "{}" {
        CinderRuntime::new(content, false).map_err(|e| format!("failed to create runtime: {e}"))?
    } else {
        let state: WorldState = serde_json::from_str(&state_json)
            .map_err(|e| format!("failed to deserialize saved state: {e}"))?;
        CinderRuntime::from_state(content, state, false)
            .map_err(|e| format!("failed to create runtime from state: {e}"))?
    };
    let session = ActiveSession {
        runtime,
        pack_id,
        session_id: session_id.to_string(),
        player_id: player_id.to_string(),
    };
    sessions
        .lock()
        .map_err(|_| "lock poisoned".to_string())?
        .insert(session_id.to_string(), session);
    Ok(())
}

fn with_session<F, T>(sessions: &SessionMap, session_id: &str, f: F) -> Result<T, String>
where
    F: FnOnce(&mut ActiveSession) -> Result<T, String>,
{
    let mut guard = sessions.lock().map_err(|_| "lock poisoned".to_string())?;
    let session = guard
        .get_mut(session_id)
        .ok_or_else(|| "session not found".to_string())?;
    f(session)
}

pub fn create_session(
    sessions: &SessionMap,
    player_id: &str,
    pack_id: &str,
    state_json: Option<&str>,
) -> Result<(String, ActiveSession), String> {
    let content = loader::load_named_pack(pack_id, None)
        .map_err(|e| format!("failed to load pack '{pack_id}': {e}"))?;

    let runtime = match state_json {
        Some(json) => {
            let state: WorldState = serde_json::from_str(json)
                .map_err(|e| format!("failed to deserialize saved state: {e}"))?;
            CinderRuntime::from_state(content, state, false)
                .map_err(|e| format!("failed to create runtime from state: {e}"))?
        }
        None => CinderRuntime::new(content, false)
            .map_err(|e| format!("failed to create runtime: {e}"))?,
    };

    let session_id = Uuid::new_v4().to_string();
    let session = ActiveSession {
        runtime,
        pack_id: pack_id.to_string(),
        session_id: session_id.clone(),
        player_id: player_id.to_string(),
    };

    sessions
        .lock()
        .map_err(|_| "lock poisoned".to_string())?
        .insert(session_id.clone(), session.clone());

    Ok((session_id, session))
}

pub async fn run_command(
    sessions: &SessionMap,
    session_id: &str,
    input: &str,
) -> Result<CommandResponse, String> {
    let session = {
        let mut guard = sessions.lock().map_err(|_| "lock poisoned".to_string())?;
        guard
            .remove(session_id)
            .ok_or_else(|| "session not found".to_string())?
    };

    let input = input.to_string();
    let (session, result, session_feedback) = tokio::task::spawn_blocking(move || {
        let session = session;
        let task_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut outcome = session
                .runtime
                .run_turn(&input)
                .map_err(|e| format!("turn error: {e}"));
            let turn_text = outcome.as_ref().ok().map(|o| o.text.clone());
            outcome = match outcome {
                Ok(turn) if !turn.game_over => {
                    match session.runtime.run_tick() {
                        Ok(tick) => {
                            let combined = if tick.text.is_empty() {
                                turn.text
                            } else {
                                format!("{}\n\n{}", turn.text, tick.text)
                            };
                            Ok(TurnOutcome {
                                text: combined,
                                game_over: turn.game_over || tick.game_over,
                            })
                        }
                        Err(e) => Err(format!("tick error: {e}")),
                    }
                }
                other => other,
            };
            let session_feedback = outcome.as_ref().ok().and_then(|outcome| {
                if outcome.game_over {
                    response::session_feedback_data(&session.runtime)
                } else {
                    None
                }
            });
            outcome = outcome.and_then(|outcome| {
                session
                    .runtime
                    .continue_after_game_over(outcome)
                    .map_err(|error| format!("appointment rollover error: {error}"))
            });
            if let Some(ref text) = turn_text {
                let _ = session.runtime.push_transcript_line(text);
            }
            (outcome, session_feedback)
        }));
        let (result, session_feedback) = match task_result {
            Ok((outcome, session_feedback)) => (outcome, session_feedback),
            Err(payload) => (
                Err(format!(
                    "command panicked: {}",
                    response::panic_payload_message(&payload)
                )),
                None,
            ),
        };
        (session, result, session_feedback)
    })
    .await
    .map_err(|e| format!("blocking task failed: {e}"))?;

    let movie = result
        .as_ref()
        .ok()
        .and_then(|_| consume_projector_sequence(&session.runtime));

    {
        let mut guard = sessions.lock().map_err(|_| "lock poisoned".to_string())?;
        guard.insert(session_id.to_string(), session);
    }

    result.map(|outcome| CommandResponse {
        text: outcome.text,
        game_over: outcome.game_over,
        movie,
        session_feedback,
    })
}

pub fn switch_room(
    sessions: &SessionMap,
    session_id: &str,
    room_id: &str,
) -> Result<TurnOutcome, String> {
    let mut guard = sessions.lock().map_err(|_| "lock poisoned".to_string())?;
    let session = guard
        .get_mut(session_id)
        .ok_or_else(|| "session not found".to_string())?;
    let outcome = session
        .runtime
        .switch_room_view(room_id)
        .map_err(|e| format!("room switch error: {e}"))?;
    let _ = session.runtime.push_transcript_line(&outcome.text);
    Ok(outcome)
}

pub fn follow_actor(
    sessions: &SessionMap,
    session_id: &str,
    actor_id: Option<&str>,
) -> Result<TurnOutcome, String> {
    let mut guard = sessions.lock().map_err(|_| "lock poisoned".to_string())?;
    let session = guard
        .get_mut(session_id)
        .ok_or_else(|| "session not found".to_string())?;
    let outcome = session
        .runtime
        .follow_actor(actor_id)
        .map_err(|e| format!("follow error: {e}"))?;
    let _ = session.runtime.push_transcript_line(&outcome.text);
    Ok(outcome)
}

pub fn set_locale(
    sessions: &SessionMap,
    session_id: &str,
    locale: &str,
) -> Result<String, String> {
    let mut guard = sessions.lock().map_err(|_| "lock poisoned".to_string())?;
    let session = guard
        .get_mut(session_id)
        .ok_or_else(|| "session not found".to_string())?;
    let pack = loader::load_pack_from_dir_with_locale(
        &loader::pack_dir(&session.pack_id),
        Some(locale),
    )
    .map_err(|e| format!("failed to load locale '{locale}': {e}"))?;
    let language_name = pack.ui_text.language_name.clone();
    let runtime = session.runtime.with_content(pack);
    runtime
        .relocalize_story_vars()
        .map_err(|e| format!("relocalize error: {e}"))?;
    let ui_text = runtime.content().ui_text.clone();
    let changed_text = runtime.content().render_template(
        &ui_text.language_changed_text,
        &[("language_name", language_name.as_str())],
    );
    session.runtime = runtime;
    Ok(changed_text)
}

pub fn get_session_ui(sessions: &SessionMap, session_id: &str) -> Result<UiSnapshot, String> {
    with_session(sessions, session_id, |session| ui::build_ui_snapshot(session))
}

pub fn get_transcript(sessions: &SessionMap, session_id: &str) -> Result<Vec<String>, String> {
    with_session(sessions, session_id, |session| {
        session
            .runtime
            .transcript_lines()
            .map_err(|e| e.to_string())
    })
}

pub fn get_runtime(sessions: &SessionMap, session_id: &str) -> Result<CinderRuntime, String> {
    let guard = sessions.lock().map_err(|_| "lock poisoned".to_string())?;
    let session = guard
        .get(session_id)
        .ok_or_else(|| "session not found".to_string())?;
    Ok(session.runtime.clone())
}

pub fn delete_session(sessions: &SessionMap, session_id: &str) -> Result<(), String> {
    let mut guard = sessions.lock().map_err(|_| "lock poisoned".to_string())?;
    guard
        .remove(session_id)
        .ok_or_else(|| "session not found".to_string())?;
    Ok(())
}

pub fn export_session_state(sessions: &SessionMap, session_id: &str) -> Result<String, String> {
    let guard = sessions.lock().map_err(|_| "lock poisoned".to_string())?;
    let session = guard
        .get(session_id)
        .ok_or_else(|| "session not found".to_string())?;
    let state = session
        .runtime
        .export_state()
        .map_err(|e| format!("state export error: {e}"))?;
    serde_json::to_string(&state).map_err(|e| format!("serialization error: {e}"))
}
