//! Axum router configuration for all endpoints

use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::server::handlers::{insights, logs, status};

/// Create the main application router
pub fn create_router() -> Router {
    Router::new()
        // Status and version endpoints
        .route("/status", get(status::status))
        .route("/version", get(status::version))
        .route("/api", get(status::api_info))
        
        // Logs endpoint
        .route("/logs", get(logs::get_logs))
        
        // Insights endpoints
        .route("/insights/add", post(insights::add_insight))
        .route("/insights/get", post(insights::get_insight))
        .route("/insights/update", put(insights::update_insight))
        .route("/insights/remove", delete(insights::remove_insight))
        .route("/insights/clear", delete(insights::clear_insights))
        .route("/insights/index", delete(insights::reindex))
        .route("/insights/list/topics", get(insights::list_topics))
        .route("/insights/list/insights", get(insights::list_insights))
        .route("/insights/search", get(insights::search_insights))
}
