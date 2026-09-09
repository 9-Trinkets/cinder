use cinder_core::engine::state::WorldState;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

pub(super) type SessionRow = (String, String, String);
pub(super) const MAX_TRANSCRIPT_LINES: i64 = 200;
pub(super) const MAX_SESSION_WRITE_RETRIES: usize = 5;

#[derive(Debug)]
pub(super) struct PendingTranscriptEntry {
    pub role: String,
    pub text: String,
}

/// DB `role` value for a narrative line kind. "player" lines carry the echoed
/// command; everything else uses the kind's snake_case name so reloaded
/// history can recover the styling.
pub(super) fn narrative_role(kind: &cinder_core::engine::narrative::NarrativeLineKind) -> &'static str {
    match kind {
        cinder_core::engine::narrative::NarrativeLineKind::Player => "player",
        cinder_core::engine::narrative::NarrativeLineKind::Heading => "heading",
        cinder_core::engine::narrative::NarrativeLineKind::Error => "error",
        cinder_core::engine::narrative::NarrativeLineKind::Narration => "narrative",
        cinder_core::engine::narrative::NarrativeLineKind::System => "system",
        cinder_core::engine::narrative::NarrativeLineKind::Channel => "channel",
    }
}

pub(super) fn narrative_kind(role: &str) -> cinder_core::engine::narrative::NarrativeLineKind {
    match role {
        "player" => cinder_core::engine::narrative::NarrativeLineKind::Player,
        "heading" => cinder_core::engine::narrative::NarrativeLineKind::Heading,
        "error" => cinder_core::engine::narrative::NarrativeLineKind::Error,
        "system" => cinder_core::engine::narrative::NarrativeLineKind::System,
        "channel" => cinder_core::engine::narrative::NarrativeLineKind::Channel,
        _ => cinder_core::engine::narrative::NarrativeLineKind::Narration,
    }
}

pub(super) async fn load_play_row(
    tx: &mut Transaction<'_, Postgres>,
    play_id: &Uuid,
    player_id: &Uuid,
    for_update: bool,
) -> Result<SessionRow, String> {
    let query = if for_update {
        "SELECT pack_id, locale, state_json::text FROM game_plays WHERE id = $1 AND player_id = $2 FOR UPDATE"
    } else {
        "SELECT pack_id, locale, state_json::text FROM game_plays WHERE id = $1 AND player_id = $2"
    };

    sqlx::query_as::<_, SessionRow>(query)
        .bind(play_id)
        .bind(player_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| format!("db error: {e}"))?
        .ok_or_else(|| "session not found".to_string())
}

pub(super) async fn load_play_row_unlocked(
    pool: &PgPool,
    play_id: &Uuid,
    player_id: &Uuid,
) -> Result<SessionRow, String> {
    sqlx::query_as::<_, SessionRow>(
        "SELECT pack_id, locale, state_json::text FROM game_plays WHERE id = $1 AND player_id = $2",
    )
    .bind(play_id)
    .bind(player_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("db error: {e}"))?
    .ok_or_else(|| "session not found".to_string())
}

pub(super) async fn fetch_transcript_lines(
    pool: &PgPool,
    play_id: &Uuid,
    player_id: &Uuid,
) -> Result<Vec<String>, String> {
    sqlx::query_scalar::<_, String>(
        "SELECT CASE WHEN te.role = 'player' THEN '> ' || te.text ELSE te.text END
         FROM transcript_entries te
         JOIN game_plays s ON s.id = te.play_id
         WHERE te.play_id = $1 AND s.player_id = $2
         ORDER BY te.turn_number ASC, te.id ASC
         LIMIT $3",
    )
    .bind(play_id)
    .bind(player_id)
    .bind(MAX_TRANSCRIPT_LINES)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("transcript query error: {e}"))
}

pub(super) async fn insert_transcript_entries(
    tx: &mut Transaction<'_, Postgres>,
    play_id: &Uuid,
    turn_number: u32,
    entries: &[PendingTranscriptEntry],
) -> Result<(), String> {
    for entry in entries {
        sqlx::query(
            "INSERT INTO transcript_entries (play_id, turn_number, role, text) VALUES ($1, $2, $3, $4)",
        )
        .bind(play_id)
        .bind(turn_number as i32)
        .bind(&entry.role)
        .bind(&entry.text)
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("transcript insert error: {e}"))?;
    }
    Ok(())
}

pub(super) fn transcript_lines_from_state_json(state_json: &str) -> Result<Vec<String>, String> {
    if state_json.is_empty() || state_json == "{}" {
        return Ok(Vec::new());
    }
    let state: WorldState = serde_json::from_str(state_json)
        .map_err(|e| format!("failed to deserialize state: {e}"))?;
    Ok(state.transcript)
}

pub(super) async fn replace_transcript_entries_with_lines(
    tx: &mut Transaction<'_, Postgres>,
    play_id: &Uuid,
    lines: &[String],
) -> Result<(), String> {
    sqlx::query("DELETE FROM transcript_entries WHERE play_id = $1")
        .bind(play_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("transcript delete error: {e}"))?;

    for line in lines {
        sqlx::query(
            "INSERT INTO transcript_entries (play_id, turn_number, role, text) VALUES ($1, $2, $3, $4)",
        )
        .bind(play_id)
        .bind(0_i32)
        .bind("narrative")
        .bind(line)
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("transcript insert error: {e}"))?;
    }
    Ok(())
}

pub(super) fn parse_uuid(value: &str, field: &str) -> Result<Uuid, String> {
    Uuid::parse_str(value).map_err(|e| format!("invalid {field}: {e}"))
}
