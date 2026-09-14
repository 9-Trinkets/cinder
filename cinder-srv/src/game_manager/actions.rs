use cinder_core::content::loader;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::GamePhase;
use sqlx::PgPool;
use uuid::Uuid;

use super::db::{
    insert_transcript_entries, load_play_row, load_play_row_unlocked, narrative_role, parse_uuid,
    PendingTranscriptEntry, MAX_PLAY_WRITE_RETRIES,
};
use super::response::{act_closure_data, game_closure_data};
use super::ui::build_ui_snapshot;
use super::{CommandResponse, build_runtime_impl, consume_projector_sequence, with_runtime};

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
    let system_lines = content.opening.system_lines.clone();

    let runtime =
        CinderRuntime::new(content, false).map_err(|e| format!("failed to create runtime: {e}"))?;

    let intro_text = runtime
        .current_intro_text()
        .map_err(|e| format!("intro text error: {e}"))?;
    let _ = runtime.push_transcript_line(&intro_text);
    for system_line in &system_lines {
        let _ = runtime.push_transcript_line(system_line);
    }
    let scripted_lines = runtime
        .drain_scripted_sequences()
        .map_err(|e| format!("opening sequence error: {e}"))?;
    for line in scripted_lines.iter() {
        let _ = runtime.push_transcript_line(&line.text);
    }
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
    let mut transcript_entries = vec![PendingTranscriptEntry {
        role: "narrative".to_string(),
        text: intro_text.clone(),
    }];
    transcript_entries.extend(
        system_lines
            .iter()
            .map(|system_line| PendingTranscriptEntry {
                role: "system".to_string(),
                text: system_line.clone(),
            }),
    );
    transcript_entries.extend(scripted_lines.iter().map(|line| PendingTranscriptEntry {
        role: narrative_role(&line.kind).to_string(),
        text: line.text.clone(),
    }));
    insert_transcript_entries(&mut tx, &play_id, 0, &transcript_entries).await?;
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
    let play_id = parse_uuid(play_id, "play id")?;
    let player_id = parse_uuid(player_id, "player id")?;
    let input_owned = input.to_string();
    with_runtime(
        pool,
        &play_id,
        &player_id,
        move |runtime, pack_id, transcript_lines| {
            // A command sent while a game-over already committed (e.g. a
            // queued action racing on the network) must not mutate the ended
            // play; return the ending snapshot as-is.
            if runtime
                .export_state()
                .map(|state| state.phase == GamePhase::GameEnded)
                .unwrap_or(false)
            {
                let ui_snapshot = build_ui_snapshot(runtime, pack_id, transcript_lines)?;
                let response = CommandResponse {
                    text: String::new(),
                    game_over: true,
                    game_closure: ui_snapshot.game_closure.clone(),
                    ui_snapshot: Some(ui_snapshot),
                    ..Default::default()
                };
                return Ok((response, Vec::new()));
            }
            let mut outcome = runtime
                .run_turn(&input_owned)
                .map_err(|e| format!("turn error: {e}"))?;

            let turn_text = outcome.text.clone();
            // Typed narrative lines for the player's turn. Ticks and act
            // rollover append extra prose to `text` that we also surface.
            let mut narrative = outcome.lines.clone();
            let mut extra_text: Vec<String> = Vec::new();

            let menu_active = runtime
                .export_state()
                .map(|s| s.active_menu_id.is_some())
                .unwrap_or(false);

            if outcome.phase == GamePhase::Active && !menu_active {
                match runtime.run_tick() {
                    Ok(tick) => {
                        if !tick.text.is_empty() {
                            outcome.text = format!("{}\n\n{}", outcome.text, tick.text);
                            extra_text.push(tick.text);
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
                act_closure_data(runtime, transcript_lines)
            } else {
                None
            };
            let game_closure = if outcome.phase == GamePhase::GameEnded {
                game_closure_data(runtime, transcript_lines)
            } else {
                None
            };

            if outcome.phase == GamePhase::ActEnded {
                if let Some(intro_text) = runtime
                    .advance_act()
                    .map_err(|e| format!("act rollover error: {e}"))?
                    && !intro_text.is_empty()
                {
                    outcome.text = format!("{}\n\n{}", outcome.text, intro_text);
                    extra_text.push(intro_text);
                }
                outcome.phase = GamePhase::Active;
            }

            let _ = runtime.push_transcript_line(&turn_text);

            let movie = consume_projector_sequence(runtime);
            let ui_snapshot = build_ui_snapshot(runtime, pack_id, transcript_lines)?;

            let is_game_over = outcome.phase != GamePhase::Active;
            for extra in extra_text {
                narrative.push(cinder_core::engine::narrative::NarrativeLine::narration(
                    extra,
                ));
            }
            let response = CommandResponse {
                text: outcome.text,
                lines: narrative.clone(),
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
                for line in &narrative {
                    let trimmed = line.text.trim();
                    if !trimmed.is_empty() {
                        entries.push(PendingTranscriptEntry {
                            role: narrative_role(&line.kind).to_string(),
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
    let play_id = parse_uuid(play_id, "play id")?;
    let player_id = parse_uuid(player_id, "player id")?;
    with_runtime(
        pool,
        &play_id,
        &player_id,
        move |runtime, pack_id, transcript_lines| {
            let mut outcome = runtime.run_tick().map_err(|e| format!("tick error: {e}"))?;
            let act_closure = if outcome.phase == GamePhase::ActEnded
                && runtime.content().settings.show_act_closure
            {
                act_closure_data(runtime, transcript_lines)
            } else {
                None
            };
            let game_closure = if outcome.phase == GamePhase::GameEnded {
                game_closure_data(runtime, transcript_lines)
            } else {
                None
            };
            if outcome.phase == GamePhase::ActEnded {
                if let Some(intro_text) = runtime
                    .advance_act()
                    .map_err(|e| format!("act rollover error: {e}"))?
                    && !intro_text.is_empty()
                {
                    outcome.text = format!("{}\n\n{}", outcome.text, intro_text);
                }
                outcome.phase = GamePhase::Active;
            }
            let movie = consume_projector_sequence(runtime);
            let ui_snapshot = build_ui_snapshot(runtime, pack_id, transcript_lines)?;
            let is_game_over = outcome.phase != GamePhase::Active;
            let response = CommandResponse {
                text: outcome.text.clone(),
                game_over: is_game_over,
                movie,
                act_closure,
                game_closure,
                ui_snapshot: Some(ui_snapshot),
                ..Default::default()
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
    let play_id = parse_uuid(play_id, "play id")?;
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
            let ui_snapshot = build_ui_snapshot(runtime, pack_id, _transcript_lines)?;
            let transcript_entries = if outcome.lines.is_empty() {
                vec![PendingTranscriptEntry {
                    role: "narrative".to_string(),
                    text: outcome.text.clone(),
                }]
            } else {
                outcome
                    .lines
                    .iter()
                    .map(|line| PendingTranscriptEntry {
                        role: narrative_role(&line.kind).to_string(),
                        text: line.text.clone(),
                    })
                    .collect()
            };
            let mut response = CommandResponse::new(
                outcome.text,
                outcome.phase != GamePhase::Active,
                Some(ui_snapshot),
            );
            response.lines = outcome.lines.to_vec();
            Ok((response, transcript_entries))
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
    let play_id = parse_uuid(play_id, "play id")?;
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
            let ui_snapshot = build_ui_snapshot(runtime, pack_id, _transcript_lines)?;
            let transcript_entries = if outcome.lines.is_empty() {
                vec![PendingTranscriptEntry {
                    role: "narrative".to_string(),
                    text: outcome.text.clone(),
                }]
            } else {
                outcome
                    .lines
                    .iter()
                    .map(|line| PendingTranscriptEntry {
                        role: narrative_role(&line.kind).to_string(),
                        text: line.text.clone(),
                    })
                    .collect()
            };
            let mut response = CommandResponse::new(
                outcome.text,
                outcome.phase != GamePhase::Active,
                Some(ui_snapshot),
            );
            response.lines = outcome.lines.to_vec();
            Ok((response, transcript_entries))
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
    let play_id = parse_uuid(play_id, "play id")?;
    let player_id = parse_uuid(player_id, "player id")?;
    let locale = locale.to_string();

    for _attempt in 0..MAX_PLAY_WRITE_RETRIES {
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
                let ui_snapshot = build_ui_snapshot(&runtime, &pack_id, &[])?;
                let new_state = runtime
                    .export_state()
                    .map_err(|e| format!("state export error: {e}"))?;
                let is_game_over = new_state.phase != GamePhase::Active;
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
            act_closure: ui_snapshot.act_closure.clone(),
            game_closure: ui_snapshot.game_closure.clone(),
            ui_snapshot: Some(ui_snapshot),
            ..Default::default()
        });
    }

    Err("play changed too frequently; please retry".to_string())
}

pub async fn continue_play(
    pool: &PgPool,
    play_id: &str,
    player_id: &str,
) -> Result<CommandResponse, String> {
    let play_id = parse_uuid(play_id, "play id")?;
    let player_id = parse_uuid(player_id, "player id")?;
    with_runtime(
        pool,
        &play_id,
        &player_id,
        move |runtime, pack_id, _transcript_lines| {
            runtime
                .continue_after_act()
                .map_err(|e| format!("play continuation error: {e}"))?;
            let ui_snapshot = build_ui_snapshot(runtime, pack_id, _transcript_lines)?;
            Ok((
                CommandResponse::new(String::new(), false, Some(ui_snapshot)),
                Vec::new(),
            ))
        },
    )
    .await
}
