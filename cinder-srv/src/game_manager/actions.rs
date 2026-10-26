use cinder_core::content::loader;
use cinder_core::engine::narrative::NarrativeLine;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{GamePhase, TurnOutcome};
use sqlx::PgPool;
use uuid::Uuid;

use super::db::{
    MAX_PLAY_WRITE_RETRIES, PendingTranscriptEntry, insert_transcript_entries, load_play_row,
    load_play_row_unlocked, narrative_role, parse_uuid,
};
use super::response::{act_closure_data, game_closure_data};
use super::ui::build_ui_snapshot;
use super::{CommandResponse, build_runtime_impl, consume_projector_sequence, with_runtime};

/// Reads a player's account-level display-language preference, if set.
pub async fn load_player_preferred_locale(
    pool: &PgPool,
    player_id: &Uuid,
) -> Result<Option<String>, String> {
    let preferred = sqlx::query_scalar::<_, Option<String>>(
        "SELECT preferred_locale FROM players WHERE id = $1",
    )
    .bind(player_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("db error: {e}"))?;
    Ok(preferred
        .flatten()
        .filter(|locale| !locale.trim().is_empty()))
}

pub async fn create_play(
    pool: &PgPool,
    player_id: &str,
    pack_id: &str,
    locale: Option<&str>,
) -> Result<(String, String, String), String> {
    let player_id = parse_uuid(player_id, "player id")?;
    let requested = locale
        .map(str::trim)
        .filter(|locale| !locale.is_empty())
        .map(str::to_string);
    let effective_locale = match requested {
        Some(locale) => Some(locale),
        None => load_player_preferred_locale(pool, &player_id).await?,
    };
    let content = loader::load_named_pack(pack_id, effective_locale.as_deref())
        .map_err(|e| format!("failed to load pack '{pack_id}': {e}"))?;

    let title = content.opening.title.clone();
    let locale = content.locale.clone();
    let system_lines = content.opening.system_lines.clone();

    let runtime =
        CinderRuntime::new(content, false).map_err(|e| format!("failed to create runtime: {e}"))?;

    let intro_text = runtime
        .current_intro_text()
        .map_err(|e| format!("intro text error: {e}"))?;
    // Each paragraph is its own transcript line so spoken lines can be styled
    // as dialogue instead of being buried in the scene text.
    let intro_paragraphs = split_paragraphs(&intro_text);
    for paragraph in &intro_paragraphs {
        let _ = runtime.push_transcript_line(paragraph);
    }
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
    let mut transcript_entries: Vec<PendingTranscriptEntry> = intro_paragraphs
        .iter()
        .map(|paragraph| PendingTranscriptEntry {
            role: "narrative".to_string(),
            text: paragraph.clone(),
        })
        .collect();
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
    display: Option<&str>,
) -> Result<CommandResponse, String> {
    let play_id = parse_uuid(play_id, "play id")?;
    let player_id = parse_uuid(player_id, "player id")?;
    let input_owned = input.to_string();
    let transcript_text = transcript_echo(input, display);
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

            // Typed narrative lines for the player's turn. Ticks and act
            // rollover append extra lines to `narrative` that we also surface.
            let mut narrative = outcome.lines.clone();

            let menu_active = runtime
                .export_state()
                .map(|s| s.active_menu_id.is_some())
                .unwrap_or(false);

            if outcome.phase == GamePhase::Active && !menu_active {
                match runtime.run_tick() {
                    Ok(tick) => {
                        narrative.extend(tick.lines);
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
                // The world carries on into the next act; only act-scoped
                // variables are cleared.
                continue_ended_act(runtime, &mut outcome, &mut narrative)?;
            }

            for line in &outcome.lines {
                if !line.text.trim().is_empty() {
                    let _ = runtime.push_transcript_line(&line.text);
                }
            }

            let movie = consume_projector_sequence(runtime);
            let ui_snapshot = build_ui_snapshot(runtime, pack_id, transcript_lines)?;

            let is_game_over = outcome.phase != GamePhase::Active;
            let response = CommandResponse {
                lines: narrative.clone(),
                game_over: is_game_over,
                movie,
                act_closure,
                game_closure,
                ui_snapshot: Some(ui_snapshot),
            };
            let transcript_entries = {
                let mut entries = Vec::new();
                if let Some(text) = &transcript_text {
                    entries.push(PendingTranscriptEntry {
                        role: "player".to_string(),
                        text: text.clone(),
                    });
                }
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
    run_realtime_tick_with_status(pool, play_id, player_id, |_| {}).await
}

pub async fn run_realtime_tick_with_status<F>(
    pool: &PgPool,
    play_id: &str,
    player_id: &str,
    on_speaker: F,
) -> Result<CommandResponse, String>
where
    F: Fn(&str) + Send + Sync + 'static,
{
    let play_id = parse_uuid(play_id, "play id")?;
    let player_id = parse_uuid(player_id, "player id")?;
    with_runtime(
        pool,
        &play_id,
        &player_id,
        move |runtime, pack_id, transcript_lines| {
            if let Some(speaker) = runtime.peek_upcoming_conversational_speaker() {
                on_speaker(&speaker);
            }
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
                // The world carries on into the next act; only act-scoped
                // variables are cleared.
                let lines = runtime
                    .continue_after_act()
                    .map_err(|e| format!("act continuation error: {e}"))?;
                outcome.lines.extend(lines);
                outcome.phase = GamePhase::Active;
            }
            let movie = consume_projector_sequence(runtime);
            let ui_snapshot = build_ui_snapshot(runtime, pack_id, transcript_lines)?;
            let is_game_over = outcome.phase != GamePhase::Active;
            let response = CommandResponse {
                lines: outcome.lines.clone(),
                game_over: is_game_over,
                movie,
                act_closure,
                game_closure,
                ui_snapshot: Some(ui_snapshot),
            };
            let transcript_entries: Vec<PendingTranscriptEntry> = response
                .lines
                .iter()
                .filter(|line| !line.text.trim().is_empty())
                .map(|line| PendingTranscriptEntry {
                    role: narrative_role(&line.kind).to_string(),
                    text: line.text.trim().to_string(),
                })
                .collect();
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
        sqlx::query("UPDATE players SET preferred_locale = $1 WHERE id = $2")
            .bind(&locale)
            .bind(player_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("db update error: {e}"))?;
        tx.commit()
            .await
            .map_err(|e| format!("db commit error: {e}"))?;

        return Ok(CommandResponse {
            lines: vec![cinder_core::engine::narrative::NarrativeLine::system(
                changed_text,
            )],
            game_over: is_game_over,
            act_closure: ui_snapshot.act_closure.clone(),
            game_closure: ui_snapshot.game_closure.clone(),
            ui_snapshot: Some(ui_snapshot),
            ..Default::default()
        });
    }

    Err("play changed too frequently; please retry".to_string())
}

/// Carries the game on into the next act. The lines that act opens with belong
/// to this response, so they go to the player's screen and the stored transcript
/// as well as the outcome.
fn continue_ended_act(
    runtime: &CinderRuntime,
    outcome: &mut TurnOutcome,
    narrative: &mut Vec<NarrativeLine>,
) -> Result<(), String> {
    let lines = runtime
        .continue_after_act()
        .map_err(|e| format!("act continuation error: {e}"))?;
    narrative.extend(lines.iter().cloned());
    outcome.lines.extend(lines);
    outcome.phase = GamePhase::Active;
    Ok(())
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
        move |runtime, pack_id, lines| {
            let _ = runtime
                .continue_after_act()
                .map_err(|e| format!("play continuation error: {e}"))?;
            let ui_snapshot = build_ui_snapshot(runtime, pack_id, lines)?;
            Ok((
                CommandResponse::new(Vec::new(), false, Some(ui_snapshot)),
                Vec::new(),
            ))
        },
    )
    .await
}

/// The line stored for the player's own input. A readable `display` replaces
/// the raw command; an explicitly empty `display` means the content already
/// shows what the player said, so no echo is stored.
fn transcript_echo(input: &str, display: Option<&str>) -> Option<String> {
    match display.map(str::trim) {
        Some("") => None,
        Some(text) if text.chars().count() <= 200 => Some(text.to_string()),
        _ => Some(input.to_string()),
    }
}

/// Splits text on blank lines into trimmed, non-empty paragraphs.
fn split_paragraphs(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(str::trim)
        .filter(|paragraph| !paragraph.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod paragraph_tests {
    use super::{split_paragraphs, transcript_echo};

    #[test]
    fn echo_uses_display_unless_it_is_explicitly_empty() {
        assert_eq!(transcript_echo("2", None).as_deref(), Some("2"));
        assert_eq!(
            transcript_echo("2", Some("Look at Hal")).as_deref(),
            Some("Look at Hal")
        );
        assert_eq!(transcript_echo("2", Some("   ")), None);
        let long = "x".repeat(201);
        assert_eq!(transcript_echo("2", Some(&long)).as_deref(), Some("2"));
    }

    #[test]
    fn paragraphs_split_on_blank_lines_and_drop_empties() {
        assert_eq!(
            split_paragraphs("One.\n\n  Two.  \n\n\n\nThree."),
            vec!["One.", "Two.", "Three."]
        );
        assert_eq!(split_paragraphs("Solo."), vec!["Solo."]);
        assert!(split_paragraphs("  \n\n ").is_empty());
    }
}

#[cfg(test)]
mod act_tests {
    use super::continue_ended_act;
    use cinder_core::content::loader::load_named_pack;
    use cinder_core::engine::runtime::CinderRuntime;
    use cinder_core::engine::state::{GamePhase, WorldState};

    /// Isla's first act by its quickest road, up to the turn that ends it.
    fn turn_that_ends_the_first_act() -> (CinderRuntime, cinder_core::engine::state::TurnOutcome) {
        let pack = load_named_pack("isla", Some("en")).expect("isla loads");
        let state = WorldState::new(&pack);
        let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime");
        let steps = [
            "go inside",
            "go hall",
            "go upstairs",
            "go upstairs",
            "go book room",
            "take fox book",
            "go landing",
            "go downstairs",
            "go downstairs",
            "go waiting room",
            "say wren:greet",
            "meet-probe",
            "go hall",
            "go playroom",
            "bins-probe",
            "snack-none-probe",
            "stories-read",
            "fox-1-probe",
            "closing-probe",
            "go hall",
            "go kitchen",
            "go back yard",
            "phone-1-open",
            "phone-2-next",
            "note-write",
        ];
        for step in steps {
            runtime.run_turn(step).expect(step);
        }
        let last = runtime
            .run_turn("look at notes from today")
            .expect("last turn");
        (runtime, last)
    }

    #[test]
    fn the_next_acts_opening_reaches_the_response_and_the_transcript() {
        let (runtime, mut outcome) = turn_that_ends_the_first_act();
        assert_eq!(outcome.phase, GamePhase::ActEnded);
        let mut narrative = outcome.lines.clone();

        continue_ended_act(&runtime, &mut outcome, &mut narrative).expect("continues");

        assert_eq!(outcome.phase, GamePhase::Active);
        let shown: Vec<&str> = narrative.iter().map(|line| line.text.as_str()).collect();
        let heading = shown
            .iter()
            .position(|text| text.contains("== Front Porch =="))
            .expect("the new act says where Isla is");
        let opening = shown
            .iter()
            .position(|text| text.contains("early February"))
            .expect("the new act's opening is shown");
        assert!(heading < opening, "{shown:?}");
        assert_eq!(
            narrative.len(),
            outcome.lines.len(),
            "what is shown and what is stored are the same lines"
        );
    }
}
