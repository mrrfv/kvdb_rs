use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;

pub async fn create_pool(database_url: &str) -> PgPool {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .expect("Failed to create database connection pool")
}

pub async fn delete_unused_keys(pool: &PgPool, after: &String) {
    let result = sqlx::query(&format!("DELETE FROM keys WHERE last_accessed < CURRENT_TIMESTAMP - INTERVAL '{}'", after).to_string())
        .execute(pool)
        .await;

    match result {
        Ok(result) => {
            let affected_rows = result.rows_affected();
            println!("Unused key cleanup complete, {} rows affected.", affected_rows)
        },
        Err(e) => {
            eprintln!("An error occured while cleaning up keys: {}", e);
        }
    }
}

pub fn get_environment_variable(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| {
        eprintln!("Environment variable {} not set", key);
        std::process::exit(1);
    })
}

pub fn get_environment_variable_or_default(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| {
        eprintln!("Environment variable {} not set, using default: {}", key, default);
        default.to_string()
    })
}
