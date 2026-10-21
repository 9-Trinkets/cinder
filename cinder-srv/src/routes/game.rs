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
use crate::game_manager::{self, LocaleItem};
use cinder_core::content::loader;

use super::AppState;

fn internal<E: ToString>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

#[derive(Serialize)]
pub struct PlayInfo {
    pub play_id: String,
    pub pack_id: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub intro_text: String,
    #[serde(default)]
    pub day_number: u32,
    #[serde(default)]
    pub current_room_name: String,
}

#[derive(Deserialize)]
pub struct CreatePlayRequest {
    pub pack_id: String,
    /// Optional explicit locale for this play. When absent, the player's
    /// account preference is used, then the pack's own default language.
    #[serde(default)]
    pub locale: Option<String>,
}

pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let auth_routes = Router::new()
        .route("/api/me", get(get_me).put(update_me))
        .route("/api/packs", get(list_packs))
        .route(
            "/api/packs/{pack_id}/bookmarks",
            get(list_pack_bookmarks_handler),
        )
        .route("/api/games", get(list_plays).post(create_play))
        .route("/api/games/{id}/command", post(run_command))
        .route("/api/games/{id}/tick", post(run_tick))
        .route("/api/games/{id}/ui", get(play_ui))
        .route("/api/games/{id}/transcript", get(transcript_handler))
        .route("/api/games/{id}/locale", post(set_locale_handler))
        .route("/api/games/{id}/continue", post(continue_play_handler))
        .route(
            "/api/games/{id}/bookmarks",
            get(list_play_bookmarks_handler).post(create_bookmark_handler),
        )
        .route("/api/bookmarks/{id}/resume", post(resume_bookmark_handler))
        .route("/api/bookmarks/{id}", delete(delete_bookmark_handler))
        .route("/api/games/{id}", delete(delete_play_handler))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let ws_routes = Router::new()
        .route("/api/games/{id}/ws", get(ws_tick_handler))
        .with_state(state);

    auth_routes.merge(ws_routes)
}

