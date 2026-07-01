use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    middleware,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::auth::{auth_middleware, validate_token, AuthPlayer};
use crate::game_manager;

use super::AppState;

fn internal<E: ToString>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

#[derive(Serialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub pack_id: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub intro_text: String,
}

#[derive(Deserialize)]
pub struct CreateSessionRequest {
    pub pack_id: String,
}

pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/games", get(list_sessions).post(create_session))
        .route("/api/games/{id}/command", post(run_command))
        .route("/api/games/{id}/ui", get(session_ui))
        .route("/api/games/{id}/transcript", get(transcript_handler))
        .route("/api/games/{id}/room", post(switch_room_handler))
        .route("/api/games/{id}/follow", post(follow_actor_handler))
        .route("/api/games/{id}/locale", post(set_locale_handler))
        .route("/api/games/{id}", delete(delete_session_handler))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .route("/api/games/{id}/stream", get(stream_handler))
}

pub async fn create_session(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<SessionInfo>, (StatusCode, String)> {
    let (session_id, session) = game_manager::create_session(&state.sessions, &auth.id, &req.pack_id, None)
        .map_err(internal)?;

    sqlx::query(
        "INSERT INTO game_sessions (id, player_id, pack_id, state_json) VALUES (?, ?, ?, ?)",
    )
    .bind(&session_id)
    .bind(&auth.id)
    .bind(&req.pack_id)
    .bind("{}")
    .execute(&*state.pool)
    .await
    .map_err(internal)?;

    let title = session.runtime.content().opening.title.clone();
    let intro_text = session.runtime.content().opening.intro_text.clone();

    Ok(Json(SessionInfo {
        session_id,
        pack_id: req.pack_id,
        created_at: now_iso(),
        updated_at: now_iso(),
        title,
        intro_text,
    }))
}

pub async fn list_sessions(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
) -> Result<Json<Vec<SessionInfo>>, (StatusCode, String)> {
    let rows = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT id, pack_id, created_at, updated_at FROM game_sessions WHERE player_id = ? ORDER BY updated_at DESC",
    )
    .bind(&auth.id)
    .fetch_all(&*state.pool)
    .await
    .map_err(internal)?;

    Ok(Json(
        rows.into_iter()
            .map(|(id, pack_id, created_at, updated_at)| SessionInfo {
                session_id: id,
                pack_id,
                created_at,
                updated_at,
                title: String::new(),
                intro_text: String::new(),
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct CommandRequest {
    pub input: String,
}

pub async fn run_command(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(session_id): Path<String>,
    Json(req): Json<CommandRequest>,
) -> Result<Json<game_manager::CommandResponse>, (StatusCode, String)> {
    game_manager::ensure_session(&state.sessions, &state.pool, &session_id, &auth.id)
        .await
        .map_err(internal)?;
    let resp = game_manager::run_command(&state.sessions, &session_id, &req.input)
        .await
        .map_err(internal)?;
    Ok(Json(resp))
}

pub async fn transcript_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(session_id): Path<String>,
) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    game_manager::ensure_session(&state.sessions, &state.pool, &session_id, &auth.id)
        .await
        .map_err(internal)?;
    let lines = game_manager::get_transcript(&state.sessions, &session_id).map_err(internal)?;
    Ok(Json(lines))
}

pub async fn session_ui(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(session_id): Path<String>,
) -> Result<Json<game_manager::UiSnapshot>, (StatusCode, String)> {
    game_manager::ensure_session(&state.sessions, &state.pool, &session_id, &auth.id)
        .await
        .map_err(internal)?;
    let snapshot = game_manager::get_session_ui(&state.sessions, &session_id).map_err(internal)?;
    Ok(Json(snapshot))
}

#[derive(Deserialize)]
pub struct RoomSwitchRequest {
    pub room_id: String,
}

pub async fn switch_room_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(session_id): Path<String>,
    Json(req): Json<RoomSwitchRequest>,
) -> Result<Json<game_manager::CommandResponse>, (StatusCode, String)> {
    game_manager::ensure_session(&state.sessions, &state.pool, &session_id, &auth.id)
        .await
        .map_err(internal)?;
    let outcome = game_manager::switch_room(&state.sessions, &session_id, &req.room_id)
        .map_err(internal)?;
    Ok(Json(game_manager::CommandResponse {
        text: outcome.text,
        game_over: outcome.game_over,
        movie: None,
    }))
}

#[derive(Deserialize)]
pub struct FollowRequest {
    pub actor_id: Option<String>,
}

pub async fn follow_actor_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(session_id): Path<String>,
    Json(req): Json<FollowRequest>,
) -> Result<Json<game_manager::CommandResponse>, (StatusCode, String)> {
    game_manager::ensure_session(&state.sessions, &state.pool, &session_id, &auth.id)
        .await
        .map_err(internal)?;
    let outcome =
        game_manager::follow_actor(&state.sessions, &session_id, req.actor_id.as_deref())
            .map_err(internal)?;
    Ok(Json(game_manager::CommandResponse {
        text: outcome.text,
        game_over: outcome.game_over,
        movie: None,
    }))
}

#[derive(Deserialize)]
pub struct LocaleRequest {
    pub locale: String,
}

pub async fn set_locale_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(session_id): Path<String>,
    Json(req): Json<LocaleRequest>,
) -> Result<Json<String>, (StatusCode, String)> {
    game_manager::ensure_session(&state.sessions, &state.pool, &session_id, &auth.id)
        .await
        .map_err(internal)?;
    let text = game_manager::set_locale(&state.sessions, &session_id, &req.locale)
        .map_err(internal)?;
    Ok(Json(text))
}

