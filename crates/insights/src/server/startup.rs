//! REST server startup and configuration

use anyhow::Result;
use axum::serve;
use bentley::daemon_logs::DaemonLogs;
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::server::{middleware::init_global_logger, routing::create_router};

#[cfg(feature = "ml-features")]
use crate::server::{middleware::init_global_lancedb, services::lancedb::LanceDbService};

/// Start the REST server
#[cfg(not(tarpaulin_include))] // Skip coverage - server lifecycle and daemon logs initialization
pub async fn start_server(addr: SocketAddr) -> Result<()> {
  // Initialize daemon logs for persistent logging
  let logs_path = get_server_logs_path();
  let daemon_logs = Arc::new(DaemonLogs::new(&logs_path)?);

  // Initialize global logger
  init_global_logger(daemon_logs.clone())
    .map_err(|_| anyhow::anyhow!("Failed to initialize global logger"))?;

  // Initialize LanceDB service (only with ml-features)
  #[cfg(feature = "ml-features")]
  {
    let lancedb_path = get_lancedb_data_path();
    let lancedb_service = Arc::new(
      LanceDbService::new(lancedb_path, "insights_embeddings")
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize LanceDB: {}", e))?,
    );

    // Initialize global LanceDB service
    init_global_lancedb(lancedb_service.clone())
      .map_err(|_| anyhow::anyhow!("Failed to initialize global LanceDB service"))?;

    daemon_logs.info("LanceDB service initialized successfully", "insights-server").await;
  }

  #[cfg(not(feature = "ml-features"))]
  {
    daemon_logs.info("Running in lightweight mode (no ML features)", "insights-server").await;
  }

  // Log server startup
  daemon_logs.info(&format!("Starting insights REST server on {addr}"), "insights-server").await;
  bentley::info!(&format!("Starting insights REST server on {addr}"));

  // Create the router with additional middleware
  let app = create_router().layer(
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

/// Get the path for LanceDB data storage
#[cfg(all(feature = "ml-features", not(tarpaulin_include)))] // Skip coverage - filesystem path operations
fn get_lancedb_data_path() -> std::path::PathBuf {
  dirs::home_dir()
    .unwrap_or_else(|| std::path::Path::new("/tmp").to_path_buf())
    .join(".blizz")
    .join("persistent")
    .join("insights")
    .join("lancedb")
}
