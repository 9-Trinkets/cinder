use cinder_core::content::loader;
use cinder_core::content::types::UiTextDefinition;
use cinder_core::engine::runtime::{CinderRuntime, MenuChoiceOption};
use cinder_core::engine::state::{TurnOutcome, WorldState};
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone, Serialize)]
pub struct LocaleItem {
    pub code: String,
    pub label: String,
}

#[derive(Clone, Serialize)]
pub struct ObjectiveItem {
    pub summary: String,
    pub message: String,
}

#[derive(Clone, Serialize)]
pub struct UiSnapshot {
    pub title: String,
    pub time_label: String,
    pub day_number: u32,
    pub help_text: String,
    pub about_body: String,
    pub current_locale: String,
    pub locale_options: Vec<LocaleItem>,
    pub objectives: Vec<ObjectiveItem>,
    pub rooms: Vec<MenuOptionData>,
    pub follow_options: Vec<MenuOptionData>,
    pub channel_surfing_only: bool,
    pub ui_text: UiTextDefinition,
}

#[derive(Clone, Serialize)]
pub struct MenuOptionData {
    pub id: String,
    pub title: String,
    pub menu_text: String,
}

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
    pool: &SqlitePool,
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
        "SELECT pack_id, state_json FROM game_sessions WHERE id = ? AND player_id = ?",
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
) -> Result<TurnOutcome, String> {
    let session = {
        let mut guard = sessions.lock().map_err(|_| "lock poisoned".to_string())?;
        guard
            .remove(session_id)
            .ok_or_else(|| "session not found".to_string())?
    };

    let input = input.to_string();
    let (result, session) = tokio::task::spawn_blocking(move || {
        let outcome = session
            .runtime
            .run_turn(&input)
            .map_err(|e| format!("turn error: {e}"));
        let outcome = match outcome {
            Ok(outcome) if !outcome.game_over => {
                match session.runtime.run_tick() {
                    Ok(tick_outcome) => {
                        let combined = if tick_outcome.text.is_empty() {
                            outcome.text
                        } else {
                            format!("{}\n\n{}", outcome.text, tick_outcome.text)
                        };
                        Ok(TurnOutcome {
                            text: combined,
                            game_over: outcome.game_over || tick_outcome.game_over,
                        })
                    }
                    Err(e) => Err(format!("tick error: {e}")),
                }
            }
            other => other,
        };
        (outcome, session)
    })
    .await
    .map_err(|e| format!("blocking task failed: {e}"))?;

    {
        let mut guard = sessions.lock().map_err(|_| "lock poisoned".to_string())?;
        guard.insert(session_id.to_string(), session);
    }
    result
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
    session
        .runtime
        .switch_room_view(room_id)
        .map_err(|e| format!("room switch error: {e}"))
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
    session
        .runtime
        .follow_actor(actor_id)
        .map_err(|e| format!("follow error: {e}"))
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

fn menu_option_data(opts: Vec<MenuChoiceOption>) -> Vec<MenuOptionData> {
    opts.into_iter()
        .map(|o| MenuOptionData {
            id: o.command,
            title: o.title,
            menu_text: o.menu_text,
        })
        .collect()
}

pub fn get_session_ui(sessions: &SessionMap, session_id: &str) -> Result<UiSnapshot, String> {
    with_session(sessions, session_id, |session| {
        let time_label = session
            .runtime
            .current_time_label()
            .map_err(|e| e.to_string())?;
        let day_number = session
            .runtime
            .current_day_number()
            .map_err(|e| e.to_string())?;
        let objectives = session
            .runtime
            .current_objective_summaries()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|(s, m)| ObjectiveItem {
                summary: s,
                message: m,
            })
            .collect();
        let locales = loader::available_locales(&loader::pack_dir(&session.pack_id))
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|l| LocaleItem {
                code: l.code,
                label: l.label,
            })
            .collect();
        let content = session.runtime.content();

        Ok(UiSnapshot {
            title: content.opening.title.clone(),
            time_label,
            day_number,
            help_text: session.runtime.help_text(),
            about_body: content.ui_text.about_body.clone(),
            current_locale: content.locale.clone(),
            locale_options: locales,
            objectives,
            rooms: menu_option_data(
                session
                    .runtime
                    .room_switch_options()
                    .map_err(|e| e.to_string())?,
            ),
            follow_options: menu_option_data(
                session
                    .runtime
                    .follow_actor_options()
                    .map_err(|e| e.to_string())?,
            ),
            channel_surfing_only: content.settings.channel_surfing_only,
            ui_text: content.ui_text.clone(),
        })
    })
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
