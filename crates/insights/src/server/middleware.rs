//! Request context and middleware for the insights REST API
//!
//! Provides unified request context containing logger and request metadata
//! that is automatically injected into all endpoints via middleware.

use axum::{
    extract::Request,
    http::{HeaderMap, Method, Uri},
    middleware::Next,
    response::Response,
};
use bentley::DaemonLogs;
use bentley::daemon_logs::LogContext;
use std::sync::Arc;
use uuid::Uuid;


/// Request context containing logger and request metadata
#[derive(Clone)]
pub struct RequestContext {
    /// Unique ID for this request
    pub request_id: Uuid,
    /// HTTP method
    pub method: Method,
    /// Request URI
    pub uri: Uri,
    /// Request headers
    pub headers: HeaderMap,
    /// Shared logger instance
    pub logger: Arc<DaemonLogs>,
}

impl RequestContext {
    /// Create a new request context
    pub fn new(
        method: Method,
        uri: Uri,
        headers: HeaderMap,
        logger: Arc<DaemonLogs>,
    ) -> Self {
        Self {
            request_id: Uuid::new_v4(),
            method,
            uri,
            headers,
            logger,
        }
    }

    /// Log an info message with request context
    pub async fn log_info(&self, message: &str, component: &str) {
        self.log_with_context(message, "info", component, None, None).await;
    }

    /// Log a success message with request context
    pub async fn log_success(&self, message: &str, component: &str) {
        self.log_with_context(message, "success", component, None, None).await;
    }

    /// Log an error message with request context
    pub async fn log_error(&self, message: &str, component: &str) {
        self.log_with_context(message, "error", component, None, None).await;
    }

    /// Log a warning message with request context
    pub async fn log_warn(&self, message: &str, component: &str) {
        self.log_with_context(message, "warn", component, None, None).await;
    }

    /// Log with full context information
    pub async fn log_with_context(&self, message: &str, level: &str, component: &str, status_code: Option<u16>, duration_ms: Option<f64>) {
        let user_agent = self.headers.get("user-agent")
            .map(|v| v.to_str().unwrap_or("unknown"))
            .unwrap_or("none")
            .to_string();

        let context = LogContext {
            request_id: Some(self.request_id.to_string()),
            method: Some(self.method.to_string()),
            path: Some(self.uri.path().to_string()),
            user_agent: Some(user_agent),
            status_code,
            duration_ms,
        };

        match level {
            "info" => self.logger.info_with_context(message, component, context).await,
            "success" => self.logger.success_with_context(message, component, context).await,
            "warn" => self.logger.warn_with_context(message, component, context).await,
            "error" => self.logger.error_with_context(message, component, context).await,
            _ => self.logger.info_with_context(message, component, context).await,
        }
    }

    /// Log request start
    pub async fn log_request_start(&self) {
        self.log_with_context("Request started", "info", "http-request", None, None).await;
    }

    /// Log request completion with status
    pub async fn log_request_complete(&self, status_code: u16, duration_ms: f64) {
        self.log_with_context("Request completed", "info", "http-request", Some(status_code), Some(duration_ms)).await;
    }
}

/// Global logger instance
static GLOBAL_LOGGER: once_cell::sync::OnceCell<Arc<DaemonLogs>> = once_cell::sync::OnceCell::new();

/// Initialize the global logger
pub fn init_global_logger(logger: Arc<DaemonLogs>) -> Result<(), Arc<DaemonLogs>> {
    GLOBAL_LOGGER.set(logger)
}

/// Get the global logger instance
pub fn get_global_logger() -> &'static Arc<DaemonLogs> {
    GLOBAL_LOGGER
        .get()
        .expect("Global logger should be initialized before use")
}

/// Middleware to inject RequestContext into all requests
pub async fn request_context_middleware(
    request: Request,
    next: Next,
) -> Response {
    let logger = get_global_logger().clone();

    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();

    let context = RequestContext::new(method, uri, headers, logger);

    // Log request start
    let start_time = std::time::Instant::now();
    context.log_request_start().await;

    // Add context to request extensions
    let mut request = request;
    request.extensions_mut().insert(context.clone());

    // Process the request
    let response = next.run(request).await;

    // Log request completion
    let duration = start_time.elapsed();
    let duration_ms = duration.as_secs_f64() * 1000.0;
    context.log_request_complete(response.status().as_u16(), duration_ms).await;

    response
}