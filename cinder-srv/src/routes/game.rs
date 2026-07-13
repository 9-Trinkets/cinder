use axum::{
    Json, Router,
    extract::{Path, Query, State, ws::WebSocketUpgrade},
    http::StatusCode,
    middleware,
    response::IntoResponse,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::{AuthPlayer, auth_middleware, validate_token};
use crate::game_manager;

use super::AppState;

fn internal<E: ToString>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

#[derive(Serialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub pack_id: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub intro_text: String,
}

#[derive(Deserialize)]
pub struct CreateSessionRequest {
    pub pack_id: String,
}

pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let auth_routes = Router::new()
        .route("/api/packs", get(list_packs))
        .route("/api/games", get(list_sessions).post(create_session))
        .route("/api/games/{id}/command", post(run_command))
        .route("/api/games/{id}/tick", post(run_tick))
        .route("/api/games/{id}/ui", get(session_ui))
        .route("/api/games/{id}/transcript", get(transcript_handler))
        .route("/api/games/{id}/room", post(switch_room_handler))
        .route("/api/games/{id}/follow", post(follow_actor_handler))
        .route("/api/games/{id}/locale", post(set_locale_handler))
        .route("/api/games/{id}", delete(delete_session_handler))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    let ws_routes = Router::new()
        .route("/api/games/{id}/ws", get(ws_tick_handler))
        .with_state(state);

    auth_routes.merge(ws_routes)
}

pub async fn create_session(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<SessionInfo>, (StatusCode, String)> {
    let (session_id, title, intro_text) =
        game_manager::create_session(&state.pool, &auth.id, &req.pack_id)
            .await
            .map_err(internal)?;

    Ok(Json(SessionInfo {
        session_id,
        pack_id: req.pack_id,
        created_at: now_unix_secs(),
        updated_at: now_unix_secs(),
        title,
        intro_text,
    }))
}

#[derive(Serialize)]
pub struct PackInfo {
    pub id: String,
    pub title: String,
    pub tagline: String,
    pub description: String,
    pub theme: cinder_core::content::types::ThemeDefinition,
}

pub async fn list_packs() -> Json<Vec<PackInfo>> {
    Json(
        cinder_core::content::loader::available_packs()
            .into_iter()
            .map(|id| {
                let settings = cinder_core::content::loader::load_pack_settings(&id)
                    .unwrap_or_default();
                let title = if settings.title.is_empty() {
                    id.clone()
                } else {
                    settings.title
                };
                PackInfo {
                    id,
                    title,
                    tagline: settings.tagline,
                    description: settings.description,
                    theme: settings.theme,
                }
            })
            .collect(),
    )
}

pub async fn list_sessions(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
) -> Result<Json<Vec<SessionInfo>>, (StatusCode, String)> {
    let rows = sqlx::query_as::<_, (String, String, i64, i64)>(
        "SELECT id::text, pack_id, EXTRACT(EPOCH FROM created_at)::bigint, EXTRACT(EPOCH FROM updated_at)::bigint FROM game_sessions WHERE player_id = $1 ORDER BY updated_at DESC",
    )
    .bind(Uuid::parse_str(&auth.id).map_err(internal)?)
    .fetch_all(&*state.pool)
    .await
    .map_err(internal)?;

    Ok(Json(
        rows.into_iter()
            .map(|(id, pack_id, created_at, updated_at)| SessionInfo {
                session_id: id,
                pack_id,
                created_at: created_at.to_string(),
                updated_at: updated_at.to_string(),
                title: String::new(),
                intro_text: String::new(),
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct CommandRequest {
    pub input: String,
}

pub async fn run_command(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(session_id): Path<String>,
    Json(req): Json<CommandRequest>,
) -> Result<Json<game_manager::CommandResponse>, (StatusCode, String)> {
    let resp = game_manager::run_command(&state.pool, &session_id, &auth.id, &req.input)
        .await
        .map_err(internal)?;
    Ok(Json(resp))
}

pub async fn run_tick(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(session_id): Path<String>,
) -> Result<Json<game_manager::CommandResponse>, (StatusCode, String)> {
    let resp = game_manager::run_realtime_tick(&state.pool, &session_id, &auth.id)
        .await
        .map_err(internal)?;
    Ok(Json(resp))
}

pub async fn transcript_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(session_id): Path<String>,
) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    let lines = game_manager::get_transcript(&state.pool, &session_id, &auth.id)
        .await
        .map_err(internal)?;
    Ok(Json(lines))
}

pub async fn session_ui(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(session_id): Path<String>,
) -> Result<Json<game_manager::UiSnapshot>, (StatusCode, String)> {
    let snapshot = game_manager::get_session_ui(&state.pool, &session_id, &auth.id)
        .await
        .map_err(internal)?;
    Ok(Json(snapshot))
}

#[derive(Deserialize)]
pub struct RoomSwitchRequest {
    pub room_id: String,
}

pub async fn switch_room_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(session_id): Path<String>,
    Json(req): Json<RoomSwitchRequest>,
) -> Result<Json<game_manager::CommandResponse>, (StatusCode, String)> {
    let outcome = game_manager::switch_room(&state.pool, &session_id, &auth.id, &req.room_id)
        .await
        .map_err(internal)?;
    Ok(Json(outcome))
}

#[derive(Deserialize)]
pub struct FollowRequest {
    pub actor_id: Option<String>,
}

pub async fn follow_actor_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(session_id): Path<String>,
    Json(req): Json<FollowRequest>,
) -> Result<Json<game_manager::CommandResponse>, (StatusCode, String)> {
    let outcome =
        game_manager::follow_actor(&state.pool, &session_id, &auth.id, req.actor_id.as_deref())
            .await
            .map_err(internal)?;
    Ok(Json(outcome))
}

#[derive(Deserialize)]
pub struct LocaleRequest {
    pub locale: String,
}

pub async fn set_locale_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(session_id): Path<String>,
    Json(req): Json<LocaleRequest>,
) -> Result<Json<game_manager::CommandResponse>, (StatusCode, String)> {
    let response = game_manager::set_locale(&state.pool, &session_id, &auth.id, &req.locale)
        .await
        .map_err(internal)?;
    Ok(Json(response))
}

pub async fn delete_session_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(session_id): Path<String>,
) -> Result<Json<()>, (StatusCode, String)> {
    let session_id = Uuid::parse_str(&session_id).map_err(internal)?;
    let player_id = Uuid::parse_str(&auth.id).map_err(internal)?;
    sqlx::query("DELETE FROM game_sessions WHERE id = $1 AND player_id = $2")
        .bind(session_id)
        .bind(player_id)
        .execute(&*state.pool)
        .await
        .map_err(internal)?;
    Ok(Json(()))
}

