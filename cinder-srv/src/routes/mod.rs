pub mod auth;
pub mod game;
pub mod saves;

use std::sync::Arc;

use crate::config::Config;
use crate::db::DbPool;
use crate::game_manager::SessionMap;

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub config: Arc<Config>,
    pub sessions: SessionMap,
}
