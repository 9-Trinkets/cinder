//! Debug utility for inspecting persisted playthroughs from Postgres.
//!
//! Typical local usage:
//!
//! ```bash
//! cargo run -p cinder-srv --bin debug_playthrough -- --pack aera --transcript-limit 20
//! ```
//!
//! If Postgres is only exposed inside Docker Compose, run it on the Compose network:
//!
//! ```bash
//! docker run --rm --network cinder_default \
//!   -v "$PWD":/work -w /work \
//!   -e CINDER_DATABASE_URL='postgres://cinder:cinder@postgres:5432/cinder' \
//!   rust:1-bookworm \
//!   cargo run -p cinder-srv --bin debug_playthrough -- --pack aera --transcript-limit 20
//! ```
//!
//! Useful filters:
//! - `--play-id <uuid>` to inspect one exact playthrough
//! - `--player <username>` to select the latest play for one player
//! - `--pack <pack_id>` to select the latest play for one content pack
//! - `--transcript-limit <n>` to control transcript length

use cinder_core::engine::state::WorldState;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::collections::BTreeMap;
use std::time::Duration;

const DEFAULT_TRANSCRIPT_LIMIT: i64 = 120;
const RULE_BUNDLE_PREFIX: &str = "rule_bundle:";

#[derive(Debug, Default, PartialEq, Eq)]
struct CliArgs {
    play_id: Option<String>,
    player: Option<String>,
    pack: Option<String>,
    transcript_limit: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct PlayRow {
    play_id: String,
    username: String,
    pack_id: String,
    locale: String,
    created_at: String,
    updated_at: String,
    state_json: String,
}

#[derive(Debug, sqlx::FromRow)]
struct TranscriptRow {
    turn_number: i32,
    role: String,
    text: String,
    created_at: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let args = parse_args(std::env::args().skip(1).collect())?;
    let pool = init_pool().await?;

    let play = fetch_play(&pool, &args).await?;
    let transcript = fetch_transcript(&pool, &play.play_id, args.transcript_limit).await?;
    let state: WorldState = serde_json::from_str(&play.state_json)?;

    print_summary(&play, &state, &transcript);
    Ok(())
}

async fn init_pool() -> Result<PgPool, Box<dyn std::error::Error>> {
    let database_url = std::env::var("CINDER_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost:5432/cinder".to_string());
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&database_url)
        .await
        .map_err(|error| format!("failed to connect to database: {error}").into())
}

fn parse_args(args: Vec<String>) -> Result<CliArgs, String> {
    let mut cli = CliArgs {
        transcript_limit: DEFAULT_TRANSCRIPT_LIMIT,
        ..CliArgs::default()
    };
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--play-id" => cli.play_id = Some(next_value(&mut iter, "--play-id")?),
            "--player" => cli.player = Some(next_value(&mut iter, "--player")?),
            "--pack" => cli.pack = Some(next_value(&mut iter, "--pack")?),
            "--transcript-limit" => {
                let value = next_value(&mut iter, "--transcript-limit")?;
                cli.transcript_limit = value
                    .parse::<i64>()
                    .map_err(|_| "invalid --transcript-limit".to_string())?;
                if cli.transcript_limit <= 0 {
                    return Err("--transcript-limit must be > 0".to_string());
                }
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument '{other}'")),
        }
    }
    Ok(cli)
}

fn next_value(iter: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    iter.next()
        .ok_or_else(|| format!("missing value for {flag}"))
}

fn print_help() {
    println!(
        "Usage: cargo run -p cinder-srv --bin debug_playthrough -- [--play-id UUID] [--player USERNAME] [--pack PACK_ID] [--transcript-limit N]"
    );
}

