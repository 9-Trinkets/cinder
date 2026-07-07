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
    // In the stateless model, state is already persisted after every turn.
    // Touch updated_at to confirm the save.
    sqlx::query(
        "UPDATE game_sessions SET updated_at = NOW() WHERE id = $1 AND player_id = $2",
    )
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
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT id::text, created_at::text FROM game_sessions WHERE id = $1 AND player_id = $2",
    )
    .bind(&session_id)
    .bind(&auth.id)
    .fetch_optional(&*state.pool)
    .await
    .map_err(internal)?;

    let rows = row
        .into_iter()
        .map(|(id, created_at)| SavedGameInfo {
            session_id: id,
            created_at,
        })
        .collect::<Vec<_>>();

    Ok(Json(rows))
}

#[derive(Deserialize)]
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
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT id::text, pack_id FROM game_sessions WHERE id = $1 AND player_id = $2",
    )
    .bind(&req.session_id)
    .bind(&auth.id)
    .fetch_optional(&*state.pool)
    .await
    .map_err(internal)?
    .ok_or_else(|| (StatusCode::NOT_FOUND, "session not found".to_string()))?;

    let (sid, pack_id) = row;

    Ok(Json(LoadGameResponse { session_id: sid, pack_id }))
}

fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("epoch={d}")
}
