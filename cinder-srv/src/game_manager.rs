use cinder_core::content::loader;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

mod response;
mod ui;

pub use self::response::{CommandResponse, consume_projector_sequence};
pub use self::ui::UiSnapshot;

type SessionRow = (String, String, String);

#[derive(Debug)]
struct PendingTranscriptEntry {
    role: String,
    text: String,
}

async fn load_session_row(
    tx: &mut Transaction<'_, Postgres>,
    session_id: &Uuid,
    player_id: &Uuid,
    for_update: bool,
) -> Result<SessionRow, String> {
    let query = if for_update {
        "SELECT pack_id, locale, state_json::text FROM game_sessions WHERE id = $1 AND player_id = $2 FOR UPDATE"
    } else {
        "SELECT pack_id, locale, state_json::text FROM game_sessions WHERE id = $1 AND player_id = $2"
    };

    sqlx::query_as::<_, SessionRow>(query)
        .bind(session_id)
        .bind(player_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| format!("db error: {e}"))?
        .ok_or_else(|| "session not found".to_string())
}

async fn with_runtime<F, R>(
    pool: &PgPool,
    session_id: &Uuid,
    player_id: &Uuid,
    f: F,
) -> Result<R, String>
where
    F: FnOnce(&CinderRuntime, &str) -> Result<(R, Vec<PendingTranscriptEntry>), String>
        + Send
        + 'static,
    R: Send + 'static,
{
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("db begin error: {e}"))?;
    let (pack_id, locale, state_json) =
        load_session_row(&mut tx, session_id, player_id, true).await?;
    backfill_transcript_if_missing(&mut tx, session_id, &state_json).await?;

    let (result, transcript_entries, persisted_locale, new_state_json, turn_number) =
        tokio::task::spawn_blocking(move || {
            let content = loader::load_named_pack(&pack_id, Some(&locale))
                .map_err(|e| format!("failed to load pack '{pack_id}' locale '{locale}': {e}"))?;

            let runtime = build_runtime_impl(content, &state_json)?;

            let (result, transcript_entries) = f(&runtime, &pack_id)?;

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

    sqlx::query(
        "UPDATE game_sessions SET locale = $1, state_json = $2::jsonb, updated_at = NOW() WHERE id = $3 AND player_id = $4",
    )
    .bind(&persisted_locale)
    .bind(&new_state_json)
    .bind(session_id)
    .bind(player_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("db update error: {e}"))?;
    insert_transcript_entries(&mut tx, session_id, turn_number, &transcript_entries).await?;
    tx.commit()
        .await
        .map_err(|e| format!("db commit error: {e}"))?;

    Ok(result)
}

async fn insert_transcript_entries(
    tx: &mut Transaction<'_, Postgres>,
    session_id: &Uuid,
    turn_number: u32,
    entries: &[PendingTranscriptEntry],
) -> Result<(), String> {
    for entry in entries {
        sqlx::query(
            "INSERT INTO transcript_entries (session_id, turn_number, role, text) VALUES ($1, $2, $3, $4)",
        )
        .bind(session_id)
        .bind(turn_number as i32)
        .bind(&entry.role)
        .bind(&entry.text)
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("transcript insert error: {e}"))?;
    }
    Ok(())
}

fn transcript_lines_from_state_json(state_json: &str) -> Result<Vec<String>, String> {
    if state_json.is_empty() || state_json == "{}" {
        return Ok(Vec::new());
    }
    let state: WorldState = serde_json::from_str(state_json)
        .map_err(|e| format!("failed to deserialize state: {e}"))?;
    Ok(state.transcript)
}

async fn replace_transcript_entries_with_lines(
    tx: &mut Transaction<'_, Postgres>,
    session_id: &Uuid,
    lines: &[String],
) -> Result<(), String> {
    sqlx::query("DELETE FROM transcript_entries WHERE session_id = $1")
        .bind(session_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("transcript delete error: {e}"))?;

    for line in lines {
        sqlx::query(
            "INSERT INTO transcript_entries (session_id, turn_number, role, text) VALUES ($1, $2, $3, $4)",
        )
        .bind(session_id)
        .bind(0_i32)
        .bind("narrative")
        .bind(line)
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("transcript insert error: {e}"))?;
    }
    Ok(())
}

