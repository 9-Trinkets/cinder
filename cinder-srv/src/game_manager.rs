use cinder_core::content::loader;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{TurnOutcome, WorldState};
use sqlx::PgPool;

mod response;
mod ui;

pub use self::response::{consume_projector_sequence, CommandResponse};
pub use self::ui::UiSnapshot;

// ── Stateless helpers ───────────────────────────────

/// Load WorldState from Postgres, reconstruct CinderRuntime, run `f`,
/// persist the updated state, return `f`'s result.
async fn with_runtime<F, R>(
    pool: &PgPool,
    session_id: &str,
    player_id: &str,
    f: F,
) -> Result<R, String>
where
    F: FnOnce(&CinderRuntime) -> Result<R, String> + Send + 'static,
    R: Send + 'static,
{
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT pack_id, state_json::text FROM game_sessions WHERE id = $1 AND player_id = $2",
    )
    .bind(session_id)
    .bind(player_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("db error: {e}"))?
    .ok_or_else(|| "session not found".to_string())?;

    let (pack_id, state_json) = row;
    let sid = session_id.to_string();

    let (result, new_state_json) = tokio::task::spawn_blocking(move || {
        let content = loader::load_named_pack(&pack_id, None)
            .map_err(|e| format!("failed to load pack '{pack_id}': {e}"))?;

        let runtime = build_runtime_impl(content, &state_json)?;

        let result = f(&runtime)?;

        let new_state = runtime
            .export_state()
            .map_err(|e| format!("state export error: {e}"))?;
        let new_state_json =
            serde_json::to_string(&new_state).map_err(|e| format!("serialization error: {e}"))?;

        Ok::<_, String>((result, new_state_json))
    })
    .await
    .map_err(|e| format!("blocking task panicked: {e:?}"))??;

    sqlx::query(
        "UPDATE game_sessions SET state_json = $1::jsonb, updated_at = NOW() WHERE id = $2",
    )
    .bind(&new_state_json)
    .bind(&sid)
    .execute(pool)
    .await
    .map_err(|e| format!("db update error: {e}"))?;

    Ok(result)
}

/// Load state from Postgres, reconstruct CinderRuntime, run read-only `f`, drop runtime.
/// Does NOT persist state back.
async fn with_runtime_readonly<F, R>(
    pool: &PgPool,
    session_id: &str,
    player_id: &str,
    f: F,
) -> Result<R, String>
where
    F: FnOnce(&CinderRuntime) -> Result<R, String> + Send + 'static,
    R: Send + 'static,
{
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT pack_id, state_json::text FROM game_sessions WHERE id = $1 AND player_id = $2",
    )
    .bind(session_id)
    .bind(player_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("db error: {e}"))?
    .ok_or_else(|| "session not found".to_string())?;

    let (pack_id, state_json) = row;

    tokio::task::spawn_blocking(move || {
        let content = loader::load_named_pack(&pack_id, None)
            .map_err(|e| format!("failed to load pack '{pack_id}': {e}"))?;

                let runtime = build_runtime_impl(content, &state_json)?;

        f(&runtime)


    })
    .await
    .map_err(|e| format!("blocking task panicked: {e:?}"))?
}

// ── Public API ──────────────────────────────────────

pub async fn create_session(pool: &PgPool, player_id: &str, pack_id: &str) -> Result<(String, String, String), String> {
    let content = loader::load_named_pack(pack_id, None)
        .map_err(|e| format!("failed to load pack '{pack_id}': {e}"))?;

    let title = content.opening.title.clone();

    let runtime = CinderRuntime::new(content, false)
        .map_err(|e| format!("failed to create runtime: {e}"))?;

    let intro_text = runtime
        .current_intro_text()
        .map_err(|e| format!("intro text error: {e}"))?;

    let session_id = uuid::Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO game_sessions (id, player_id, pack_id, state_json) VALUES ($1, $2, $3, $4)",
    )
    .bind(&session_id)
    .bind(player_id)
    .bind(pack_id)
    .bind("{}")
    .execute(pool)
    .await
    .map_err(|e| format!("db insert error: {e}"))?;

    Ok((session_id, title, intro_text))
}

pub async fn run_command(
    pool: &PgPool,
    session_id: &str,
    player_id: &str,
    input: &str,
) -> Result<CommandResponse, String> {
    let input = input.to_string();
    with_runtime(pool, session_id, player_id, move |runtime| {
        let mut outcome = runtime
            .run_turn(&input)
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

        let session_feedback = if outcome.game_over {
            response::session_feedback_data(runtime)
        } else {
            None
        };

        outcome = runtime
            .continue_after_game_over(outcome)
            .map_err(|e| format!("appointment rollover error: {e}"))?;

        let _ = runtime.push_transcript_line(&turn_text);

        let movie = consume_projector_sequence(runtime);

        Ok(CommandResponse {
            text: outcome.text,
            game_over: outcome.game_over,
            movie,
            session_feedback,
        })
    })
    .await
}