fn now_unix_secs() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string()
}

#[derive(Deserialize)]
pub struct WsQuery {
    pub token: String,
}

pub async fn ws_tick_handler(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Query(query): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let claims =
        validate_token(&query.token, state.config.jwt_secret.as_bytes()).map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                "invalid or expired token".to_string(),
            )
        })?;

    let session_id = Uuid::parse_str(&session_id).map_err(internal)?;
    let player_id = Uuid::parse_str(&claims.sub).map_err(internal)?;

    let pool = state.pool.clone();
    let session_id_str = session_id.to_string();
    let player_id_str = player_id.to_string();

    Ok(ws.on_upgrade(move |socket| handle_ws(socket, pool, session_id_str, player_id_str)))
}

async fn handle_ws(
    mut socket: axum::extract::ws::WebSocket,
    pool: crate::db::DbPool,
    session_id: String,
    player_id: String,
) {
    use axum::extract::ws::Message;
    use futures_util::StreamExt;

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                match game_manager::run_realtime_tick(&pool, &session_id, &player_id).await {
                    Ok(resp) => {
                        if resp.text.is_empty() && resp.movie.is_none() && !resp.game_over && resp.session_closure.is_none() {
                            continue;
                        }
                        match serde_json::to_string(&resp) {
                            Ok(json) => {
                                if socket.send(Message::Text(json.into())).await.is_err() {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    Err(e) => {
                        tracing::error!("ws tick error: {e}");
                        break;
                    }
                }
            }
            msg = socket.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}