async fn backfill_transcript_if_missing(
    tx: &mut Transaction<'_, Postgres>,
    session_id: &Uuid,
    state_json: &str,
) -> Result<(), String> {
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM transcript_entries WHERE session_id = $1",
    )
    .bind(session_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| format!("transcript count error: {e}"))?;

    if existing > 0 {
        return Ok(());
    }

    let lines = transcript_lines_from_state_json(state_json)?;
    if lines.is_empty() {
        return Ok(());
    }

    replace_transcript_entries_with_lines(tx, session_id, &lines).await
}

fn parse_uuid(value: &str, field: &str) -> Result<Uuid, String> {
    Uuid::parse_str(value).map_err(|e| format!("invalid {field}: {e}"))
}

// ── Public API ──────────────────────────────────────

pub async fn create_session(
    pool: &PgPool,
    player_id: &str,
    pack_id: &str,
) -> Result<(String, String, String), String> {
    let player_id = parse_uuid(player_id, "player id")?;
    let content = loader::load_named_pack(pack_id, None)
        .map_err(|e| format!("failed to load pack '{pack_id}': {e}"))?;

    let title = content.opening.title.clone();
    let locale = content.locale.clone();

    let runtime =
        CinderRuntime::new(content, false).map_err(|e| format!("failed to create runtime: {e}"))?;

    let intro_text = runtime
        .current_intro_text()
        .map_err(|e| format!("intro text error: {e}"))?;
    let _ = runtime.push_transcript_line(&intro_text);
    let initial_state_json = serde_json::to_string(
        &runtime
            .export_state()
            .map_err(|e| format!("state export error: {e}"))?,
    )
    .map_err(|e| format!("serialization error: {e}"))?;

    let session_id = Uuid::new_v4();
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("db begin error: {e}"))?;

    sqlx::query(
        "INSERT INTO game_sessions (id, player_id, pack_id, locale, state_json) VALUES ($1, $2, $3, $4, $5::jsonb)",
    )
    .bind(session_id)
    .bind(player_id)
    .bind(pack_id)
    .bind(&locale)
    .bind(&initial_state_json)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("db insert error: {e}"))?;
    replace_transcript_entries_with_lines(&mut tx, &session_id, &[intro_text.clone()]).await?;
    tx.commit()
        .await
        .map_err(|e| format!("db commit error: {e}"))?;

    Ok((session_id.to_string(), title, intro_text))
}

pub async fn run_command(
    pool: &PgPool,
    session_id: &str,
    player_id: &str,
    input: &str,
) -> Result<CommandResponse, String> {
    let session_id = parse_uuid(session_id, "session id")?;
    let player_id = parse_uuid(player_id, "player id")?;
    let input_owned = input.to_string();
    with_runtime(pool, &session_id, &player_id, move |runtime, pack_id| {
        let mut outcome = runtime
            .run_turn(&input_owned)
            .map_err(|e| format!("turn error: {e}"))?;

        let turn_text = outcome.text.clone();

        if !outcome.game_over {
            match runtime.run_tick() {
                Ok(tick) => {
                    if !tick.text.is_empty() {
                        outcome.text = format!("{}\n\n{}", outcome.text, tick.text);
                    }
                    outcome.game_over = outcome.game_over || tick.game_over;
                }
                Err(e) => return Err(format!("tick error: {e}")),
            }
        }

        let session_closure = if outcome.game_over {
            response::session_closure_data(runtime)
        } else {
            None
        };

        outcome = runtime
            .continue_after_game_over(outcome)
            .map_err(|e| format!("appointment rollover error: {e}"))?;

        let _ = runtime.push_transcript_line(&turn_text);

        let movie = consume_projector_sequence(runtime);
        let ui_snapshot = ui::build_ui_snapshot(runtime, pack_id)?;

        let response = CommandResponse {
            text: outcome.text,
            game_over: outcome.game_over,
            movie,
            session_closure,
            ui_snapshot: Some(ui_snapshot),
        };
        let transcript_entries = vec![
            PendingTranscriptEntry {
                role: "player".to_string(),
                text: input_owned.clone(),
            },
            PendingTranscriptEntry {
                role: "narrative".to_string(),
                text: response.text.clone(),
            },
        ];

        Ok((response, transcript_entries))
    })
    .await
}

