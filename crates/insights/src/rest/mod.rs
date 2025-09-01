//! REST API module for insights service
//!
//! Provides HTTP REST endpoints for the insights knowledge management system.
//! Uses axum for routing and schemars for OpenAPI documentation generation.

pub mod handlers;
pub mod middleware;
pub mod routing;
pub mod server;
pub mod types;