async fn fetch_play(pool: &PgPool, args: &CliArgs) -> Result<PlayRow, sqlx::Error> {
    if let Some(play_id) = args.play_id.as_deref() {
        return sqlx::query_as::<_, PlayRow>(
            "SELECT gp.id::text AS play_id, p.username, gp.pack_id, gp.locale,
                    to_char(gp.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at,
                    to_char(gp.updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated_at,
                    gp.state_json::text AS state_json
             FROM game_plays gp
             JOIN players p ON p.id = gp.player_id
             WHERE gp.id::text = $1",
        )
        .bind(play_id)
        .fetch_one(pool)
        .await;
    }

    sqlx::query_as::<_, PlayRow>(
        "SELECT gp.id::text AS play_id, p.username, gp.pack_id, gp.locale,
                to_char(gp.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at,
                to_char(gp.updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated_at,
                gp.state_json::text AS state_json
         FROM game_plays gp
         JOIN players p ON p.id = gp.player_id
         WHERE ($1::text IS NULL OR p.username = $1)
           AND ($2::text IS NULL OR gp.pack_id = $2)
         ORDER BY gp.updated_at DESC
         LIMIT 1",
    )
    .bind(args.player.as_deref())
    .bind(args.pack.as_deref())
    .fetch_one(pool)
    .await
}

async fn fetch_transcript(
    pool: &PgPool,
    play_id: &str,
    transcript_limit: i64,
) -> Result<Vec<TranscriptRow>, sqlx::Error> {
    sqlx::query_as::<_, TranscriptRow>(
        "SELECT turn_number, role, text,
                to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at
         FROM transcript_entries
         WHERE play_id::text = $1
         ORDER BY turn_number DESC, id DESC
         LIMIT $2",
    )
    .bind(play_id)
    .bind(transcript_limit)
    .fetch_all(pool)
    .await
    .map(|mut rows| {
        rows.reverse();
        rows
    })
}

fn print_summary(play: &PlayRow, state: &WorldState, transcript: &[TranscriptRow]) {
    println!("play_id: {}", play.play_id);
    println!("player: {}", play.username);
    println!("pack: {}", play.pack_id);
    println!("locale: {}", play.locale);
    println!("created_at: {}", play.created_at);
    println!("updated_at: {}", play.updated_at);
    println!("turn_number: {}", state.turn_number);
    println!("phase: {:?}", state.phase);
    println!("current_room_id: {}", state.current_room_id);
    println!("current_time_minutes: {}", state.current_time_minutes);
    println!("active_stage_ids: {}", state.active_objective_stage_ids.join(", "));
    println!();

    let bundle_vars = collect_bundle_story_vars(state);
    if bundle_vars.is_empty() {
        println!("bundle_story_vars: <none>");
    } else {
        println!("bundle_story_vars:");
        for (key, value) in bundle_vars {
            println!("  {key} = {value}");
        }
    }
    println!();

    println!("transcript_entries: {}", transcript.len());
    for row in transcript {
        println!(
            "[turn {}] {} {}: {}",
            row.turn_number, row.created_at, row.role, row.text
        );
    }
}

fn collect_bundle_story_vars(state: &WorldState) -> BTreeMap<String, String> {
    state
        .story_vars
        .values()
        .iter()
        .filter(|(key, _)| key.starts_with(RULE_BUNDLE_PREFIX))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{CliArgs, DEFAULT_TRANSCRIPT_LIMIT, collect_bundle_story_vars, parse_args};
    use cinder_core::engine::state::WorldState;

    #[test]
    fn parse_args_defaults() {
        assert_eq!(
            parse_args(vec![]).expect("parse"),
            CliArgs {
                play_id: None,
                player: None,
                pack: None,
                transcript_limit: DEFAULT_TRANSCRIPT_LIMIT,
            }
        );
    }

    #[test]
    fn parse_args_accepts_filters() {
        let args = parse_args(vec![
            "--player".to_string(),
            "naush".to_string(),
            "--pack".to_string(),
            "aera".to_string(),
            "--transcript-limit".to_string(),
            "50".to_string(),
        ])
        .expect("parse");

        assert_eq!(args.player.as_deref(), Some("naush"));
        assert_eq!(args.pack.as_deref(), Some("aera"));
        assert_eq!(args.transcript_limit, 50);
    }

    #[test]
    fn collect_bundle_story_vars_filters_to_rule_bundle_keys() {
        let content = cinder_core::content::loader::load_named_pack("aera", Some("en"))
            .expect("load aera");
        let mut state = WorldState::new(&content);
        state.story_vars.set_unchecked("rule_bundle:progress:test:meal", "true");
        state.story_vars.set_unchecked("cook_recipe", "garlic-noodles");

        let bundle_vars = collect_bundle_story_vars(&state);

        assert_eq!(bundle_vars.len(), 1);
        assert_eq!(
            bundle_vars.get("rule_bundle:progress:test:meal")
                .map(String::as_str),
            Some("true")
        );
    }
}