pub async fn run_realtime_tick(
    pool: &PgPool,
    session_id: &str,
    player_id: &str,
) -> Result<CommandResponse, String> {
    let session_id = parse_uuid(session_id, "session id")?;
    let player_id = parse_uuid(player_id, "player id")?;
    with_runtime(pool, &session_id, &player_id, move |runtime, pack_id| {
        let outcome = runtime.run_tick().map_err(|e| format!("tick error: {e}"))?;
        let session_closure = if outcome.game_over {
            response::session_closure_data(runtime)
        } else {
            None
        };
        let outcome = runtime
            .continue_after_game_over(outcome)
            .map_err(|e| format!("appointment rollover error: {e}"))?;
        let movie = consume_projector_sequence(runtime);
        let ui_snapshot = ui::build_ui_snapshot(runtime, pack_id)?;
        let response = CommandResponse {
            text: outcome.text.clone(),
            game_over: outcome.game_over,
            movie,
            session_closure,
            ui_snapshot: Some(ui_snapshot),
        };
        let transcript_entries = if response.text.is_empty() {
            Vec::new()
        } else {
            vec![PendingTranscriptEntry {
                role: "narrative".to_string(),
                text: response.text.clone(),
            }]
        };
        Ok((response, transcript_entries))
    })
    .await
}

pub async fn switch_room(
    pool: &PgPool,
    session_id: &str,
    player_id: &str,
    room_id: &str,
) -> Result<CommandResponse, String> {
    let session_id = parse_uuid(session_id, "session id")?;
    let player_id = parse_uuid(player_id, "player id")?;
    let room_id = room_id.to_string();
    with_runtime(pool, &session_id, &player_id, move |runtime, pack_id| {
        let outcome = runtime
            .switch_room_view(&room_id)
            .map_err(|e| format!("room switch error: {e}"))?;
        let _ = runtime.push_transcript_line(&outcome.text);
        let ui_snapshot = ui::build_ui_snapshot(runtime, pack_id)?;
        let transcript_entries = vec![PendingTranscriptEntry {
            role: "narrative".to_string(),
            text: outcome.text.clone(),
        }];
        Ok((
            CommandResponse {
                text: outcome.text,
                game_over: outcome.game_over,
                movie: None,
                session_closure: None,
                ui_snapshot: Some(ui_snapshot),
            },
            transcript_entries,
        ))
    })
    .await
}

pub async fn follow_actor(
    pool: &PgPool,
    session_id: &str,
    player_id: &str,
    actor_id: Option<&str>,
) -> Result<CommandResponse, String> {
    let session_id = parse_uuid(session_id, "session id")?;
    let player_id = parse_uuid(player_id, "player id")?;
    let actor_id = actor_id.map(|s| s.to_string());
    with_runtime(pool, &session_id, &player_id, move |runtime, pack_id| {
        let outcome = runtime
            .follow_actor(actor_id.as_deref())
            .map_err(|e| format!("follow error: {e}"))?;
        let _ = runtime.push_transcript_line(&outcome.text);
        let ui_snapshot = ui::build_ui_snapshot(runtime, pack_id)?;
        let transcript_entries = vec![PendingTranscriptEntry {
            role: "narrative".to_string(),
            text: outcome.text.clone(),
        }];
        Ok((
            CommandResponse {
                text: outcome.text,
                game_over: outcome.game_over,
                movie: None,
                session_closure: None,
                ui_snapshot: Some(ui_snapshot),
            },
            transcript_entries,
        ))
    })
    .await
}

pub async fn set_locale(
    pool: &PgPool,
    session_id: &str,
    player_id: &str,
    locale: &str,
) -> Result<CommandResponse, String> {
    let session_id = parse_uuid(session_id, "session id")?;
    let player_id = parse_uuid(player_id, "player id")?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("db begin error: {e}"))?;
    let (pack_id, _, state_json) = load_session_row(&mut tx, &session_id, &player_id, true).await?;
    let locale = locale.to_string();
    let locale_for_runtime = locale.clone();
    let (changed_text, game_over, ui_snapshot, new_state_json) =
        tokio::task::spawn_blocking(move || {
            let localized_pack = loader::load_pack_from_dir_with_locale(
                &loader::pack_dir(&pack_id),
                Some(&locale_for_runtime),
            )
            .map_err(|e| format!("failed to load locale '{locale_for_runtime}': {e}"))?;
            let language_name = localized_pack.ui_text.language_name.clone();
            let runtime = build_runtime_impl(localized_pack, &state_json)?;
            runtime
                .relocalize_story_vars()
                .map_err(|e| format!("relocalize error: {e}"))?;
            let changed_text = runtime.content().render_template(
                &runtime.content().ui_text.language_changed_text,
                &[("language_name", language_name.as_str())],
            );
            let ui_snapshot = ui::build_ui_snapshot(&runtime, &pack_id)?;
            let new_state = runtime
                .export_state()
                .map_err(|e| format!("state export error: {e}"))?;
            let game_over = new_state.game_over;
            let new_state_json = serde_json::to_string(&new_state)
                .map_err(|e| format!("serialization error: {e}"))?;

            Ok::<_, String>((changed_text, game_over, ui_snapshot, new_state_json))
        })
        .await
        .map_err(|e| format!("blocking task panicked: {e:?}"))??;

    sqlx::query(
        "UPDATE game_sessions SET locale = $1, state_json = $2::jsonb, updated_at = NOW() WHERE id = $3 AND player_id = $4",
    )
    .bind(&locale)
    .bind(&new_state_json)
    .bind(session_id)
    .bind(player_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("db update error: {e}"))?;
    tx.commit()
        .await
        .map_err(|e| format!("db commit error: {e}"))?;

    Ok(CommandResponse {
        text: changed_text,
        game_over,
        movie: None,
        session_closure: ui_snapshot.session_closure.clone(),
        ui_snapshot: Some(ui_snapshot),
    })
}

