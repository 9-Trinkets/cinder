use cinder_core::content::loader;
use cinder_core::content::types::UiTextDefinition;
use cinder_core::engine::runtime::{ActiveMenuInfo, CinderRuntime, LookOptionItem, MenuChoiceOption};
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
    pub current_room_name: String,
    pub followed_actor_name: Option<String>,
    pub help_text: String,
    pub about_body: String,
    pub current_locale: String,
    pub locale_options: Vec<LocaleItem>,
    pub objectives: Vec<ObjectiveItem>,
    pub objective_message: String,
    pub progress_completed: usize,
    pub progress_total: usize,
    pub rooms: Vec<MenuOptionData>,
    pub follow_options: Vec<MenuOptionData>,
    pub channel_surfing_only: bool,
    pub action_bar_actions: Vec<ActionBarAction>,
    pub overflow_actions: Vec<OverflowAction>,
    pub look_options: Vec<LookOptionData>,
    pub talk_options: Vec<MenuOptionData>,
    pub active_menu: Option<ActiveMenuData>,
    pub ui_text: UiTextDefinition,
}

#[derive(Clone, Serialize)]
pub struct MovieFrameData {
    pub text: String,
    pub duration_ms: u64,
}

#[derive(Clone, Serialize)]
pub struct MovieData {
    pub title: String,
    pub frames: Vec<MovieFrameData>,
    pub narrative_lines: Vec<String>,
}

#[derive(Clone, Serialize)]
pub struct CommandResponse {
    pub text: String,
    pub game_over: bool,
    pub movie: Option<MovieData>,
}

#[derive(Clone, Serialize)]
pub struct ActionBarAction {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Serialize)]
pub struct OverflowAction {
    pub id: String,
    pub label: String,
    pub group: String,
    pub usage: String,
}

#[derive(Clone, Serialize)]
pub struct LookOptionData {
    pub id: String,
    pub title: String,
    pub command: String,
}

#[derive(Clone, Serialize)]
pub struct MenuOptionData {
    pub id: String,
    pub title: String,
    pub menu_text: String,
}

#[derive(Clone, Serialize)]
pub struct ActiveMenuData {
    pub prompt: String,
    pub options: Vec<MenuOptionData>,
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
) -> Result<CommandResponse, String> {
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
        let turn_text = outcome.as_ref().ok().map(|o| o.text.clone());
        let outcome = match outcome {
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
        if let Some(ref text) = turn_text {
            let _ = session.runtime.push_transcript_line(text);
        }
        (outcome, session)
    })
    .await
    .map_err(|e| format!("blocking task failed: {e}"))?;

    let movie = result.as_ref().ok().and_then(|_| {
        consume_projector_sequence(&session.runtime)
    });

    {
        let mut guard = sessions.lock().map_err(|_| "lock poisoned".to_string())?;
        guard.insert(session_id.to_string(), session);
    }

    result.map(|outcome| CommandResponse {
        text: outcome.text,
        game_over: outcome.game_over,
        movie,
    })
}

