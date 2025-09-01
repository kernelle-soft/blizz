//! REST server startup and configuration

use anyhow::Result;
use axum::serve;
use bentley::daemon_logs::DaemonLogs;
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::server::routing::{create_router, create_router_with_logger};

/// Start the REST server
pub async fn start_server(addr: SocketAddr) -> Result<()> {
  // Initialize daemon logs for persistent logging
  let logs_path = get_server_logs_path();
  let daemon_logs = Arc::new(DaemonLogs::new(&logs_path)?);
  
  // Log server startup
  daemon_logs.info(&format!("Starting insights REST server on {addr}"), "insights-server").await;
  bentley::info!(&format!("Starting insights REST server on {addr}"));

  // Create the router with all endpoints, passing the shared logger
  let app = create_router_with_logger(daemon_logs.clone()).layer(
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
      daemon_logs.error(&format!("Server error: {}", e), "insights-server").await;
      Err(anyhow::anyhow!("Server error: {}", e))
    }
  }
}

/// Get the path for server logs  
fn get_server_logs_path() -> std::path::PathBuf {
  dirs::home_dir()
    .unwrap_or_else(|| std::path::Path::new("/tmp").to_path_buf())
    .join(".insights") 
    .join("rest_server.logs.jsonl")
}
