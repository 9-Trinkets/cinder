//! Debug utility for printing actor-turn prompts from persisted playthroughs.
//!
//! Typical local usage:
//!
//! ```bash
//! cargo run -p cinder-srv --bin debug_prompts -- --pack aera
//! ```
//!
//! Print one actor's prompt from the latest matching playthrough:
//!
//! ```bash
//! cargo run -p cinder-srv --bin debug_prompts -- --pack aera --actor ren
//! ```
//!
//! If Postgres is only exposed inside Docker Compose, run it on the Compose network:
//!
//! ```bash
//! docker run --rm --network cinder_default \
//!   -v "$PWD":/work -w /work \
//!   -e CINDER_DATABASE_URL='postgres://cinder:cinder@postgres:5432/cinder' \
//!   rust:1-bookworm \
//!   cargo run -p cinder-srv --bin debug_prompts -- --pack aera --actor ren
//! ```

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::actor_turn::build_actor_turn;
use cinder_core::engine::dialogue::{
    actor_turn_decider_system_prompt_text, render_actor_turn_decider_prompt,
};
use cinder_core::engine::state::WorldState;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Default, PartialEq, Eq)]
struct CliArgs {
    play_id: Option<String>,
    player: Option<String>,
    pack: Option<String>,
    actor: Option<String>,
    all_actors: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct PlayRow {
    play_id: String,
    username: String,
    pack_id: String,
    locale: String,
    state_json: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let args = parse_args(std::env::args().skip(1).collect())?;
    let pool = init_pool().await?;
    let play = fetch_play(&pool, &args).await?;
    let content = load_named_pack(&play.pack_id, Some(&play.locale))?;
    let state: WorldState = serde_json::from_str(&play.state_json)?;
    let actor_ids = selected_actor_ids(&content, &state, &args)?;

    println!("play_id: {}", play.play_id);
    println!("player: {}", play.username);
    println!("pack: {}", play.pack_id);
    println!("locale: {}", play.locale);
    println!("actors: {}", actor_ids.join(", "));
    println!();

    for actor_id in actor_ids {
        let actor = content
            .actor(&actor_id)
            .ok_or_else(|| format!("missing actor '{actor_id}'"))?;
        let current_room_id = state.actor_room_id(&actor.id, &actor.room_id).to_string();
        let rules = content.movement_rules(&actor.id);
        let build = build_actor_turn(Arc::new(content.clone()), &state, actor, &rules)?;
        let system_prompt = actor_turn_decider_system_prompt_text(&build.request);
        let prompt = render_actor_turn_decider_prompt(&build.request);

        println!("=== actor: {} ({}) ===", actor.id, actor.name);
        println!("room: {current_room_id}");
        println!();
        println!("--- system prompt ---");
        println!("{system_prompt}");
        println!();
        println!("--- request json ---");
        println!("{}", serde_json::to_string_pretty(&build.request)?);
        println!();
        println!("--- rendered prompt ---");
        println!("{prompt}");
        println!();
    }

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
    let mut cli = CliArgs::default();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--play-id" => cli.play_id = Some(next_value(&mut iter, "--play-id")?),
            "--player" => cli.player = Some(next_value(&mut iter, "--player")?),
            "--pack" => cli.pack = Some(next_value(&mut iter, "--pack")?),
            "--actor" => cli.actor = Some(next_value(&mut iter, "--actor")?),
            "--all-actors" => cli.all_actors = true,
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
        "Usage: cargo run -p cinder-srv --bin debug_prompts -- [--play-id UUID] [--player USERNAME] [--pack PACK_ID] [--actor ACTOR_ID] [--all-actors]"
    );
}

async fn fetch_play(pool: &PgPool, args: &CliArgs) -> Result<PlayRow, sqlx::Error> {
    if let Some(play_id) = args.play_id.as_deref() {
        return sqlx::query_as::<_, PlayRow>(
            "SELECT gp.id::text AS play_id, p.username, gp.pack_id, gp.locale,
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

fn selected_actor_ids(
    content: &cinder_core::content::types::ContentPack,
    state: &WorldState,
    args: &CliArgs,
) -> Result<Vec<String>, String> {
    if let Some(actor_id) = args.actor.as_deref() {
        return content
            .actor(actor_id)
            .map(|_| vec![actor_id.to_string()])
            .ok_or_else(|| format!("unknown actor '{actor_id}'"));
    }

    let actor_ids = if args.all_actors {
        content
            .actors
            .iter()
            .map(|actor| actor.id.clone())
            .collect::<Vec<_>>()
    } else {
        content
            .actors
            .iter()
            .filter(|actor| state.actor_room_id(&actor.id, &actor.room_id) == state.current_room_id)
            .map(|actor| actor.id.clone())
            .collect::<Vec<_>>()
    };
    if actor_ids.is_empty() {
        Err("no matching actors for prompt dump".to_string())
    } else {
        Ok(actor_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::{CliArgs, parse_args};

    #[test]
    fn parse_args_defaults() {
        assert_eq!(parse_args(vec![]).expect("parse"), CliArgs::default());
    }

    #[test]
    fn parse_args_accepts_actor_filters() {
        let args = parse_args(vec![
            "--pack".to_string(),
            "aera".to_string(),
            "--actor".to_string(),
            "ren".to_string(),
            "--all-actors".to_string(),
        ])
        .expect("parse");

        assert_eq!(args.pack.as_deref(), Some("aera"));
        assert_eq!(args.actor.as_deref(), Some("ren"));
        assert!(args.all_actors);
    }
}
