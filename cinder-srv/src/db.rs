use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::Arc;

use crate::config::Config;

pub async fn init_pool(config: &Config) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

pub type DbPool = Arc<PgPool>;