pub fn consume_projector_sequence(runtime: &CinderRuntime) -> Option<MovieData> {
    let raw = runtime.consume_pending_projector_sequence();
    eprintln!("[web] consume_pending_projector_sequence: raw={:?}", raw.as_ref().err().map(|e| e.to_string()));
    let sequence = raw.ok()??;
    let narrative_lines = runtime
        .consume_pending_projector_narrative_lines()
        .ok()
        .unwrap_or_default();
    eprintln!("[web] frames count: {}, narrative lines: {}", sequence.frames.len(), narrative_lines.len());
    let frames = sequence
        .frames
        .into_iter()
        .map(|f| MovieFrameData {
            text: f.text,
            duration_ms: f.duration_ms,
        })
        .collect();
    Some(MovieData {
        title: sequence.title,
        frames,
        narrative_lines,
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
        let objectives: Vec<ObjectiveItem> = session
            .runtime
            .current_objective_summaries()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|(s, m)| ObjectiveItem {
                summary: s,
                message: m,
            })
            .collect();
        let (progress_completed, progress_total) = session
            .runtime
            .current_objective_progress()
            .map_err(|e| e.to_string())?;
        let objective_message = objectives
            .first()
            .map(|o| o.message.clone())
            .unwrap_or_default();
        let locales = loader::available_locales(&loader::pack_dir(&session.pack_id))
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|l| LocaleItem {
                code: l.code,
                label: l.label,
            })
            .collect();
        let content = session.runtime.content();

        let current_room_id = session.runtime.current_room_id().map_err(|e| e.to_string())?;
        let current_room_name = content
            .room(&current_room_id)
            .map(|r| r.title.clone())
            .unwrap_or(current_room_id);
        let followed_actor_name = session
            .runtime
            .followed_actor_id()
            .map_err(|e| e.to_string())?
            .and_then(|id| content.actor(&id).map(|a| a.name.clone()));

        let (action_bar_actions, content_defined_bar) =
            if !content.ui_text.action_bar.actions.is_empty() {
                (
                    content
                        .ui_text
                        .action_bar
                        .actions
                        .iter()
                        .map(|a| ActionBarAction {
                            id: a.id.clone(),
                            label: a.label.clone(),
                        })
                        .collect(),
                    true,
                )
            } else {
                (
                    vec![
                        ActionBarAction {
                            id: "look".into(),
                            label: "Look".into(),
                        },
                        ActionBarAction {
                            id: "move".into(),
                            label: "Move".into(),
                        },
                        ActionBarAction {
                            id: "follow".into(),
                            label: "Follow".into(),
                        },
                    ],
                    false,
                )
            };

        let look_options: Vec<LookOptionData> = session
            .runtime
            .current_room_look_options()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|o: LookOptionItem| LookOptionData {
                id: o.id,
                title: o.label,
                command: o.command,
            })
            .collect();

        let talk_options: Vec<MenuOptionData> = session
            .runtime
            .current_room_talk_options()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|o: LookOptionItem| MenuOptionData {
                id: o.id,
                title: o.label.clone(),
                menu_text: o.label,
            })
            .collect();

        let active_menu: Option<ActiveMenuData> = session
            .runtime
            .current_active_menu_info()
            .map_err(|e| e.to_string())?
            .map(|info: ActiveMenuInfo| ActiveMenuData {
                prompt: info.prompt,
                options: info
                    .options
                    .into_iter()
                    .map(|o| MenuOptionData {
                        id: o.id,
                        title: o.title,
                        menu_text: o.menu_text,
                    })
                    .collect(),
            });

        let mut action_bar_actions = action_bar_actions;
        if !content_defined_bar
            && !talk_options.is_empty()
            && !action_bar_actions.iter().any(|a| a.id == "speak" || a.id == "talk")
        {
            action_bar_actions.push(ActionBarAction {
                id: "talk".into(),
                label: "Talk".into(),
            });
        }

        let bar_ids: Vec<&str> = action_bar_actions.iter().map(|a| a.id.as_str()).collect();
        let has_talk = bar_ids.contains(&"speak") || bar_ids.contains(&"talk");
        let modal_covered: Vec<&str> = vec!["inspect_feature", "inspect_actor"];
        let overflow_actions: Vec<OverflowAction> = content
            .commands
            .actions
            .iter()
            .filter(|c| {
                if !c.player_enabled || bar_ids.contains(&c.id.as_str()) {
                    return false;
                }
                if modal_covered.contains(&c.id.as_str()) {
                    return false;
                }
                if (c.id == "speak" || c.id == "talk") && has_talk {
                    return false;
                }
                true
            })
            .map(|c| {
                let label = c
                    .id
                    .split('_')
                    .map(|w| {
                        let mut chars = w.chars();
                        chars
                            .next()
                            .map(|f| f.to_uppercase().to_string() + chars.as_str())
                            .unwrap_or_default()
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                let usage = c
                    .player_command
                    .as_ref()
                    .map(|p| p.usage.clone())
                    .unwrap_or_default();
                OverflowAction {
                    id: c.id.clone(),
                    label,
                    group: c.group.clone(),
                    usage,
                }
            })
            .collect();

        Ok(UiSnapshot {
            title: content.opening.title.clone(),
            time_label,
            day_number,
            current_room_name,
            followed_actor_name,
            help_text: session.runtime.help_text(),
            about_body: content.ui_text.about_body.clone(),
            current_locale: content.locale.clone(),
            locale_options: locales,
            objectives,
            objective_message,
            progress_completed,
            progress_total,
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
            action_bar_actions,
            overflow_actions,
            look_options,
            talk_options,
            active_menu,
            ui_text: content.ui_text.clone(),
        })
    })
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