pub async fn get_session_ui(
    pool: &PgPool,
    session_id: &str,
    player_id: &str,
) -> Result<UiSnapshot, String> {
    let session_id = parse_uuid(session_id, "session id")?;
    let player_id = parse_uuid(player_id, "player id")?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("db begin error: {e}"))?;
    let (pack_id, locale, state_json) =
        load_session_row(&mut tx, &session_id, &player_id, true).await?;

    let (snapshot, persisted_locale, new_state_json) = tokio::task::spawn_blocking(move || {
        let content = loader::load_named_pack(&pack_id, Some(&locale))
            .map_err(|e| format!("failed to load pack '{pack_id}' locale '{locale}': {e}"))?;
        let runtime = build_runtime_impl(content, &state_json)?;
        let snapshot = ui::build_ui_snapshot(&runtime, &pack_id)?;
        let persisted_locale = runtime.content().locale.clone();
        let new_state_json = serde_json::to_string(
            &runtime
                .export_state()
                .map_err(|e| format!("state export error: {e}"))?,
        )
        .map_err(|e| format!("serialization error: {e}"))?;
        Ok::<_, String>((snapshot, persisted_locale, new_state_json))
    })
    .await
    .map_err(|e| format!("blocking task panicked: {e:?}"))??;

    sqlx::query(
        "UPDATE game_sessions SET locale = $1, state_json = $2::jsonb, updated_at = NOW() WHERE id = $3 AND player_id = $4",
    )
    .bind(&persisted_locale)
    .bind(&new_state_json)
    .bind(session_id)
    .bind(player_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("db update error: {e}"))?;
    tx.commit()
        .await
        .map_err(|e| format!("db commit error: {e}"))?;

    Ok(snapshot)
}

pub async fn get_transcript(
    pool: &PgPool,
    session_id: &str,
    player_id: &str,
) -> Result<Vec<String>, String> {
    let session_id = parse_uuid(session_id, "session id")?;
    let player_id = parse_uuid(player_id, "player id")?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("db begin error: {e}"))?;
    let (_, _, state_json) = load_session_row(&mut tx, &session_id, &player_id, false).await?;
    let rows = sqlx::query_scalar::<_, String>(
        "SELECT CASE WHEN te.role = 'player' THEN '> ' || te.text ELSE te.text END
         FROM transcript_entries te
         JOIN game_sessions s ON s.id = te.session_id
         WHERE te.session_id = $1 AND s.player_id = $2
         ORDER BY te.turn_number ASC, te.id ASC",
    )
    .bind(session_id)
    .bind(player_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| format!("transcript query error: {e}"))?;

    if rows.is_empty() {
        let lines = transcript_lines_from_state_json(&state_json)?;
        if !lines.is_empty() {
            replace_transcript_entries_with_lines(&mut tx, &session_id, &lines).await?;
        }
        tx.commit()
            .await
            .map_err(|e| format!("db commit error: {e}"))?;
        return Ok(lines);
    }

    tx.rollback()
        .await
        .map_err(|e| format!("db rollback error: {e}"))?;
    Ok(rows)
}

fn build_runtime_impl(
    content: cinder_core::content::types::ContentPack,
    state_json: &str,
) -> Result<CinderRuntime, String> {
    if state_json.is_empty() || state_json == "{}" {
        CinderRuntime::new(content, false).map_err(|e| format!("failed to create runtime: {e}"))
    } else {
        let state: WorldState = serde_json::from_str(state_json)
            .map_err(|e| format!("failed to deserialize state: {e}"))?;
        CinderRuntime::from_state(content, state, false)
            .map_err(|e| format!("failed to create runtime from state: {e}"))
    }
}
