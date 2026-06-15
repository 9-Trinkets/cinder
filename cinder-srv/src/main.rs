mod auth;
mod config;
mod db;
mod game_manager;
mod routes;

use std::sync::Arc;

use axum::{routing::get, Json};
use serde::Serialize;
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::db::DbPool;
use crate::game_manager::SessionMap;
use crate::routes::AppState;

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Arc::new(Config::from_env());
    let pool: DbPool = Arc::new(
        db::init_pool(&config)
            .await
            .expect("failed to initialize database"),
    );
    let sessions: SessionMap = game_manager::new_session_map();

    let state = Arc::new(AppState {
        pool,
        config: config.clone(),
        sessions,
    });

    let app = routes::game::routes(state.clone())
        .merge(routes::saves::routes(state.clone()))
        .route("/api/auth/signup", axum::routing::post(routes::auth::signup))
        .route("/api/auth/login", axum::routing::post(routes::auth::login))
        .route("/", get(health))
        .route("/api/health", get(health))
        .layer(CorsLayer::permissive())
        .with_state(state)
        .fallback(|| async { (axum::http::StatusCode::NOT_FOUND, "not found") });

    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("cinder-srv listening on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind address");
    axum::serve(listener, app)
        .await
        .expect("server error");
}
