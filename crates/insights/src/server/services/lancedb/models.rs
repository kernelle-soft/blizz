//! Data models for LanceDB operations

use serde::{Deserialize, Serialize};

/// Record structure for storing in LanceDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightRecord {
    pub id: String,
    pub topic: String,  
    pub name: String,
    pub overview: String,
    pub details: String,
    pub embedding: Vec<f32>,
    pub created_at: String,
    pub updated_at: String,
}

impl InsightRecord {
    pub fn new(
        topic: String,
        name: String, 
        overview: String,
        details: String,
        embedding: Vec<f32>,
        created_at: String,
        updated_at: String,
    ) -> Self {
        let id = format!("{topic}:{name}");
        Self {
            id,
            topic,
            name,
            overview,
            details,
            embedding,
            created_at,
            updated_at,
        }
    }
}

/// Result of an embedding similarity search
#[derive(Debug, Clone)]
pub struct EmbeddingSearchResult {
    pub id: String,
    pub topic: String,
    pub name: String,
    pub overview: String,
    pub details: String,
    pub similarity: f32,
}
