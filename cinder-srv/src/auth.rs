use axum::{
    extract::{FromRequestParts, Request},
    http::{StatusCode, request::Parts},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::routes::AppState;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

pub fn create_token(player_id: &str, secret: &[u8]) -> Result<String, jsonwebtoken::errors::Error> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    let claims = Claims {
        sub: player_id.to_string(),
        iat: now,
        exp: now + 86400 * 7,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )
}

pub fn validate_token(token: &str, secret: &[u8]) -> Result<Claims, jsonwebtoken::errors::Error> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::default(),
    )
    .map(|data| data.claims)
}

#[derive(Debug, Clone)]
pub struct AuthPlayer {
    pub id: String,
}

impl<S: Sync> FromRequestParts<S> for AuthPlayer {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthPlayer>()
            .cloned()
            .ok_or((StatusCode::UNAUTHORIZED, "not authenticated"))
    }
}

pub async fn auth_middleware(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "missing Authorization header".to_string(),
            )
        })?;

    let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            "invalid Authorization format".to_string(),
        )
    })?;

    let claims = validate_token(token, state.config.jwt_secret.as_bytes()).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            "invalid or expired token".to_string(),
        )
    })?;

    req.extensions_mut().insert(AuthPlayer { id: claims.sub });

    Ok(next.run(req).await)
}
