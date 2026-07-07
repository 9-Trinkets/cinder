use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct Config {
    pub jwt_secret: String,
    pub database_url: String,
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            jwt_secret: std::env::var("CINDER_JWT_SECRET")
                .unwrap_or_else(|_| "change-me-in-production".to_string()),
            database_url: std::env::var("CINDER_DATABASE_URL")
                .unwrap_or_else(|_| "postgres://localhost:5432/cinder".to_string()),
            host: std::env::var("CINDER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: std::env::var("CINDER_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3000),
        }
    }
}
