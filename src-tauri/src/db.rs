use std::path::Path;

use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    ConnectOptions, SqlitePool,
};

pub async fn init(data_dir: &Path) -> Result<SqlitePool, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(data_dir)?;

    let filename = if cfg!(debug_assertions) {
        "personal-dev.db"
    } else {
        "personal.db"
    };

    let database_path = data_dir.join(filename);

    let options = SqliteConnectOptions::new()
        .filename(&database_path)
        .create_if_missing(true)
        .foreign_keys(true)
        .with_regexp()
        .disable_statement_logging();

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;

    if let Err(error) = sqlx::migrate!("db/migrations").run(&pool).await {
        pool.close().await;
        return Err(error.into());
    }

    println!("SQLite database: {}", database_path.display());

    Ok(pool)
}