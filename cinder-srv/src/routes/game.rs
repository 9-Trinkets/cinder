use axum::{
    extract::{Path, State},
    http::StatusCode,
    middleware,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::auth::{auth_middleware, AuthPlayer};
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
        .route_layer(middleware::from_fn_with_state(state, auth_middleware))
}

pub async fn create_session(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<SessionInfo>, (StatusCode, String)> {
    let (session_id, title, intro_text) =
        game_manager::create_session(&state.pool, &auth.id, &req.pack_id)
            .await
            .map_err(internal)?;

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
        "SELECT id::text, pack_id, created_at::text, updated_at::text FROM game_sessions WHERE player_id = $1 ORDER BY updated_at DESC",
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
    let resp = game_manager::run_command(&state.pool, &session_id, &auth.id, &req.input)
        .await
        .map_err(internal)?;
    Ok(Json(resp))
}

pub async fn transcript_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(session_id): Path<String>,
) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    let lines = game_manager::get_transcript(&state.pool, &session_id, &auth.id)
        .await
        .map_err(internal)?;
    Ok(Json(lines))
}

pub async fn session_ui(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(session_id): Path<String>,
) -> Result<Json<game_manager::UiSnapshot>, (StatusCode, String)> {
    let snapshot = game_manager::get_session_ui(&state.pool, &session_id, &auth.id)
        .await
        .map_err(internal)?;
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
    let outcome = game_manager::switch_room(&state.pool, &session_id, &auth.id, &req.room_id)
        .await
        .map_err(internal)?;
    Ok(Json(game_manager::CommandResponse {
        text: outcome.text,
        game_over: outcome.game_over,
        movie: None,
        session_feedback: None,
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
    let outcome =
        game_manager::follow_actor(&state.pool, &session_id, &auth.id, req.actor_id.as_deref())
            .await
            .map_err(internal)?;
    Ok(Json(game_manager::CommandResponse {
        text: outcome.text,
        game_over: outcome.game_over,
        movie: None,
        session_feedback: None,
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
    let text = game_manager::set_locale(&state.pool, &session_id, &auth.id, &req.locale)
        .await
        .map_err(internal)?;
    Ok(Json(text))
}

pub async fn delete_session_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(session_id): Path<String>,
) -> Result<Json<()>, (StatusCode, String)> {
    sqlx::query("DELETE FROM game_sessions WHERE id = $1 AND player_id = $2")
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
