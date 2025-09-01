//! Axum router configuration for all endpoints

use axum::{
  routing::{delete, get, post, put},
  Router,
};
use bentley::daemon_logs::DaemonLogs;
use std::sync::Arc;

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
    .route("/insights/search", post(insights::search_insights))
}

/// Create the main application router with shared logger
pub fn create_router_with_logger(daemon_logs: Arc<DaemonLogs>) -> Router {
  Router::new()
    // Status and version endpoints
    .route("/status", get(status::status))
    .route("/version", get(status::version))
    .route("/api", get(status::api_info))
    // Logs endpoint with shared logger
    .route("/logs", get(logs::get_logs_with_shared_state))
    // Insights endpoints
    .route("/insights/add", post(insights::add_insight))
    .route("/insights/get", post(insights::get_insight))
    .route("/insights/update", put(insights::update_insight))
    .route("/insights/remove", delete(insights::remove_insight))
    .route("/insights/clear", delete(insights::clear_insights))
    .route("/insights/index", delete(insights::reindex))
    .route("/insights/list/topics", get(insights::list_topics))
    .route("/insights/list/insights", get(insights::list_insights))
    .route("/insights/search", post(insights::search_insights))
    // Share the logger instance as axum state
    .with_state(daemon_logs)
}
