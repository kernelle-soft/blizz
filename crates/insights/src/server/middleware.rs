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
use bentley::daemon_logs::LogContext;
use bentley::DaemonLogs;
use std::sync::Arc;
use uuid::Uuid;

#[cfg(feature = "ml-features")]
use crate::server::services::vector_database::BoxedVectorDatabase;

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
  /// Vector database service instance (only available with ml-features)
  #[cfg(feature = "ml-features")]
  pub vector_db: Arc<BoxedVectorDatabase>,
}

impl RequestContext {
  /// Create a new request context (with ML features)
  #[cfg(feature = "ml-features")]
  pub fn new(
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    logger: Arc<DaemonLogs>,
    vector_db: Arc<BoxedVectorDatabase>,
  ) -> Self {
    Self { request_id: Uuid::new_v4(), method, uri, headers, logger, vector_db }
  }

  /// Create a new request context (without ML features)  
  #[cfg(not(feature = "ml-features"))]
  pub fn new(method: Method, uri: Uri, headers: HeaderMap, logger: Arc<DaemonLogs>) -> Self {
    Self { request_id: Uuid::new_v4(), method, uri, headers, logger }
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
  pub async fn log_with_context(
    &self,
    message: &str,
    level: &str,
    component: &str,
    status_code: Option<u16>,
    duration_ms: Option<f64>,
  ) {
    let user_agent = self
      .headers
      .get("user-agent")
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

  /// Log request start (only at verbose level to reduce noise)
  pub async fn log_request_start(&self) {
    // Only log request starts at verbose level to reduce console noise
    // self.log_with_context("Request started", "verbose", "http-request", None, None).await;
  }

  /// Log request completion with status (only for errors or slow requests)
  pub async fn log_request_complete(&self, status_code: u16, duration_ms: f64) {
    // Only log completed requests if they're errors or slow (> 1000ms)
    if status_code >= 400 || duration_ms > 1000.0 {
      self
        .log_with_context(
          &format!("Request completed: {} ({}ms)", status_code, duration_ms as u32),
          "info",
          "http-request",
          Some(status_code),
          Some(duration_ms),
        )
        .await;
    } else {
      // Only log successful fast requests at verbose level
      // self.log_with_context("Request completed", "verbose", "http-request", Some(status_code), Some(duration_ms)).await;
    }
  }
}

/// Log levels for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
  Error = 0,
  Warn = 1,
  Info = 2,
  Success = 3,
  Verbose = 4,
  Debug = 5,
}

impl LogLevel {
  pub fn parse(s: &str) -> Self {
    match s.to_lowercase().as_str() {
      "error" => LogLevel::Error,
      "warn" => LogLevel::Warn,
      "info" => LogLevel::Info,
      "success" | "sccs" => LogLevel::Success,
      "verbose" | "verb" => LogLevel::Verbose,
      "debug" => LogLevel::Debug,
      _ => LogLevel::Info,
    }
  }

  pub fn as_str(&self) -> &'static str {
    match self {
      LogLevel::Error => "error",
      LogLevel::Warn => "warn",
      LogLevel::Info => "info",
      LogLevel::Success => "success",
      LogLevel::Verbose => "verbose",
      LogLevel::Debug => "debug",
    }
  }
}

/// Global logger instance
static GLOBAL_LOGGER: once_cell::sync::OnceCell<Arc<DaemonLogs>> = once_cell::sync::OnceCell::new();

/// Global log level (defaults to Info to reduce verbosity)
static GLOBAL_LOG_LEVEL: once_cell::sync::OnceCell<LogLevel> = once_cell::sync::OnceCell::new();

/// Global vector database service instance (only with ml-features)
#[cfg(feature = "ml-features")]
static GLOBAL_VECTOR_DB: once_cell::sync::OnceCell<Arc<BoxedVectorDatabase>> =
  once_cell::sync::OnceCell::new();

/// Initialize the global logger and log level
pub fn init_global_logger(logger: Arc<DaemonLogs>) -> Result<(), Arc<DaemonLogs>> {
  // Set default log level to Info (less verbose than before)
  let _ = GLOBAL_LOG_LEVEL.set(LogLevel::Info);
  GLOBAL_LOGGER.set(logger)
}

/// Set the global log level
pub fn set_log_level(level: LogLevel) {
  let _ = GLOBAL_LOG_LEVEL.set(level);
}

/// Check if a log level should be output based on current global level
pub fn should_log(level: LogLevel) -> bool {
  let current_level = GLOBAL_LOG_LEVEL.get().copied().unwrap_or(LogLevel::Info);
  level <= current_level
}

/// Centralized logging function that goes to both console and file
pub async fn log_to_both(level: LogLevel, message: &str, component: &str) {
  // Only log if level is enabled
  if !should_log(level) {
    return;
  }

  // Log to console using bentley
  match level {
    LogLevel::Error => bentley::error(message),
    LogLevel::Warn => bentley::warn(message),
    LogLevel::Info => bentley::info(message),
    LogLevel::Success => bentley::success(message),
    LogLevel::Verbose => bentley::verbose(message),
    LogLevel::Debug => bentley::debug(message),
  }

  // Also log to file if we have a global logger
  if let Some(logger) = GLOBAL_LOGGER.get() {
    let _ = logger.add_log(level.as_str(), message, component).await;
  }
}

/// Initialize the global vector database service (only with ml-features)
#[cfg(feature = "ml-features")]
pub fn init_global_vector_db(
  vector_db: Arc<BoxedVectorDatabase>,
) -> Result<(), Arc<BoxedVectorDatabase>> {
  GLOBAL_VECTOR_DB.set(vector_db)
}

/// Get the global logger instance
pub fn get_global_logger() -> &'static Arc<DaemonLogs> {
  GLOBAL_LOGGER.get().expect("Global logger should be initialized before use")
}

/// Get the global vector database service instance (only with ml-features)
#[cfg(feature = "ml-features")]
pub fn get_global_vector_db() -> &'static Arc<BoxedVectorDatabase> {
  GLOBAL_VECTOR_DB.get().expect("Global vector database service should be initialized before use")
}

/// Middleware to inject RequestContext into all requests
pub async fn request_context_middleware(request: Request, next: Next) -> Response {
  let logger = get_global_logger().clone();

  let method = request.method().clone();
  let uri = request.uri().clone();
  let headers = request.headers().clone();

  // Create context conditionally based on ML features availability
  let context = {
    #[cfg(feature = "ml-features")]
    {
      let vector_db = get_global_vector_db().clone();
      RequestContext::new(method, uri, headers, logger, vector_db)
    }

    #[cfg(not(feature = "ml-features"))]
    {
      RequestContext::new(method, uri, headers, logger)
    }
  };

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
/// Convenient async logging functions that write to both console and file
pub async fn server_info(message: &str, component: &str) {
  log_to_both(LogLevel::Info, message, component).await;
}

pub async fn server_verbose(message: &str, component: &str) {
  log_to_both(LogLevel::Verbose, message, component).await;
}

pub async fn server_warn(message: &str, component: &str) {
  log_to_both(LogLevel::Warn, message, component).await;
}

pub async fn server_error(message: &str, component: &str) {
  log_to_both(LogLevel::Error, message, component).await;
}
