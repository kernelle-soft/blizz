pub mod search;
pub mod similarity;

#[cfg(feature = "ml-features")]
pub mod vector_database;
#[cfg(feature = "ml-features")]
pub mod embeddings;
#[cfg(feature = "ml-features")]
pub mod lancedb;