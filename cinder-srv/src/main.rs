mod auth;
mod config;
mod db;
mod game_manager;
mod routes;

use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use axum::{
    Json,
    http::{HeaderMap, HeaderValue, Request},
    routing::get,
};
use serde::Serialize;
use tower_governor::{
    GovernorLayer, errors::GovernorError, governor::GovernorConfigBuilder,
    key_extractor::KeyExtractor,
};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TrustedProxyIpKeyExtractor;

impl KeyExtractor for TrustedProxyIpKeyExtractor {
    type Key = IpAddr;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        let peer_ip = req
            .extensions()
            .get::<axum::extract::ConnectInfo<SocketAddr>>()
            .map(|info: &axum::extract::ConnectInfo<SocketAddr>| info.0.ip())
            .ok_or(GovernorError::UnableToExtractKey)?;

        if is_trusted_proxy(peer_ip) {
            forwarded_ip(req.headers()).or(Some(peer_ip))
        } else {
            Some(peer_ip)
        }
        .ok_or(GovernorError::UnableToExtractKey)
    }
}

fn forwarded_ip(headers: &HeaderMap) -> Option<IpAddr> {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value
                .split(',')
                .find_map(|part| part.trim().parse::<IpAddr>().ok())
        })
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<IpAddr>().ok())
        })
}

fn is_trusted_proxy(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
        IpAddr::V6(ip) => ip.is_loopback() || ip.is_unique_local() || ip.is_unicast_link_local(),
    }
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
        if config.strict_config {
            panic!("CINDER_JWT_SECRET must be set when CINDER_STRICT_CONFIG is enabled");
        }
        tracing::warn!(
            "CINDER_JWT_SECRET is using the default value. Set a strong secret for production."
        );
    }

    if config.strict_config && config.cors_origin.is_none() {
        panic!("CINDER_CORS_ORIGIN must be set when CINDER_STRICT_CONFIG is enabled");
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
            .key_extractor(TrustedProxyIpKeyExtractor)
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
        .route("/api/auth/login", axum::routing::post(routes::auth::login))
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
