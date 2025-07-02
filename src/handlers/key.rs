use axum::{extract::{Query, State}, response::{IntoResponse, Response}, Json};
use chrono::Utc;

use crate::models::{AppState, Key, KeyCreateInput, KeyCreateResponse, KeyGetInput, KeyGetResponse, KeyUpdateInput, KeyUpdateResponse, KeyDeleteInput, KeyDeleteResponse, ErrorResponse};

fn get_name_or_generate(input: &Option<String>) -> String {
    input.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

// Handler to create a new key
#[axum::debug_handler]
pub async fn new_key_handler(
    State(app_state): State<AppState>,
    Json(input): Json<KeyCreateInput>,
) -> Result<Json<KeyCreateResponse>, Response> {
    let pool = &app_state.pool;

    // Check if value is shorter than the maximum allowed length
    if let Some(value) = &input.value {
        if value.len() > app_state.config.max_value_length {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("Value exceeds maximum length of {} characters", app_state.config.max_value_length),
                    success: false,
                })
            ).into_response());
        }
    }

    // Helper closure to validate key names
    let validate_key_name = |key: &Option<String>, key_type: &str| {
        if let Some(name) = key {
            if name.len() > app_state.config.max_key_name_length || !is_valid_key_name(name) {
                return Err((
                    axum::http::StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: format!(
                            "{} key name is invalid or exceeds maximum length of {} characters",
                            key_type,
                            app_state.config.max_key_name_length
                        ),
                        success: false,
                    }),
                ).into_response());
            }
        }
        Ok(())
    };

    // Validate normal and read-only key names
    validate_key_name(&input.name, "Key")?;
    validate_key_name(&input.name_readonly, "Read-only")?;

    // Check if name and roname are provided, if not, generate random names
    let name = get_name_or_generate(&input.name);
    let roname = get_name_or_generate(&input.name_readonly);

    let row = sqlx::query_as::<_, Key>(
        r#"
        INSERT INTO keys (name, roname, value, last_accessed)
        VALUES ($1, $2, $3, $4)
        RETURNING name, roname, value, last_accessed
        "#
    )
    .bind(name)
    .bind(roname)
    .bind(input.value.unwrap_or_default())
    .bind(Utc::now())
    .fetch_one(pool)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                success: false,
            })
        ).into_response()
    })?;

    Ok(Json(KeyCreateResponse {
        name: row.name,
        name_readonly: row.roname,
        success: true,
    }))
}

// Handler to get a key by name (using query parameter 'name')
pub async fn get_key_handler(
    State(app_state): State<AppState>,
    Query(input): Query<KeyGetInput>,
) -> Result<Json<KeyGetResponse>, Response> {
    let pool = &app_state.pool;

    let row = sqlx::query_as::<_, Key>(
        r#"
        SELECT name, roname, value, last_accessed FROM keys WHERE name = $1 OR roname = $1
        "#)
    .bind(&input.name)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                success: false,
            })
        ).into_response()
    })?;

    // Update last_accessed timestamp
    sqlx::query(
        r#"
        UPDATE keys SET last_accessed = $1 WHERE name = $2 OR roname = $2
        "#)
    .bind(Utc::now())
    .bind(&input.name)
    .execute(pool)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                success: false,
            })
        ).into_response()
    })?;

    Ok(Json(KeyGetResponse {
        value: row.value,
        success: true,
    }))
}

// Handler to update a key's value by name. Accepts JSON input with 'name' and 'value'.
pub async fn update_key_handler(
    State(app_state): State<AppState>,
    Json(input): Json<KeyUpdateInput>,
) -> Result<Json<KeyUpdateResponse>, Response> {
    let pool = &app_state.pool;

    // Check if value is shorter than the maximum allowed length
    if input.value.len() > app_state.config.max_value_length {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Value exceeds maximum length of {} characters", app_state.config.max_value_length),
                success: false,
            })
        ).into_response());
    }

    let result = sqlx::query(
        r#"
        UPDATE keys SET value = $1, last_accessed = $2 WHERE name = $3
        "#    )
    .bind(&input.value)
    .bind(Utc::now())
    .bind(&input.name)
    .execute(pool)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                success: false,
            })
        ).into_response()
    })?;

    // Check if the update was successful
    if result.rows_affected() == 0 {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Key '{}' not found or read-only key used", &input.name),
                success: false,
            })
        ).into_response());
    }

    Ok(Json(KeyUpdateResponse {
        success: true,
    }))
}

// Handler to delete a key by name
pub async fn delete_key_handler(
    State(app_state): State<AppState>,
    Query(input): Query<KeyDeleteInput>,
) -> Result<Json<KeyDeleteResponse>, Response> {
    let pool = &app_state.pool;

    let row = sqlx::query(
        r#"
        DELETE FROM keys WHERE name = $1
        "#)
    .bind(&input.name)
    .execute(pool)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                success: false,
            })
        ).into_response()
    })?;

    // Check if any rows were affected
    if row.rows_affected() == 0 {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Key '{}' not found or read-only key used", &input.name),
                success: false,
            })
        ).into_response());
    }

    Ok(Json(KeyDeleteResponse {
        success: true,
    }))
}

// Function to check validity of key name.
// It must be alphanumeric and can contain underscores, dashes, and dots.
pub fn is_valid_key_name(name: &str) -> bool {
    // Check if the name is empty
    if name.is_empty() {
        return false;
    }

    // Check if the name contains only valid characters
    name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
}
