//! Insights - Knowledge Management and Storage System
//!
//! A high-performance knowledge management system providing structured insight
//! storage and retrieval for development workflows and team collaboration.

pub mod commands;
#[cfg(feature = "neural")]
pub mod embedding_client;
#[cfg(feature = "neural")]
pub mod embedding_model;
pub mod insight;
pub mod search;
#[cfg(feature = "semantic")]
pub mod semantic;
pub mod similarity;
