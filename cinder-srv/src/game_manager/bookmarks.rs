use cinder_core::content::loader;
use cinder_core::engine::narrative::NarrativeLine;
use cinder_core::engine::state::{GamePhase, WorldState};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::db::{fetch_transcript_lines, load_play_row_unlocked, parse_uuid};
use super::ui::build_ui_snapshot;
use super::{UiSnapshot, build_runtime_impl, get_transcript};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BookmarkInfo {
    pub id: String,
    pub pack_id: String,
    pub play_id: Option<String>,
    pub label: String,
    pub turn_number: u32,
    pub current_room_name: String,
    pub day_number: u32,
    pub time_label: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StoredTranscriptEntry {
    pub role: String,
    pub text: String,
    pub turn_number: i32,
}

#[derive(Serialize)]
pub struct ResumeBookmarkResult {
    pub play_id: String,
    pub snapshot: UiSnapshot,
    pub lines: Vec<NarrativeLine>,
}

pub async fn create_bookmark(
    pool: &PgPool,
    play_id: &str,
    player_id: &str,
    label_override: Option<String>,
) -> Result<BookmarkInfo, String> {
    let play_uuid = parse_uuid(play_id, "play id")?;
    let player_uuid = parse_uuid(player_id, "player id")?;

    let (pack_id, locale, state_json) =
        load_play_row_unlocked(pool, &play_uuid, &player_uuid).await?;

    let state = WorldState::from_saved_json(&state_json)
        .map_err(|e| format!("failed to deserialize state: {e}"))?;

    if state.phase != GamePhase::Active {
        return Err("cannot bookmark when game is over".to_string());
    }

    let content = loader::load_named_pack(&pack_id, Some(&locale))
        .map_err(|e| format!("failed to load pack '{pack_id}' locale '{locale}': {e}"))?;
    let runtime = build_runtime_impl(content, &state_json)?;

    let turn_number = state.turn_number;
    let current_room_id = state.current_room_id.clone();
    let current_room_name = runtime
        .content()
        .room(&current_room_id)
        .map(|r| r.title.clone())
        .unwrap_or_else(|| current_room_id.clone());
    let day_number = (state.current_time_minutes / (24 * 60)) + 1;
    let time_label = runtime.current_time_label().unwrap_or_default();

    let label = match label_override {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => {
            if time_label.is_empty() {
                format!("{current_room_name} (Day {day_number})")
            } else {
                format!("{current_room_name} (Day {day_number}, {time_label})")
            }
        }
    };

    // Load full transcript entries for this play
    let raw_entries = sqlx::query_as::<_, (String, String, i32)>(
        "SELECT role, text, turn_number
         FROM transcript_entries
         WHERE play_id = $1
         ORDER BY turn_number ASC, id ASC",
    )
    .bind(play_uuid)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("db transcript error: {e}"))?;

    let stored_entries: Vec<StoredTranscriptEntry> = raw_entries
        .into_iter()
        .map(|(role, text, turn_number)| StoredTranscriptEntry {
            role,
            text,
            turn_number,
        })
        .collect();

    let transcript_json =
        serde_json::to_string(&stored_entries).map_err(|e| format!("serialization error: {e}"))?;

    let bookmark_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO bookmarks (
            id, player_id, pack_id, play_id, label, locale,
            state_json, transcript_json, turn_number,
            current_room_id, current_room_name, day_number, time_label, created_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6,
            $7::jsonb, $8::jsonb, $9,
            $10, $11, $12, $13, NOW()
        )",
    )
    .bind(bookmark_id)
    .bind(player_uuid)
    .bind(&pack_id)
    .bind(play_uuid)
    .bind(&label)
    .bind(&locale)
    .bind(&state_json)
    .bind(&transcript_json)
    .bind(turn_number as i32)
    .bind(&current_room_id)
    .bind(&current_room_name)
    .bind(day_number as i32)
    .bind(&time_label)
    .execute(pool)
    .await
    .map_err(|e| format!("db insert bookmark error: {e}"))?;

    let now_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string();

    Ok(BookmarkInfo {
        id: bookmark_id.to_string(),
        pack_id,
        play_id: Some(play_uuid.to_string()),
        label,
        turn_number,
        current_room_name,
        day_number,
        time_label,
        created_at: now_epoch,
    })
}

pub async fn list_pack_bookmarks(
    pool: &PgPool,
    player_id: &str,
    pack_id: &str,
) -> Result<Vec<BookmarkInfo>, String> {
    let player_uuid = parse_uuid(player_id, "player id")?;

    let rows = sqlx::query_as::<_, (String, String, Option<String>, String, i32, String, i32, String, i64)>(
        "SELECT id::text, pack_id, play_id::text, label, turn_number, current_room_name, day_number, time_label, EXTRACT(EPOCH FROM created_at)::bigint
         FROM bookmarks
         WHERE player_id = $1 AND pack_id = $2
         ORDER BY created_at DESC",
    )
    .bind(player_uuid)
    .bind(pack_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("db list bookmarks error: {e}"))?;

    Ok(rows
        .into_iter()
        .map(
            |(
                id,
                pack_id,
                play_id,
                label,
                turn_number,
                current_room_name,
                day_number,
                time_label,
                created_at,
            )| {
                BookmarkInfo {
                    id,
                    pack_id,
                    play_id,
                    label,
                    turn_number: turn_number as u32,
                    current_room_name,
                    day_number: day_number as u32,
                    time_label,
                    created_at: created_at.to_string(),
                }
            },
        )
        .collect())
}