pub async fn create_play(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Json(req): Json<CreatePlayRequest>,
) -> Result<Json<PlayInfo>, (StatusCode, String)> {
    let player_uuid = Uuid::parse_str(&auth.id).map_err(internal)?;
    let existing = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM game_plays WHERE player_id = $1 AND pack_id = $2 LIMIT 1",
    )
    .bind(player_uuid)
    .bind(&req.pack_id)
    .fetch_optional(&*state.pool)
    .await
    .map_err(internal)?;

    if existing.is_some() {
        return Err((
            StatusCode::CONFLICT,
            "A running chronicle already exists for this tale. Delete it before beginning a new one.".to_string(),
        ));
    }

    let (play_id, title, intro_text) =
        game_manager::create_play(&state.pool, &auth.id, &req.pack_id, req.locale.as_deref())
            .await
            .map_err(internal)?;

    Ok(Json(PlayInfo {
        play_id,
        pack_id: req.pack_id,
        created_at: now_unix_secs(),
        updated_at: now_unix_secs(),
        day_number: 1,
        current_room_name: String::new(),
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
    pub tags: Vec<String>,
    /// The pack's own default language when no locale is chosen.
    pub default_language: String,
    /// Every locale this pack ships, for availability-aware UI.
    pub locale_options: Vec<LocaleItem>,
}

#[derive(Deserialize)]
pub struct ListPacksQuery {
    /// Display locale to localize pack metadata for. Falls back to the account
    /// preference, then each pack's own default language.
    #[serde(default)]
    pub locale: Option<String>,
}

pub async fn list_packs(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Query(query): Query<ListPacksQuery>,
) -> Json<Vec<PackInfo>> {
    let requested = match query.locale.filter(|code| !code.trim().is_empty()) {
        Some(code) => Some(code),
        None => account_locale(&state, &auth).await,
    };
    Json(
        cinder_core::content::loader::available_packs()
            .into_iter()
            .map(|id| {
                let locale_options = pack_locale_options(&id);
                let locale = requested
                    .as_deref()
                    .filter(|code| locale_options.iter().any(|option| option.code == *code));
                let settings =
                    cinder_core::content::loader::load_pack_settings_for_locale(&id, locale)
                        .unwrap_or_default();
                let title = if settings.title.is_empty() {
                    id.clone()
                } else {
                    settings.title
                };
                let default_language = if settings.default_language.trim().is_empty() {
                    cinder_core::content::loader::DEFAULT_LOCALE.to_string()
                } else {
                    settings.default_language.clone()
                };
                PackInfo {
                    id,
                    title,
                    tagline: settings.tagline,
                    description: settings.description,
                    theme: settings.theme,
                    tags: settings.tags,
                    default_language,
                    locale_options,
                }
            })
            .collect(),
    )
}

/// The signed-in account's saved display-language preference, if any.
async fn account_locale(state: &AppState, auth: &AuthPlayer) -> Option<String> {
    let player_uuid = Uuid::parse_str(&auth.id).ok()?;
    sqlx::query_scalar::<_, Option<String>>("SELECT preferred_locale FROM players WHERE id = $1")
        .bind(player_uuid)
        .fetch_optional(&*state.pool)
        .await
        .ok()
        .flatten()
        .flatten()
        .filter(|code| !code.trim().is_empty())
}

/// Locale options for a single pack, or an empty list if the pack is unreadable.
fn pack_locale_options(pack_id: &str) -> Vec<LocaleItem> {
    loader::available_locales(&loader::pack_dir(pack_id))
        .unwrap_or_default()
        .into_iter()
        .map(|locale| LocaleItem {
            code: locale.code,
            label: locale.label,
        })
        .collect()
}

#[derive(Serialize)]
pub struct PlayerProfile {
    pub player_id: String,
    pub username: String,
    /// The account-level display-language preference, or null when unset.
    pub preferred_locale: Option<String>,
    /// Union of every locale shipped by any pack, for the account picker.
    pub locale_options: Vec<LocaleItem>,
}

pub async fn get_me(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
) -> Result<Json<PlayerProfile>, (StatusCode, String)> {
    let player_uuid = Uuid::parse_str(&auth.id).map_err(internal)?;
    let row = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT username, preferred_locale FROM players WHERE id = $1",
    )
    .bind(player_uuid)
    .fetch_optional(&*state.pool)
    .await
    .map_err(internal)?
    .ok_or_else(|| (StatusCode::NOT_FOUND, "player not found".to_string()))?;

    Ok(Json(PlayerProfile {
        player_id: auth.id,
        username: row.0,
        preferred_locale: row.1.filter(|locale| !locale.trim().is_empty()),
        locale_options: account_locale_options(),
    }))
}

#[derive(Deserialize)]
pub struct UpdateMeRequest {
    /// The new preference. An empty string clears it (back to pack defaults).
    pub preferred_locale: Option<String>,
}

pub async fn update_me(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Json(req): Json<UpdateMeRequest>,
) -> Result<Json<PlayerProfile>, (StatusCode, String)> {
    let player_uuid = Uuid::parse_str(&auth.id).map_err(internal)?;
    let normalized = req
        .preferred_locale
        .as_deref()
        .map(str::trim)
        .filter(|locale| !locale.is_empty());

    sqlx::query("UPDATE players SET preferred_locale = $1 WHERE id = $2")
        .bind(normalized)
        .bind(player_uuid)
        .execute(&*state.pool)
        .await
        .map_err(internal)?;

    get_me(State(state), auth).await
}

/// Union of every pack's shipped locales, deduplicated by code and ordered with
/// the default locale first.
fn account_locale_options() -> Vec<LocaleItem> {
    let mut options: Vec<LocaleItem> = Vec::new();
    for pack_id in loader::available_packs() {
        for locale in pack_locale_options(&pack_id) {
            if !options.iter().any(|existing| existing.code == locale.code) {
                options.push(locale);
            }
        }
    }
    options.sort_by(|left, right| {
        let default = loader::DEFAULT_LOCALE;
        match (left.code.as_str(), right.code.as_str()) {
            (a, b) if a == default && b != default => std::cmp::Ordering::Less,
            (a, b) if b == default && a != default => std::cmp::Ordering::Greater,
            _ => left.code.cmp(&right.code),
        }
    });
    options
}

pub async fn list_plays(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
) -> Result<Json<Vec<PlayInfo>>, (StatusCode, String)> {
    let rows = sqlx::query_as::<_, (String, String, i64, i64, Option<String>, Option<i64>)>(
        "SELECT id::text, pack_id, EXTRACT(EPOCH FROM created_at)::bigint, EXTRACT(EPOCH FROM updated_at)::bigint, \
         state_json->>'current_room_id', (state_json->>'current_time_minutes')::bigint \
         FROM game_plays WHERE player_id = $1 ORDER BY updated_at DESC",
    )
    .bind(Uuid::parse_str(&auth.id).map_err(internal)?)
    .fetch_all(&*state.pool)
    .await
    .map_err(internal)?;

    let mut room_titles: std::collections::HashMap<
        String,
        std::collections::HashMap<String, String>,
    > = std::collections::HashMap::new();

    Ok(Json(
        rows.into_iter()
            .map(
                |(id, pack_id, created_at, updated_at, room_id, time_minutes)| {
                    let day_number = time_minutes
                        .map(|minutes| (minutes as u32 / (24 * 60)) + 1)
                        .unwrap_or(0);
                    let current_room_name = room_id
                        .map(|room_id| {
                            let titles = room_titles.entry(pack_id.clone()).or_insert_with(|| {
                                cinder_core::content::loader::load_named_pack(&pack_id, None)
                                    .map(|pack| {
                                        pack.rooms
                                            .into_iter()
                                            .map(|room| (room.id, room.title))
                                            .collect()
                                    })
                                    .unwrap_or_default()
                            });
                            titles.get(&room_id).cloned().unwrap_or(room_id)
                        })
                        .unwrap_or_default();
                    PlayInfo {
                        play_id: id,
                        pack_id,
                        created_at: created_at.to_string(),
                        updated_at: updated_at.to_string(),
                        title: String::new(),
                        intro_text: String::new(),
                        day_number,
                        current_room_name,
                    }
                },
            )
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct CommandRequest {
    pub input: String,
    /// Readable text to record in the transcript instead of `input`, for
    /// commands issued by UI controls (e.g. `attack golem-pale-se`).
    #[serde(default)]
    pub display: Option<String>,
}

pub async fn run_command(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(play_id): Path<String>,
    Json(req): Json<CommandRequest>,
) -> Result<Json<game_manager::CommandResponse>, (StatusCode, String)> {
    let resp = game_manager::run_command(
        &state.pool,
        &play_id,
        &auth.id,
        &req.input,
        req.display.as_deref(),
    )
    .await
    .map_err(internal)?;
    Ok(Json(resp))
}

pub async fn run_tick(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(play_id): Path<String>,
) -> Result<Json<game_manager::CommandResponse>, (StatusCode, String)> {
    let resp = game_manager::run_realtime_tick(&state.pool, &play_id, &auth.id)
        .await
        .map_err(internal)?;
    Ok(Json(resp))
}

pub async fn transcript_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(play_id): Path<String>,
) -> Result<Json<Vec<cinder_core::engine::narrative::NarrativeLine>>, (StatusCode, String)> {
    let lines = game_manager::get_transcript(&state.pool, &play_id, &auth.id)
        .await
        .map_err(internal)?;
    Ok(Json(lines))
}

pub async fn play_ui(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(play_id): Path<String>,
) -> Result<Json<game_manager::UiSnapshot>, (StatusCode, String)> {
    let snapshot = game_manager::get_play_ui(&state.pool, &play_id, &auth.id)
        .await
        .map_err(internal)?;
    Ok(Json(snapshot))
}

#[derive(Deserialize)]
pub struct LocaleRequest {
    pub locale: String,
}

pub async fn set_locale_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(play_id): Path<String>,
    Json(req): Json<LocaleRequest>,
) -> Result<Json<game_manager::CommandResponse>, (StatusCode, String)> {
    let response = game_manager::set_locale(&state.pool, &play_id, &auth.id, &req.locale)
        .await
        .map_err(internal)?;
    Ok(Json(response))
}

pub async fn continue_play_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(play_id): Path<String>,
) -> Result<Json<game_manager::CommandResponse>, (StatusCode, String)> {
    let response = game_manager::continue_play(&state.pool, &play_id, &auth.id)
        .await
        .map_err(internal)?;
    Ok(Json(response))
}

pub async fn delete_play_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(play_id): Path<String>,
) -> Result<Json<()>, (StatusCode, String)> {
    let play_id = Uuid::parse_str(&play_id).map_err(internal)?;
    let player_id = Uuid::parse_str(&auth.id).map_err(internal)?;
    sqlx::query("DELETE FROM game_plays WHERE id = $1 AND player_id = $2")
        .bind(play_id)
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
    pub tick_ms: Option<u64>,
}

