use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

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
    let username = req.username.trim().to_string();
    let password = req.password;

    if username.len() < 3 || username.len() > 32 {
        return Err((
            StatusCode::BAD_REQUEST,
            "username must be 3–32 characters".to_string(),
        ));
    }
    if !username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "username: letters, numbers, underscores, hyphens only".to_string(),
        ));
    }
    if password.len() < 8 || password.len() > 128 {
        return Err((
            StatusCode::BAD_REQUEST,
            "password must be 8–128 characters".to_string(),
        ));
    }

    let exists =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::bigint FROM players WHERE username = $1")
            .bind(&username)
            .fetch_one(&*state.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if exists > 0 {
        return Err((StatusCode::CONFLICT, "username taken".to_string()));
    }

    let password_hash = bcrypt::hash(&password, bcrypt::DEFAULT_COST)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let player_id = Uuid::new_v4();
    sqlx::query("INSERT INTO players (id, username, password_hash) VALUES ($1, $2, $3)")
        .bind(player_id)
        .bind(&username)
        .bind(&password_hash)
        .execute(&*state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let token = auth::create_token(&player_id.to_string(), state.config.jwt_secret.as_bytes())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AuthResponse {
        token,
        player_id: player_id.to_string(),
    }))
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    let username = req.username.trim().to_string();
    if username.is_empty() || req.password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "invalid credentials".to_string(),
        ));
    }

    let row = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, password_hash FROM players WHERE username = $1",
    )
    .bind(&username)
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

    let token = auth::create_token(&player_id.to_string(), state.config.jwt_secret.as_bytes())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AuthResponse {
        token,
        player_id: player_id.to_string(),
    }))
}
