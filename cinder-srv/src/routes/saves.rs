use axum::{
    extract::{Path, State},
    http::StatusCode,
    middleware,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use std::sync::Arc;

use crate::auth::{auth_middleware, AuthPlayer};
use crate::game_manager;

use super::AppState;

fn internal<E: ToString>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

#[derive(Serialize)]
pub struct SavedGameInfo {
    pub session_id: String,
    pub created_at: String,
}

pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/games/{id}/save", post(save_game))
        .route("/api/games/{id}/saves", get(list_saves))
        .route("/api/games/{id}/load", post(load_game))
        .route_layer(middleware::from_fn_with_state(state, auth_middleware))
}

pub async fn save_game(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(session_id): Path<String>,
) -> Result<Json<SavedGameInfo>, (StatusCode, String)> {
    let state_json =
        game_manager::export_session_state(&state.sessions, &session_id).map_err(internal)?;

    sqlx::query(
        "UPDATE game_sessions SET state_json = ?, updated_at = datetime('now') WHERE id = ? AND player_id = ?",
    )
    .bind(&state_json)
    .bind(&session_id)
    .bind(&auth.id)
    .execute(&*state.pool)
    .await
    .map_err(internal)?;

    Ok(Json(SavedGameInfo {
        session_id,
        created_at: now_iso(),
    }))
}

pub async fn list_saves(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(session_id): Path<String>,
) -> Result<Json<Vec<SavedGameInfo>>, (StatusCode, String)> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT id, created_at FROM game_sessions WHERE id = ? AND player_id = ?",
    )
    .bind(&session_id)
    .bind(&auth.id)
    .fetch_optional(&*state.pool)
    .await
    .map_err(internal)?
    .into_iter()
    .map(|(id, created_at)| SavedGameInfo {
        session_id: id,
        created_at,
    })
    .collect::<Vec<_>>();

    Ok(Json(rows))
}

#[derive(serde::Deserialize)]
pub struct LoadGameRequest {
    pub session_id: String,
}

#[derive(Serialize)]
pub struct LoadGameResponse {
    pub session_id: String,
    pub pack_id: String,
}

pub async fn load_game(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Json(req): Json<LoadGameRequest>,
) -> Result<Json<LoadGameResponse>, (StatusCode, String)> {
    let row = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, pack_id, state_json FROM game_sessions WHERE id = ? AND player_id = ?",
    )
    .bind(&req.session_id)
    .bind(&auth.id)
    .fetch_optional(&*state.pool)
    .await
    .map_err(internal)?
    .ok_or_else(|| (StatusCode::NOT_FOUND, "session not found".to_string()))?;

    let (sid, pack_id, state_json) = row;

    {
        let mut guard = state.sessions.lock().map_err(internal)?;
        guard.remove(&sid);
    }

    game_manager::create_session(&state.sessions, &auth.id, &pack_id, Some(&state_json))
        .map_err(internal)?;

    Ok(Json(LoadGameResponse {
        session_id: sid,
        pack_id,
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
