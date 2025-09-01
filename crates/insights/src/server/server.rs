//! REST server startup and configuration

use anyhow::Result;
use axum::serve;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::server::routing::create_router;

/// Start the REST server
pub async fn start_server(addr: SocketAddr) -> Result<()> {
  bentley::info!(&format!("Starting insights REST server on {addr}"));

  // Create the router with all endpoints
  let app = create_router().layer(
    ServiceBuilder::new().layer(TraceLayer::new_for_http()).layer(CorsLayer::permissive()), // TODO: Configure CORS properly for production
  );

  // Create listener
  let listener = TcpListener::bind(addr).await?;
  bentley::info!(&format!("Server listening on {addr}"));

  // Start serving
  serve(listener, app).await.map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

  Ok(())
}
