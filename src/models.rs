use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub listen_on: String,
    pub rate_limit_per_second: u64,
    pub rate_limit_burst_size: u32,
    pub cors_origins: Vec<String>,
    pub key_cleanup_every_s: f64,
    pub delete_unused_keys_after: String,
    pub max_value_length: usize,
    pub max_key_name_length: usize,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub pool: sqlx::PgPool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Key {
    pub name: String,
    pub roname: String,
    pub value: String,
    pub last_accessed: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct KeyCreateInput {
    pub name: Option<String>,
    pub name_readonly: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct KeyCreateResponse {
    pub name: String,
    pub name_readonly: String,
    pub success: bool,
}

#[derive(Debug, Deserialize)]
pub struct KeyGetInput {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct KeyGetResponse {
    pub value: String,
    pub success: bool,
}

#[derive(Debug, Deserialize)]
pub struct KeyUpdateInput {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct KeyUpdateResponse {
    pub success: bool,
}

#[derive(Debug, Deserialize)]
pub struct KeyDeleteInput {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct KeyDeleteResponse {
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub success: bool
}
