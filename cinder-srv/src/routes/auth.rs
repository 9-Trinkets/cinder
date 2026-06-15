use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::auth;

use super::AppState;

#[derive(Deserialize)]
pub struct SignupRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub player_id: String,
}

pub async fn signup(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SignupRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    if req.username.is_empty() || req.password.len() < 4 {
        return Err((
            StatusCode::BAD_REQUEST,
            "username required, password min 4 chars".to_string(),
        ));
    }

    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM players WHERE username = ?",
    )
    .bind(&req.username)
    .fetch_one(&*state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if exists > 0 {
        return Err((StatusCode::CONFLICT, "username taken".to_string()));
    }

    let password_hash = bcrypt::hash(&req.password, bcrypt::DEFAULT_COST)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let player_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO players (id, username, password_hash) VALUES (?, ?, ?)")
        .bind(&player_id)
        .bind(&req.username)
        .bind(&password_hash)
        .execute(&*state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let token = auth::create_token(&player_id, state.config.jwt_secret.as_bytes())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AuthResponse { token, player_id }))
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT id, password_hash FROM players WHERE username = ?",
    )
    .bind(&req.username)
    .fetch_optional(&*state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or_else(|| (StatusCode::UNAUTHORIZED, "invalid credentials".to_string()))?;

    let (player_id, password_hash) = row;

    let valid = bcrypt::verify(&req.password, &password_hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !valid {
        return Err((StatusCode::UNAUTHORIZED, "invalid credentials".to_string()));
    }

    let token = auth::create_token(&player_id, state.config.jwt_secret.as_bytes())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AuthResponse { token, player_id }))
}
