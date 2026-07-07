use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    middleware,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::auth::{AuthPlayer, auth_middleware};

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
    let (checkpoint_id, created_at) =
        crate::game_manager::create_checkpoint(&state.pool, &session_id, &auth.id)
            .await
            .map_err(internal)?;

    Ok(Json(SavedGameInfo {
        session_id: checkpoint_id,
        created_at,
    }))
}

pub async fn list_saves(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(session_id): Path<String>,
) -> Result<Json<Vec<SavedGameInfo>>, (StatusCode, String)> {
    let rows = crate::game_manager::list_checkpoints(&state.pool, &session_id, &auth.id)
        .await
        .map_err(internal)?
        .into_iter()
        .map(|(checkpoint_id, created_at)| SavedGameInfo {
            session_id: checkpoint_id,
            created_at,
        })
        .collect::<Vec<_>>();

    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct LoadGameRequest {
    pub session_id: String,
    #[serde(default)]
    pub checkpoint_id: Option<String>,
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
    let session_id = req.session_id;
    let checkpoint_id = req
        .checkpoint_id
        .filter(|checkpoint_id| checkpoint_id != &session_id);
    let pack_id = crate::game_manager::restore_checkpoint(
        &state.pool,
        &session_id,
        &auth.id,
        checkpoint_id.as_deref(),
    )
    .await
    .map_err(internal)?;

    Ok(Json(LoadGameResponse {
        session_id,
        pack_id,
    }))
}
