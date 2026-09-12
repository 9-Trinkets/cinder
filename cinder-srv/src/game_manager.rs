use cinder_core::content::loader;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

mod actions;
mod db;
mod response;
mod ui;

pub use self::actions::{
    continue_play, create_play, follow_actor, run_command, run_realtime_tick, set_locale,
    switch_room,
};
pub use self::response::{consume_projector_sequence, CommandResponse};
pub use self::ui::UiSnapshot;

use db::{
    fetch_transcript_lines, insert_transcript_entries, load_play_row, load_play_row_unlocked,
    narrative_kind, parse_uuid, replace_transcript_entries_with_lines,
    transcript_lines_from_state_json, PendingTranscriptEntry, MAX_PLAY_WRITE_RETRIES,
};

async fn with_runtime<F, R>(
    pool: &PgPool,
    play_id: &Uuid,
    player_id: &Uuid,
    f: F,
) -> Result<R, String>
where
    F: Fn(&CinderRuntime, &str, &[String]) -> Result<(R, Vec<PendingTranscriptEntry>), String>
        + Send
        + Sync
        + 'static,
    R: Send + 'static,
{
    let f = Arc::new(f);

    for _attempt in 0..MAX_PLAY_WRITE_RETRIES {
        let (pack_id, locale, state_json) =
            load_play_row_unlocked(pool, play_id, player_id).await?;
        let transcript_lines = {
            let rows = fetch_transcript_lines(pool, play_id, player_id).await?;
            if rows.is_empty() {
                transcript_lines_from_state_json(&state_json)?
            } else {
                rows
            }
        };

        let f = Arc::clone(&f);
        let base_pack_id = pack_id.clone();
        let base_locale = locale.clone();
        let base_state_json = state_json.clone();
        let (result, transcript_entries, persisted_locale, new_state_json, turn_number) =
            tokio::task::spawn_blocking(move || {
                let content = loader::load_named_pack(&pack_id, Some(&locale)).map_err(|e| {
                    format!("failed to load pack '{pack_id}' locale '{locale}': {e}")
                })?;

                let runtime = build_runtime_impl(content, &state_json)?;

                let (result, transcript_entries) = f(&runtime, &pack_id, &transcript_lines)?;

                let persisted_locale = runtime.content().locale.clone();
                let new_state = runtime
                    .export_state()
                    .map_err(|e| format!("state export error: {e}"))?;
                let turn_number = new_state.turn_number;
                let new_state_json = serde_json::to_string(&new_state)
                    .map_err(|e| format!("serialization error: {e}"))?;

                Ok::<_, String>((
                    result,
                    transcript_entries,
                    persisted_locale,
                    new_state_json,
                    turn_number,
                ))
            })
            .await
            .map_err(|e| format!("blocking task panicked: {e:?}"))??;

        let mut tx = pool
            .begin()
            .await
            .map_err(|e| format!("db begin error: {e}"))?;
        let (current_pack_id, current_locale, current_state_json) =
            load_play_row(&mut tx, play_id, player_id, true).await?;
        if current_pack_id != base_pack_id
            || current_locale != base_locale
            || current_state_json != base_state_json
        {
            tx.rollback()
                .await
                .map_err(|e| format!("db rollback error: {e}"))?;
            continue;
        }

        sqlx::query(
            "UPDATE game_plays SET locale = $1, state_json = $2::jsonb, updated_at = NOW() WHERE id = $3 AND player_id = $4",
        )
        .bind(&persisted_locale)
        .bind(&new_state_json)
        .bind(play_id)
        .bind(player_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("db update error: {e}"))?;
        insert_transcript_entries(&mut tx, play_id, turn_number, &transcript_entries).await?;
        tx.commit()
            .await
            .map_err(|e| format!("db commit error: {e}"))?;

        return Ok(result);
    }

    Err("play changed too frequently; please retry".to_string())
}

// ── Public queries ──────────────────────────────────

pub async fn get_play_ui(
    pool: &PgPool,
    play_id: &str,
    player_id: &str,
) -> Result<UiSnapshot, String> {
    let play_id = parse_uuid(play_id, "play id")?;
    let player_id = parse_uuid(player_id, "player id")?;
    let (pack_id, locale, state_json) = load_play_row_unlocked(pool, &play_id, &player_id).await?;
    let transcript_lines = fetch_transcript_lines(pool, &play_id, &player_id).await?;
    let transcript_lines = if transcript_lines.is_empty() {
        transcript_lines_from_state_json(&state_json)?
    } else {
        transcript_lines
    };

    let snapshot = tokio::task::spawn_blocking(move || {
        let content = loader::load_named_pack(&pack_id, Some(&locale))
            .map_err(|e| format!("failed to load pack '{pack_id}' locale '{locale}': {e}"))?;
        let runtime = build_runtime_impl(content, &state_json)?;
        ui::build_ui_snapshot(&runtime, &pack_id, &transcript_lines)
    })
    .await
    .map_err(|e| format!("blocking task panicked: {e:?}"))??;

    Ok(snapshot)
}

pub async fn get_transcript(
    pool: &PgPool,
    play_id: &str,
    player_id: &str,
) -> Result<Vec<cinder_core::engine::narrative::NarrativeLine>, String> {
    let play_id = parse_uuid(play_id, "play id")?;
    let player_id = parse_uuid(player_id, "player id")?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("db begin error: {e}"))?;
    let (_, _, state_json) = load_play_row(&mut tx, &play_id, &player_id, false).await?;
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT te.role, te.text
         FROM transcript_entries te
         JOIN game_plays s ON s.id = te.play_id
         WHERE te.play_id = $1 AND s.player_id = $2
         ORDER BY te.turn_number ASC, te.id ASC",
    )
    .bind(play_id)
    .bind(player_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| format!("transcript query error: {e}"))?;

    if rows.is_empty() {
        let lines = transcript_lines_from_state_json(&state_json)?;
        if !lines.is_empty() {
            replace_transcript_entries_with_lines(&mut tx, &play_id, &lines).await?;
        }
        tx.commit()
            .await
            .map_err(|e| format!("db commit error: {e}"))?;
        return Ok(lines
            .into_iter()
            .map(cinder_core::engine::narrative::NarrativeLine::narration)
            .collect());
    }

    tx.rollback()
        .await
        .map_err(|e| format!("db rollback error: {e}"))?;
    Ok(rows
        .into_iter()
        .map(
            |(role, text)| cinder_core::engine::narrative::NarrativeLine {
                kind: narrative_kind(&role),
                text,
            },
        )
        .collect())
}

fn build_runtime_impl(
    content: cinder_core::content::types::ContentPack,
    state_json: &str,
) -> Result<CinderRuntime, String> {
    if state_json.is_empty() || state_json == "{}" {
        CinderRuntime::new(content, false).map_err(|e| format!("failed to create runtime: {e}"))
    } else {
        let mut state: WorldState = serde_json::from_str(state_json)
            .map_err(|e| format!("failed to deserialize state: {e}"))?;
        let current_room_id = state.current_room_id.clone();
        state.mark_actor_room_visited(&content.settings.combat.player_actor_id, &current_room_id);
        CinderRuntime::from_state(content, state, false)
            .map_err(|e| format!("failed to create runtime from state: {e}"))
    }
}