pub async fn ws_tick_handler(
    State(state): State<Arc<AppState>>,
    Path(play_id): Path<String>,
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

    let play_id = Uuid::parse_str(&play_id).map_err(internal)?;
    let player_id = Uuid::parse_str(&claims.sub).map_err(internal)?;

    let pool = state.pool.clone();
    let play_id_str = play_id.to_string();
    let player_id_str = player_id.to_string();
    let tick_ms = query.tick_ms.unwrap_or(2000);

    Ok(ws.on_upgrade(move |socket| handle_ws(socket, pool, play_id_str, player_id_str, tick_ms)))
}

async fn handle_ws(
    socket: axum::extract::ws::WebSocket,
    pool: crate::db::DbPool,
    play_id: String,
    player_id: String,
    tick_ms: u64,
) {
    use axum::extract::ws::Message;
    use futures_util::{SinkExt, StreamExt};

    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (out_tx, mut out_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

    let writer_handle = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    let tick_duration = std::time::Duration::from_millis(tick_ms);
    let mut interval =
        tokio::time::interval_at(tokio::time::Instant::now() + tick_duration, tick_duration);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let status_tx = out_tx.clone();
                let on_speaker = move |actor_name: &str| {
                    let _ = status_tx.send(Message::Text(serde_json::json!({
                        "type": "tick_status",
                        "status": "generating",
                        "actor_name": actor_name
                    }).to_string().into()));
                };

                match game_manager::run_realtime_tick_with_status(&pool, &play_id, &player_id, on_speaker).await {
                    Ok(resp) => {
                        let _ = out_tx.send(Message::Text(serde_json::json!({
                            "type": "tick_status",
                            "status": "idle"
                        }).to_string().into()));

                        if resp.lines.is_empty()
                            && resp.movie.is_none()
                            && !resp.game_over
                            && resp.act_closure.is_none()
                            && resp.ui_snapshot.is_none()
                        {
                            continue;
                        }
                        match serde_json::to_string(&resp) {
                            Ok(json) => {
                                if out_tx.send(Message::Text(json.into())).is_err() {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    Err(e) => {
                        tracing::error!("ws tick error: {e}");
                        let _ = out_tx.send(Message::Text(serde_json::json!({
                            "type": "tick_status",
                            "status": "idle"
                        }).to_string().into()));
                        break;
                    }
                }
            }
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }

    writer_handle.abort();
}

#[derive(Deserialize)]
pub struct CreateBookmarkRequest {
    pub label: Option<String>,
}

pub async fn create_bookmark_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(play_id): Path<String>,
    Json(req): Json<CreateBookmarkRequest>,
) -> Result<Json<game_manager::BookmarkInfo>, (StatusCode, String)> {
    let bookmark = game_manager::create_bookmark(&state.pool, &play_id, &auth.id, req.label)
        .await
        .map_err(internal)?;
    Ok(Json(bookmark))
}

pub async fn list_play_bookmarks_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(play_id): Path<String>,
) -> Result<Json<Vec<game_manager::BookmarkInfo>>, (StatusCode, String)> {
    let bookmarks = game_manager::list_play_bookmarks(&state.pool, &play_id, &auth.id)
        .await
        .map_err(internal)?;
    Ok(Json(bookmarks))
}

pub async fn list_pack_bookmarks_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(pack_id): Path<String>,
) -> Result<Json<Vec<game_manager::BookmarkInfo>>, (StatusCode, String)> {
    let bookmarks = game_manager::list_pack_bookmarks(&state.pool, &auth.id, &pack_id)
        .await
        .map_err(internal)?;
    Ok(Json(bookmarks))
}

