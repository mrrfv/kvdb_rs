mod db;
mod models;
mod handlers;

use db::{create_pool, get_environment_variable, get_environment_variable_or_default, delete_unused_keys};
use models::{AppConfig, AppState};

use std::{net::SocketAddr, sync::Arc};
use axum::{http::Method, response::IntoResponse, routing::post, Json, Router};
use handlers::*;
use sqlx::PgPool;
use dotenv::dotenv;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use tower_http::cors::{CorsLayer, Any};
use axum::http::{HeaderValue};
use tower_http::cors::{AllowOrigin};

#[tokio::main]
async fn main() {
    dotenv().ok();

    let app_config = AppConfig {
        database_url: get_environment_variable("DATABASE_URL"),
        listen_on: get_environment_variable_or_default("LISTEN_ON", "0.0.0.0:3005"),
        rate_limit_per_second: get_environment_variable("RATE_LIMIT_PER_SECOND")
            .parse()
            .expect("Invalid RATE_LIMIT_PER_SECOND value"),
        rate_limit_burst_size: get_environment_variable("RATE_LIMIT_BURST_SIZE")
            .parse()
            .expect("Invalid RATE_LIMIT_BURST_SIZE value"),
        cors_origins: get_environment_variable_or_default("CORS_ORIGINS", "")
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        key_cleanup_every_s: get_environment_variable_or_default("KEY_CLEANUP_EVERY_S", "0.0")
            .parse()
            .expect("Invalid KEY_CLEANUP_EVERY_S value"),
        delete_unused_keys_after: get_environment_variable_or_default("DELETE_UNUSED_KEYS_AFTER", "default_disabled"),
        max_value_length: get_environment_variable("MAX_VALUE_LENGTH")
            .parse()
            .expect("Invalid MAX_VALUE_LENGTH value"),
        max_key_name_length: get_environment_variable("MAX_KEY_NAME_LENGTH")
            .parse()
            .expect("Invalid MAX_KEY_NAME_LENGTH value"),
    };

    let pool: PgPool = create_pool(&app_config.database_url).await;

    // This macro embeds migrations from the ./migrations directory at compile time
    if let Err(e) = sqlx::migrate!("./migrations").run(&pool).await {
        eprintln!("Failed to run embedded database migrations: {:?}", e);
        panic!("Failed to run embedded database migrations: {:?}", e);
    }

    let rate_limit_config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(app_config.rate_limit_per_second)
            .burst_size(app_config.rate_limit_burst_size)
            .error_handler(|err| {
                eprintln!("Rate limit error: {}", err);
                (
                    axum::http::StatusCode::TOO_MANY_REQUESTS,
                    Json(models::ErrorResponse {
                        error: "Rate limit exceeded".to_string(),
                        success: false,
                    })
                ).into_response()
            })
            .finish()
            .unwrap(),
    );

    let governor_limiter = rate_limit_config.limiter().clone();

    // Cleanup rate limiter state every 60 seconds
    tokio::spawn({
        let governor_limiter = governor_limiter.clone();
        async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                governor_limiter.retain_recent();
            }
        }
    });

    // 0.0 indicates the feature should be disabled
    if app_config.key_cleanup_every_s != 0.0 && app_config.delete_unused_keys_after != "default_disabled" {
        // Delete unused keys now, and on an interval
        delete_unused_keys(&pool, &app_config.delete_unused_keys_after).await;
        let coroutine_pool = pool.clone();
        let delete_unused_keys_after = app_config.delete_unused_keys_after.clone();
        let key_cleanup_every_s = app_config.key_cleanup_every_s;
        tokio::spawn({
            async move {
                loop {
                    
                    tokio::time::sleep(std::time::Duration::from_secs_f64(key_cleanup_every_s)).await;
                    delete_unused_keys(&coroutine_pool, &delete_unused_keys_after).await;
                }
            }
        });
    } else {
        println!("Key cleanup disabled or DELETE_UNUSED_KEYS_AFTER not set");
    }

    // Initialize CORS middleware
    let cors = if app_config.cors_origins.iter().any(|o| o == "*") {
        // Handle cases when the list of CORS origins contains a wildcard (*)
        CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
            .allow_origin(Any)
            .allow_headers([axum::http::header::CONTENT_TYPE])
    } else {
        // All other cases, e.g. https://*.example.org, https://example.com

        // Clone the list of allowed origins for use in the async predicate
        let allowed_origins = app_config.cors_origins.clone();

        CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
            .allow_headers([axum::http::header::CONTENT_TYPE])
            .allow_origin(AllowOrigin::async_predicate(
            move |origin: HeaderValue, _request_parts| {
                let allowed_origins = allowed_origins.clone();
                async move {
                // If "*" is present, allow all origins
                if allowed_origins.iter().any(|o| o == "*") {
                    return true;
                }
                // Check for exact match or wildcard match
                allowed_origins.iter().any(|allowed| {
                    if allowed.contains('*') {
                        // Simple wildcard matching: "*.domain.com"
                        let pattern = allowed.replace('.', r"\.").replace('*', ".*");
                        let re = regex::Regex::new(&format!("^{}$", pattern)).unwrap();
                        re.is_match(origin.to_str().unwrap_or(""))
                    } else {
                        allowed == origin.to_str().unwrap_or("")
                    }
                })
                }
            }
            ))
    };

    // Create the Axum application with the rate limiter
    let app_state = AppState {
        config: app_config.clone(),
        pool: pool.clone(),
    };

    // Define OpenAPI documentation
    #[derive(OpenApi)]
    #[openapi(
        paths(
            new_key_handler,
            get_key_handler,
            update_key_handler,
            delete_key_handler,
        ),
        components(
            schemas(models::ErrorResponse)
        ),
        tags(
            (name = "kvdb", description = "Postgres-based key value database server")
        )
    )]
    struct ApiDoc;

    let api = ApiDoc::openapi();

    let app = Router::new()
        .route("/key",
            post(new_key_handler)
            .get(get_key_handler)
            .patch(update_key_handler)
            .delete(delete_key_handler)
        )
        .route("/health", axum::routing::get(|| async { "OK" }))
        .with_state(app_state)
        .layer(GovernorLayer {
            config: rate_limit_config,
        })
        .layer(cors)
        .merge(SwaggerUi::new("/").url("/api-doc/openapi.json", api));

    // Print app configuration without sensitive data (like database URL)
    let mut sanitized_config = app_config.clone();
    sanitized_config.database_url = "REDACTED".to_string(); // Redact sensitive data
    println!("Starting server with configuration: {:#?}", sanitized_config);

    let listener = tokio::net::TcpListener::bind(app_config.listen_on).await.unwrap();
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
}