pub async fn list_play_bookmarks(
    pool: &PgPool,
    play_id: &str,
    player_id: &str,
) -> Result<Vec<BookmarkInfo>, String> {
    let play_uuid = parse_uuid(play_id, "play id")?;
    let player_uuid = parse_uuid(player_id, "player id")?;

    let pack_id = sqlx::query_scalar::<_, String>(
        "SELECT pack_id FROM game_plays WHERE id = $1 AND player_id = $2",
    )
    .bind(play_uuid)
    .bind(player_uuid)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("db error: {e}"))?
    .ok_or_else(|| "play not found".to_string())?;

    list_pack_bookmarks(pool, player_id, &pack_id).await
}

pub async fn delete_bookmark(
    pool: &PgPool,
    bookmark_id: &str,
    player_id: &str,
) -> Result<(), String> {
    let bookmark_uuid = parse_uuid(bookmark_id, "bookmark id")?;
    let player_uuid = parse_uuid(player_id, "player id")?;

    sqlx::query("DELETE FROM bookmarks WHERE id = $1 AND player_id = $2")
        .bind(bookmark_uuid)
        .bind(player_uuid)
        .execute(pool)
        .await
        .map_err(|e| format!("db delete bookmark error: {e}"))?;

    Ok(())
}

pub async fn resume_bookmark(
    pool: &PgPool,
    bookmark_id: &str,
    player_id: &str,
) -> Result<ResumeBookmarkResult, String> {
    let bookmark_uuid = parse_uuid(bookmark_id, "bookmark id")?;
    let player_uuid = parse_uuid(player_id, "player id")?;

    let row = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT pack_id, locale, state_json::text, transcript_json::text
         FROM bookmarks
         WHERE id = $1 AND player_id = $2",
    )
    .bind(bookmark_uuid)
    .bind(player_uuid)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("db error: {e}"))?
    .ok_or_else(|| "bookmark not found".to_string())?;

    let (pack_id, locale, state_json, transcript_json) = row;

    let stored_entries: Vec<StoredTranscriptEntry> = serde_json::from_str(&transcript_json)
        .map_err(|e| format!("failed to deserialize transcript from bookmark: {e}"))?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("db begin error: {e}"))?;

    // Check if an existing game_plays row exists for this player and pack
    let existing_play = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM game_plays WHERE player_id = $1 AND pack_id = $2 FOR UPDATE",
    )
    .bind(player_uuid)
    .bind(&pack_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("db error: {e}"))?;

    let target_play_uuid = match existing_play {
        Some(play_id) => {
            // Replace existing running session
            sqlx::query(
                "UPDATE game_plays
                 SET locale = $1, state_json = $2::jsonb, updated_at = NOW()
                 WHERE id = $3 AND player_id = $4",
            )
            .bind(&locale)
            .bind(&state_json)
            .bind(play_id)
            .bind(player_uuid)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("db update play error: {e}"))?;

            // Replace transcript entries
            sqlx::query("DELETE FROM transcript_entries WHERE play_id = $1")
                .bind(play_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("db delete transcript error: {e}"))?;

            play_id
        }
        None => {
            // Create a new running session for this pack
            let new_play_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO game_plays (id, player_id, pack_id, locale, state_json, updated_at)
                 VALUES ($1, $2, $3, $4, $5::jsonb, NOW())",
            )
            .bind(new_play_id)
            .bind(player_uuid)
            .bind(&pack_id)
            .bind(&locale)
            .bind(&state_json)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("db insert play error: {e}"))?;

            new_play_id
        }
    };

    // Repopulate transcript entries from the bookmark
    for entry in stored_entries {
        sqlx::query(
            "INSERT INTO transcript_entries (play_id, turn_number, role, text)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(target_play_uuid)
        .bind(entry.turn_number)
        .bind(&entry.role)
        .bind(&entry.text)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("db restore transcript error: {e}"))?;
    }

    tx.commit()
        .await
        .map_err(|e| format!("db commit error: {e}"))?;

    // Now build UI snapshot and lines
    let transcript_lines = fetch_transcript_lines(pool, &target_play_uuid, &player_uuid).await?;
    let content = loader::load_named_pack(&pack_id, Some(&locale))
        .map_err(|e| format!("failed to load pack '{pack_id}' locale '{locale}': {e}"))?;
    let runtime = build_runtime_impl(content, &state_json)?;
    let snapshot = build_ui_snapshot(&runtime, &pack_id, &transcript_lines)?;
    let lines = get_transcript(
        pool,
        &target_play_uuid.to_string(),
        &player_uuid.to_string(),
    )
    .await?;

    Ok(ResumeBookmarkResult {
        play_id: target_play_uuid.to_string(),
        snapshot,
        lines,
    })
}
