mod auth;
mod config;
mod db;
mod game_manager;
mod routes;

use std::sync::Arc;

use axum::{Json, http::HeaderValue, routing::get};
use serde::Serialize;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::GovernorLayer;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::db::DbPool;
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
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Arc::new(Config::from_env());
    let pool: DbPool = Arc::new(
        db::init_pool(&config)
            .await
            .expect("failed to initialize database"),
    );

    let state = Arc::new(AppState {
        pool,
        config: config.clone(),
    });

    if config.jwt_secret == "change-me-in-production" {
        tracing::warn!("CINDER_JWT_SECRET is using the default value. Set a strong secret for production.");
    }

    let cors = match &config.cors_origin {
        Some(origin) => {
            let parsed = origin
                .parse::<HeaderValue>()
                .expect("invalid CINDER_CORS_ORIGIN");
            CorsLayer::new()
                .allow_origin(parsed)
                .allow_methods(Any)
                .allow_headers(Any)
        }
        None => CorsLayer::permissive(),
    };

    // Rate limiter for auth endpoints (per-IP)
    let governor_config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(2)
            .burst_size(5)
            .finish()
            .unwrap(),
    );
    let auth_routes = axum::Router::new()
        .route(
            "/api/auth/signup",
            axum::routing::post(routes::auth::signup),
        )
        .route(
            "/api/auth/login",
            axum::routing::post(routes::auth::login),
        )
        .layer(GovernorLayer::new(governor_config));

    let app = routes::game::routes(state.clone())
        .merge(routes::saves::routes(state.clone()))
        .merge(auth_routes)
        .route("/", get(health))
        .route("/api/health", get(health))
        .layer(cors)
        .with_state(state)
        .fallback(|| async { (axum::http::StatusCode::NOT_FOUND, "not found") });

    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("cinder-srv listening on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind address");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .expect("server error");
}