pub async fn switch_room(
    pool: &PgPool,
    session_id: &str,
    player_id: &str,
    room_id: &str,
) -> Result<TurnOutcome, String> {
    let room_id = room_id.to_string();
    with_runtime(pool, session_id, player_id, move |runtime| {
        let outcome = runtime
            .switch_room_view(&room_id)
            .map_err(|e| format!("room switch error: {e}"))?;
        let _ = runtime.push_transcript_line(&outcome.text);
        Ok(outcome)
    })
    .await
}

pub async fn follow_actor(
    pool: &PgPool,
    session_id: &str,
    player_id: &str,
    actor_id: Option<&str>,
) -> Result<TurnOutcome, String> {
    let actor_id = actor_id.map(|s| s.to_string());
    with_runtime(pool, session_id, player_id, move |runtime| {
        let outcome = runtime
            .follow_actor(actor_id.as_deref())
            .map_err(|e| format!("follow error: {e}"))?;
        let _ = runtime.push_transcript_line(&outcome.text);
        Ok(outcome)
    })
    .await
}

pub async fn set_locale(
    pool: &PgPool,
    session_id: &str,
    player_id: &str,
    locale: &str,
) -> Result<String, String> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT pack_id, state_json::text FROM game_sessions WHERE id = $1 AND player_id = $2",
    )
    .bind(session_id)
    .bind(player_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("db error: {e}"))?
    .ok_or_else(|| "session not found".to_string())?;

    let (pack_id, state_json) = row;
    let locale = locale.to_string();
    let sid = session_id.to_string();

    let (result, new_state_json) = tokio::task::spawn_blocking(move || {
        let content = loader::load_pack_from_dir_with_locale(
            &loader::pack_dir(&pack_id),
            Some(&locale),
        )
        .map_err(|e| format!("failed to load locale '{locale}': {e}"))?;

        let language_name = content.ui_text.language_name.clone();

        let runtime = build_runtime_impl(content, &state_json)?;

        // relocalize_story_vars takes &self (interior mutability)
        runtime
            .relocalize_story_vars()
            .map_err(|e| format!("relocalize error: {e}"))?;

        let changed_text = runtime.content().render_template(
            &runtime.content().ui_text.language_changed_text,
            &[("language_name", language_name.as_str())],
        );

        let new_state = runtime
            .export_state()
            .map_err(|e| format!("state export error: {e}"))?;
        let new_state_json =
            serde_json::to_string(&new_state).map_err(|e| format!("serialization error: {e}"))?;

        Ok::<_, String>((changed_text, new_state_json))
    })
    .await
    .map_err(|e| format!("blocking task panicked: {e:?}"))??;

    sqlx::query(
        "UPDATE game_sessions SET state_json = $1::jsonb, updated_at = NOW() WHERE id = $2",
    )
    .bind(&new_state_json)
    .bind(&sid)
    .execute(pool)
    .await
    .map_err(|e| format!("db update error: {e}"))?;

    Ok(result)
}

pub async fn get_session_ui(
    pool: &PgPool,
    session_id: &str,
    player_id: &str,
) -> Result<UiSnapshot, String> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT pack_id, state_json::text FROM game_sessions WHERE id = $1 AND player_id = $2",
    )
    .bind(session_id)
    .bind(player_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("db error: {e}"))?
    .ok_or_else(|| "session not found".to_string())?;

    let (pack_id, state_json) = row;

    tokio::task::spawn_blocking(move || {
        let content = loader::load_named_pack(&pack_id, None)
            .map_err(|e| format!("failed to load pack '{pack_id}': {e}"))?;

        let runtime = build_runtime_impl(content, &state_json)?;

        ui::build_ui_snapshot(&runtime, &pack_id)
    })
    .await
    .map_err(|e| format!("blocking task panicked: {e:?}"))?
}

pub async fn get_transcript(
    pool: &PgPool,
    session_id: &str,
    player_id: &str,
) -> Result<Vec<String>, String> {
    with_runtime_readonly(pool, session_id, player_id, |runtime| {
        runtime.transcript_lines().map_err(|e| e.to_string())
    })
    .await
}

fn build_runtime_impl(content: cinder_core::content::types::ContentPack, state_json: &str) -> Result<CinderRuntime, String> {
    if state_json.is_empty() || state_json == "{}" {
        CinderRuntime::new(content, false)
            .map_err(|e| format!("failed to create runtime: {e}"))
    } else {
        let state: WorldState = serde_json::from_str(state_json)
            .map_err(|e| format!("failed to deserialize state: {e}"))?;
        CinderRuntime::from_state(content, state, false)
            .map_err(|e| format!("failed to create runtime from state: {e}"))
    }
}
