//! REST server startup and configuration

use anyhow::Result;
use axum::{middleware, serve};
use bentley::daemon_logs::DaemonLogs;
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::server::{
  middleware::{init_global_logger, request_context_middleware},
  routing::create_router,
};

/// Start the REST server
#[cfg(not(tarpaulin_include))] // Skip coverage - server lifecycle and daemon logs initialization
pub async fn start_server(addr: SocketAddr) -> Result<()> {
  // Initialize daemon logs for persistent logging
  let logs_path = get_server_logs_path();
  let daemon_logs = Arc::new(DaemonLogs::new(&logs_path)?);

  // Initialize global logger
  init_global_logger(daemon_logs.clone())
    .map_err(|_| anyhow::anyhow!("Failed to initialize global logger"))?;

  // Log server startup
  daemon_logs.info(&format!("Starting insights REST server on {addr}"), "insights-server").await;
  bentley::info!(&format!("Starting insights REST server on {addr}"));

  // Create the router with automatic request context middleware
  let app = create_router().layer(middleware::from_fn(request_context_middleware)).layer(
    ServiceBuilder::new().layer(TraceLayer::new_for_http()).layer(CorsLayer::permissive()), // TODO: Configure CORS properly for production
  );

  // Create listener
  let listener = TcpListener::bind(addr).await?;
  daemon_logs.info(&format!("Server listening on {addr}"), "insights-server").await;
  bentley::info!(&format!("Server listening on {addr}"));

  // Start serving
  match serve(listener, app).await {
    Ok(_) => {
      daemon_logs.info("Server shutdown gracefully", "insights-server").await;
      Ok(())
    }
    Err(e) => {
      daemon_logs.error(&format!("Server error: {e}"), "insights-server").await;
      Err(anyhow::anyhow!("Server error: {}", e))
    }
  }
}

/// Get the path for server logs
#[cfg(not(tarpaulin_include))] // Skip coverage - filesystem path operations
fn get_server_logs_path() -> std::path::PathBuf {
  dirs::home_dir()
    .unwrap_or_else(|| std::path::Path::new("/tmp").to_path_buf())
    .join(".blizz")
    .join("persistent")
    .join("insights")
    .join("server-logs.jsonl")
}
