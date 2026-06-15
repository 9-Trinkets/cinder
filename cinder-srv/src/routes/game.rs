use axum::{
    extract::{Path, State},
    http::StatusCode,
    middleware,
    routing::{get, post},
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
}

#[derive(Deserialize)]
pub struct CreateSessionRequest {
    pub pack_id: String,
}

pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/games", get(list_sessions).post(create_session))
        .route("/api/games/{id}/command", post(run_command))
        .route_layer(middleware::from_fn_with_state(state, auth_middleware))
}

pub async fn create_session(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<SessionInfo>, (StatusCode, String)> {
    let (session_id, _) = game_manager::create_session(&state.sessions, &auth.id, &req.pack_id, None)
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

    Ok(Json(SessionInfo {
        session_id,
        pack_id: req.pack_id,
        created_at: now_iso(),
        updated_at: now_iso(),
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
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct CommandRequest {
    pub input: String,
}

#[derive(Serialize)]
pub struct CommandResponse {
    pub text: String,
    pub game_over: bool,
}

pub async fn run_command(
    State(state): State<Arc<AppState>>,
    _auth: AuthPlayer,
    Path(session_id): Path<String>,
    Json(req): Json<CommandRequest>,
) -> Result<Json<CommandResponse>, (StatusCode, String)> {
    let outcome = game_manager::run_command(&state.sessions, &session_id, &req.input)
        .map_err(internal)?;
    Ok(Json(CommandResponse {
        text: outcome.text,
        game_over: outcome.game_over,
    }))
}

fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("epoch={d}")
}
