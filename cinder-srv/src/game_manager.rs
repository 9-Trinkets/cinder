use cinder_core::content::loader;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;
use sqlx::{PgPool, Postgres, Transaction};
use std::sync::Arc;
use uuid::Uuid;

mod response;
mod ui;

pub use self::response::{CommandResponse, consume_projector_sequence};
pub use self::ui::UiSnapshot;

type SessionRow = (String, String, String);
const MAX_TRANSCRIPT_LINES: i64 = 200;
const MAX_SESSION_WRITE_RETRIES: usize = 5;

#[derive(Debug)]
struct PendingTranscriptEntry {
    role: String,
    text: String,
}

async fn load_play_row(
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

async fn load_play_row_unlocked(
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

async fn fetch_transcript_lines(
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

    for _attempt in 0..MAX_SESSION_WRITE_RETRIES {
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

    Err("session changed too frequently; please retry".to_string())
}

async fn insert_transcript_entries(
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

fn parse_uuid(value: &str, field: &str) -> Result<Uuid, String> {
    Uuid::parse_str(value).map_err(|e| format!("invalid {field}: {e}"))
}

// ── Public API ──────────────────────────────────────

pub async fn create_play(
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

    let play_id = Uuid::new_v4();
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("db begin error: {e}"))?;

    sqlx::query(
        "INSERT INTO game_plays (id, player_id, pack_id, locale, state_json) VALUES ($1, $2, $3, $4, $5::jsonb)",
    )
    .bind(play_id)
    .bind(player_id)
    .bind(pack_id)
    .bind(&locale)
    .bind(&initial_state_json)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("db insert error: {e}"))?;
    replace_transcript_entries_with_lines(&mut tx, &play_id, &[intro_text.clone()]).await?;
    tx.commit()
        .await
        .map_err(|e| format!("db commit error: {e}"))?;

    Ok((play_id.to_string(), title, intro_text))
}

pub async fn run_command(
    pool: &PgPool,
    play_id: &str,
    player_id: &str,
    input: &str,
) -> Result<CommandResponse, String> {
    let play_id = parse_uuid(play_id, "session id")?;
    let player_id = parse_uuid(player_id, "player id")?;
    let input_owned = input.to_string();
    with_runtime(
        pool,
        &play_id,
        &player_id,
        move |runtime, pack_id, transcript_lines| {
            let mut outcome = runtime
                .run_turn(&input_owned)
                .map_err(|e| format!("turn error: {e}"))?;

            let turn_text = outcome.text.clone();

            let menu_active = runtime
                .export_state()
                .map(|s| s.active_menu_id.is_some())
                .unwrap_or(false);

            use cinder_core::engine::state::GamePhase;
            if outcome.phase == GamePhase::Active && !menu_active {
                match runtime.run_tick() {
                    Ok(tick) => {
                        if !tick.text.is_empty() {
                            outcome.text = format!("{}\n\n{}", outcome.text, tick.text);
                        }
                        if tick.phase != GamePhase::Active {
                            outcome.phase = tick.phase;
                        }
                    }
                    Err(e) => return Err(format!("tick error: {e}")),
                }
            }

            let act_closure = if outcome.phase == GamePhase::ActEnded
                && runtime.content().settings.show_act_closure
            {
                response::act_closure_data(runtime, transcript_lines)
            } else {
                None
            };
            let game_closure = if outcome.phase == GamePhase::GameEnded {
                response::game_closure_data(runtime, transcript_lines)
            } else {
                None
            };

            if outcome.phase == GamePhase::ActEnded {
                if let Some(intro_text) = runtime
                    .advance_act()
                    .map_err(|e| format!("act rollover error: {e}"))?
                {
                    if !intro_text.is_empty() {
                        outcome.text = format!("{}\n\n{}", outcome.text, intro_text);
                    }
                }
                outcome.phase = GamePhase::Active;
            }

            let _ = runtime.push_transcript_line(&turn_text);

            let movie = consume_projector_sequence(runtime);
            let ui_snapshot = ui::build_ui_snapshot(runtime, pack_id, transcript_lines)?;

            let is_game_over = outcome.phase != GamePhase::Active;
            let response = CommandResponse {
                text: outcome.text,
                game_over: is_game_over,
                movie,
                act_closure,
                game_closure,
                ui_snapshot: Some(ui_snapshot),
            };
            let transcript_entries = {
                let mut entries = vec![PendingTranscriptEntry {
                    role: "player".to_string(),
                    text: input_owned.clone(),
                }];
                for line in response.text.split("\n\n") {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        entries.push(PendingTranscriptEntry {
                            role: "narrative".to_string(),
                            text: trimmed.to_string(),
                        });
                    }
                }
                entries
            };

            Ok((response, transcript_entries))
        },
    )
    .await
}

pub async fn run_realtime_tick(
    pool: &PgPool,
    play_id: &str,
    player_id: &str,
) -> Result<CommandResponse, String> {
    let play_id = parse_uuid(play_id, "session id")?;
    let player_id = parse_uuid(player_id, "player id")?;
    with_runtime(
        pool,
        &play_id,
        &player_id,
        move |runtime, pack_id, transcript_lines| {
            let mut outcome = runtime.run_tick().map_err(|e| format!("tick error: {e}"))?;
            use cinder_core::engine::state::GamePhase;
            let act_closure = if outcome.phase == GamePhase::ActEnded
                && runtime.content().settings.show_act_closure
            {
                response::act_closure_data(runtime, transcript_lines)
            } else {
                None
            };
            let game_closure = if outcome.phase == GamePhase::GameEnded {
                response::game_closure_data(runtime, transcript_lines)
            } else {
                None
            };
            if outcome.phase == GamePhase::ActEnded {
                if let Some(intro_text) = runtime
                    .advance_act()
                    .map_err(|e| format!("act rollover error: {e}"))?
                {
                    if !intro_text.is_empty() {
                        outcome.text = format!("{}\n\n{}", outcome.text, intro_text);
                    }
                }
                outcome.phase = GamePhase::Active;
            }
            let movie = consume_projector_sequence(runtime);
            let ui_snapshot = ui::build_ui_snapshot(runtime, pack_id, transcript_lines)?;
            let is_game_over = outcome.phase != GamePhase::Active;
            let response = CommandResponse {
                text: outcome.text.clone(),
                game_over: is_game_over,
                movie,
                act_closure,
                game_closure,
                ui_snapshot: Some(ui_snapshot),
            };
            let transcript_entries: Vec<PendingTranscriptEntry> = response
                .text
                .split("\n\n")
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .map(|line| PendingTranscriptEntry {
                    role: "narrative".to_string(),
                    text: line.to_string(),
                })
                .collect();
            Ok((response, transcript_entries))
        },
    )
    .await
}

pub async fn switch_room(
    pool: &PgPool,
    play_id: &str,
    player_id: &str,
    room_id: &str,
) -> Result<CommandResponse, String> {
    let play_id = parse_uuid(play_id, "session id")?;
    let player_id = parse_uuid(player_id, "player id")?;
    let room_id = room_id.to_string();
    with_runtime(
        pool,
        &play_id,
        &player_id,
        move |runtime, pack_id, _transcript_lines| {
            let outcome = runtime
                .switch_room_view(&room_id)
                .map_err(|e| format!("room switch error: {e}"))?;
            let _ = runtime.push_transcript_line(&outcome.text);
            let ui_snapshot = ui::build_ui_snapshot(runtime, pack_id, _transcript_lines)?;
            let transcript_entries = vec![PendingTranscriptEntry {
                role: "narrative".to_string(),
                text: outcome.text.clone(),
            }];
            Ok((
                CommandResponse {
                    text: outcome.text,
                    game_over: outcome.phase != cinder_core::engine::state::GamePhase::Active,
                    movie: None,
                    act_closure: None,
                    game_closure: None,
                    ui_snapshot: Some(ui_snapshot),
                },
                transcript_entries,
            ))
        },
    )
    .await
}

pub async fn follow_actor(
    pool: &PgPool,
    play_id: &str,
    player_id: &str,
    actor_id: Option<&str>,
) -> Result<CommandResponse, String> {
    let play_id = parse_uuid(play_id, "session id")?;
    let player_id = parse_uuid(player_id, "player id")?;
    let actor_id = actor_id.map(|s| s.to_string());
    with_runtime(
        pool,
        &play_id,
        &player_id,
        move |runtime, pack_id, _transcript_lines| {
            let outcome = runtime
                .follow_actor(actor_id.as_deref())
                .map_err(|e| format!("follow error: {e}"))?;
            let _ = runtime.push_transcript_line(&outcome.text);
            let ui_snapshot = ui::build_ui_snapshot(runtime, pack_id, _transcript_lines)?;
            let transcript_entries = vec![PendingTranscriptEntry {
                role: "narrative".to_string(),
                text: outcome.text.clone(),
            }];
            Ok((
                CommandResponse {
                    text: outcome.text,
                    game_over: outcome.phase != cinder_core::engine::state::GamePhase::Active,
                    movie: None,
                    act_closure: None,
                    game_closure: None,
                    ui_snapshot: Some(ui_snapshot),
                },
                transcript_entries,
            ))
        },
    )
    .await
}

pub async fn set_locale(
    pool: &PgPool,
    play_id: &str,
    player_id: &str,
    locale: &str,
) -> Result<CommandResponse, String> {
    let play_id = parse_uuid(play_id, "session id")?;
    let player_id = parse_uuid(player_id, "player id")?;
    let locale = locale.to_string();

    for _attempt in 0..MAX_SESSION_WRITE_RETRIES {
        let (pack_id, previous_locale, state_json) =
            load_play_row_unlocked(pool, &play_id, &player_id).await?;
        let target_locale = locale.clone();
        let base_pack_id = pack_id.clone();
        let base_state_json = state_json.clone();
        let base_locale = previous_locale.clone();
        let (changed_text, is_game_over, ui_snapshot, new_state_json) =
            tokio::task::spawn_blocking(move || {
                let localized_pack = loader::load_pack_from_dir_with_locale(
                    &loader::pack_dir(&pack_id),
                    Some(&target_locale),
                )
                .map_err(|e| format!("failed to load locale '{target_locale}': {e}"))?;
                let language_name = localized_pack.ui_text.language_name.clone();
                let runtime = build_runtime_impl(localized_pack, &state_json)?;
                runtime
                    .relocalize_story_vars()
                    .map_err(|e| format!("relocalize error: {e}"))?;
                let changed_text = runtime.content().render_template(
                    &runtime.content().ui_text.language_changed_text,
                    &[("language_name", language_name.as_str())],
                );
                let ui_snapshot = ui::build_ui_snapshot(&runtime, &pack_id, &[])?;
                let new_state = runtime
                    .export_state()
                    .map_err(|e| format!("state export error: {e}"))?;
                let is_game_over = new_state.phase != cinder_core::engine::state::GamePhase::Active;
                let new_state_json = serde_json::to_string(&new_state)
                    .map_err(|e| format!("serialization error: {e}"))?;

                Ok::<_, String>((changed_text, is_game_over, ui_snapshot, new_state_json))
            })
            .await
            .map_err(|e| format!("blocking task panicked: {e:?}"))??;

        let mut tx = pool
            .begin()
            .await
            .map_err(|e| format!("db begin error: {e}"))?;
        let (current_pack_id, current_locale, current_state_json) =
            load_play_row(&mut tx, &play_id, &player_id, true).await?;
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
        .bind(&locale)
        .bind(&new_state_json)
        .bind(play_id)
        .bind(player_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("db update error: {e}"))?;
        tx.commit()
            .await
            .map_err(|e| format!("db commit error: {e}"))?;

        return Ok(CommandResponse {
            text: changed_text,
            game_over: is_game_over,
            movie: None,
            act_closure: ui_snapshot.act_closure.clone(),
            game_closure: ui_snapshot.game_closure.clone(),
            ui_snapshot: Some(ui_snapshot),
        });
    }

    Err("session changed too frequently; please retry".to_string())
}

pub async fn get_play_ui(
    pool: &PgPool,
    play_id: &str,
    player_id: &str,
) -> Result<UiSnapshot, String> {
    let play_id = parse_uuid(play_id, "session id")?;
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
) -> Result<Vec<String>, String> {
    let play_id = parse_uuid(play_id, "session id")?;
    let player_id = parse_uuid(player_id, "player id")?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("db begin error: {e}"))?;
    let (_, _, state_json) = load_play_row(&mut tx, &play_id, &player_id, false).await?;
    let rows = sqlx::query_scalar::<_, String>(
        "SELECT CASE WHEN te.role = 'player' THEN '> ' || te.text ELSE te.text END
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
        return Ok(lines);
    }

    tx.rollback()
        .await
        .map_err(|e| format!("db rollback error: {e}"))?;
    Ok(rows)
}

pub async fn play_id(
    pool: &PgPool,
    play_id: &str,
    player_id: &str,
) -> Result<CommandResponse, String> {
    let play_id = parse_uuid(play_id, "session id")?;
    let player_id = parse_uuid(player_id, "player id")?;
    with_runtime(
        pool,
        &play_id,
        &player_id,
        move |runtime, pack_id, _transcript_lines| {
            runtime
                .continue_after_act()
                .map_err(|e| format!("session continuation error: {e}"))?;
            let ui_snapshot = ui::build_ui_snapshot(runtime, pack_id, _transcript_lines)?;
            Ok((
                CommandResponse {
                    text: String::new(),
                    game_over: false,
                    movie: None,
                    act_closure: None,
                    game_closure: None,
                    ui_snapshot: Some(ui_snapshot),
                },
                Vec::new(),
            ))
        },
    )
    .await
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
