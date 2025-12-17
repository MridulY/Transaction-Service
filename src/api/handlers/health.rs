use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    status: String,
    version: String,
}

pub async fn health_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "healthy".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }),
    )
}

/// Metrics endpoint for Prometheus scraping
pub async fn metrics() -> impl IntoResponse {
    use crate::infrastructure::telemetry::metrics::MetricsRecorder;

    // Note: In production, you'd store the MetricsRecorder in AppState
    // For now, we rely on the global recorder
    match MetricsRecorder::init() {
        Ok(recorder) => (StatusCode::OK, recorder.render()),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to render metrics".to_string(),
        ),
    }
}
