use cinder_core::content::loader;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{TurnOutcome, WorldState};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

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

pub fn run_command(
    sessions: &SessionMap,
    session_id: &str,
    input: &str,
) -> Result<TurnOutcome, String> {
    let mut guard = sessions.lock().map_err(|_| "lock poisoned".to_string())?;
    let session = guard
        .get_mut(session_id)
        .ok_or_else(|| "session not found".to_string())?;

    let outcome = session
        .runtime
        .run_turn(input)
        .map_err(|e| format!("turn error: {e}"))?;

    if !outcome.game_over {
        let tick_outcome = session
            .runtime
            .run_tick()
            .map_err(|e| format!("tick error: {e}"))?;
        let combined_text = if tick_outcome.text.is_empty() {
            outcome.text
        } else {
            format!("{}\n\n{}", outcome.text, tick_outcome.text)
        };
        Ok(TurnOutcome {
            text: combined_text,
            game_over: outcome.game_over || tick_outcome.game_over,
        })
    } else {
        Ok(outcome)
    }
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