pub async fn resume_bookmark_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(bookmark_id): Path<String>,
) -> Result<Json<game_manager::ResumeBookmarkResult>, (StatusCode, String)> {
    let result = game_manager::resume_bookmark(&state.pool, &bookmark_id, &auth.id)
        .await
        .map_err(internal)?;
    Ok(Json(result))
}

pub async fn delete_bookmark_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthPlayer,
    Path(bookmark_id): Path<String>,
) -> Result<Json<()>, (StatusCode, String)> {
    game_manager::delete_bookmark(&state.pool, &bookmark_id, &auth.id)
        .await
        .map_err(internal)?;
    Ok(Json(()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_locale_options_reflect_shipped_locales() {
        let layla = pack_locale_options("layla");
        let codes: Vec<&str> = layla.iter().map(|option| option.code.as_str()).collect();
        assert!(codes.contains(&"en"), "layla must ship en: {codes:?}");
        assert!(codes.contains(&"zh-TW"), "layla must ship zh-TW: {codes:?}");
        assert_eq!(codes.first().copied(), Some("en"), "en sorts first");

        let aera = pack_locale_options("aera");
        assert_eq!(
            aera.iter()
                .map(|option| option.code.as_str())
                .collect::<Vec<_>>(),
            vec!["en"],
            "aera ships English only"
        );

        // A pack without a locales/ directory still reports the default fallback.
        assert_eq!(
            pack_locale_options("does-not-exist")
                .iter()
                .map(|option| option.code.as_str())
                .collect::<Vec<_>>(),
            vec![loader::DEFAULT_LOCALE]
        );
    }

    #[test]
    fn account_locale_options_dedupes_and_puts_default_first() {
        let options = account_locale_options();
        assert!(!options.is_empty(), "at least one pack must be readable");
        assert_eq!(
            options.first().map(|option| option.code.as_str()),
            Some(loader::DEFAULT_LOCALE)
        );

        let mut codes: Vec<&str> = options.iter().map(|option| option.code.as_str()).collect();
        assert!(
            codes.contains(&"zh-TW"),
            "union must include zh-TW: {codes:?}"
        );

        let unique: std::collections::BTreeSet<&str> = codes.iter().copied().collect();
        assert_eq!(unique.len(), codes.len(), "no duplicate codes: {codes:?}");

        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), unique.len());
    }
}