pub async fn delete_session_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(session_id): Path<String>,
) -> Result<Json<()>, (StatusCode, String)> {
    game_manager::ensure_session(&state.sessions, &state.pool, &session_id, &auth.id)
        .await
        .map_err(internal)?;
    game_manager::delete_session(&state.sessions, &session_id).map_err(internal)?;
    sqlx::query("DELETE FROM game_sessions WHERE id = ? AND player_id = ?")
        .bind(&session_id)
        .bind(&auth.id)
        .execute(&*state.pool)
        .await
        .map_err(internal)?;
    Ok(Json(()))
}

fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("epoch={d}")
}

#[derive(Deserialize)]
pub struct StreamQuery {
    pub token: String,
}

pub async fn stream_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Query(query): Query<StreamQuery>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, String)> {
    let claims = validate_token(&query.token, state.config.jwt_secret.as_bytes())
        .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid token".to_string()))?;
    Ok(ws.on_upgrade(move |socket| {
        handle_ws(socket, state, AuthPlayer { id: claims.sub }, session_id)
    }))
}

async fn handle_ws(
    mut ws: WebSocket,
    state: Arc<AppState>,
    auth: AuthPlayer,
    session_id: String,
) {
    if let Err(e) = game_manager::ensure_session(&state.sessions, &state.pool, &session_id, &auth.id).await
    {
        let _ = ws.send(Message::Text(format!(r#"{{"type":"error","text":"{e}"}}"#).into())).await;
        return;
    }

    let runtime = match game_manager::get_runtime(&state.sessions, &session_id) {
        Ok(r) => r,
        Err(e) => {
            let _ = ws.send(Message::Text(format!(r#"{{"type":"error","text":"{e}"}}"#).into())).await;
            return;
        }
    };

    let typewriter_char_ms = runtime.content().settings.typewriter_char_ms;
    let npc_tick_interval_ms = runtime.content().settings.npc_tick_interval_ms;

    let settings_msg =
        serde_json::json!({ "type": "settings", "typewriter_char_ms": typewriter_char_ms });
    if ws
        .send(Message::Text(settings_msg.to_string().into()))
        .await
        .is_err()
    {
        return;
    }

    let (tick_tx, mut tick_rx) = mpsc::unbounded_channel::<Result<(String, bool), String>>();
    let tick_paused = Arc::new(AtomicBool::new(false));
    let tick_paused_bg = Arc::clone(&tick_paused);
    let runtime_bg = runtime.clone();
    eprintln!("[ws] handle_ws started for session={session_id}");

    tokio::task::spawn_blocking(move || {
        let duration = std::time::Duration::from_millis(npc_tick_interval_ms);
        loop {
            std::thread::sleep(duration);
            if tick_paused_bg.load(Ordering::Relaxed) {
                continue;
            }
            let result = match runtime_bg.run_tick() {
                Ok(outcome) => Ok((outcome.text, outcome.game_over)),
                Err(e) => Err(e.to_string()),
            };
            if tick_tx.send(result).is_err() {
                break;
            }
        }
    });

    loop {
        tokio::select! {
            maybe_tick = tick_rx.recv() => {
                match maybe_tick {
                    Some(Ok((text, game_over))) => {
                        let movie = game_manager::consume_projector_sequence(&runtime);
                        if let Some(movie) = movie {
                            let msg = serde_json::json!({
                                "type": "movie",
                                "title": movie.title,
                                "frames": movie.frames,
                                "narrative_lines": movie.narrative_lines,
                            });
                            if ws.send(Message::Text(msg.to_string().into())).await.is_err() {
                                break;
                            }
                        }
                        if text.is_empty() {
                            continue;
                        }
                        let msg = serde_json::json!({
                            "type": "tick",
                            "text": text,
                            "game_over": game_over,
                        });
                        if ws.send(Message::Text(msg.to_string().into())).await.is_err() {
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        let msg = serde_json::json!({ "type": "error", "text": e });
                        if ws.send(Message::Text(msg.to_string().into())).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
            maybe_msg = ws.recv() => {
                match maybe_msg {
                    Some(Ok(Message::Text(text))) => {
                        if text == "pause" {
                            tick_paused.store(true, Ordering::Relaxed);
                        } else if text == "resume" {
                            tick_paused.store(false, Ordering::Relaxed);
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}
