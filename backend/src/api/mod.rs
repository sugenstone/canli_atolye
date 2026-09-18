//! API katmanı: Axum router, state, error dönüşümü ve route handler'ları.

pub mod error;
pub mod extractor;
pub mod routes;
pub mod state;

use axum::routing::get;
use axum::Router;
use state::AppState;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

/// Uygulama router'ı: `/api/v1` altında tüm kaynaklar + health.
pub fn router(state: AppState) -> Router {
    let cors = state
        .config
        .cors_origin
        .as_ref()
        .map(|origin| {
            use axum::http::{HeaderName, HeaderValue, Method};
            CorsLayer::new()
                .allow_origin(
                    origin
                        .parse::<HeaderValue>()
                        .expect("CORS_ORIGIN geçerli bir URL olmalı"),
                )
                .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
                .allow_credentials(true)
                .allow_headers([HeaderName::from_static("content-type")])
        })
        .unwrap_or_default();

    Router::new()
        .route("/health", get(health))
        .nest("/api/v1", routes::v1_router())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}
