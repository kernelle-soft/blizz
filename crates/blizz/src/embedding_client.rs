use anyhow::Result;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Embedding {
  pub version: String,
  pub created_at: DateTime<Utc>,
  pub embedding: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct Embeddings {
  pub embeddings: Vec<Embedding>,
}

impl Embedding {
  pub fn new(version: String, embedding: Vec<f32>) -> Self {
    Self {
      version,
      created_at: Utc::now(),
      embedding,
    }
  }
}

impl Embeddings {
  pub fn new(embeddings: Vec<Embedding>) -> Self {
    Self { embeddings }
  }
}

/// Generate a single embedding for the given text
pub async fn generate_embedding(text: &str) -> Result<Embedding> {
  #[cfg(feature = "neural")]
  {
    let embedding_vec = crate::commands::get_embedding_from_daemon(text).await?;
    let version = "all-MiniLM-L6-v2".to_string();
    Ok(Embedding::new(version, embedding_vec))
  }

  #[cfg(not(feature = "neural"))]
  {
    let _ = text;
    Err(anyhow::anyhow!("Neural features not enabled"))
  }
}

/// Generate embeddings for multiple texts
pub async fn generate_embeddings(texts: &[&str]) -> Result<Embeddings> {
  #[cfg(feature = "neural")]
  {
    let mut embeddings = Vec::new();
    let version = "all-MiniLM-L6-v2".to_string();
    
    for text in texts {
      let embedding_vec = crate::commands::get_embedding_from_daemon(text).await?;
      embeddings.push(Embedding::new(version.clone(), embedding_vec));
    }
    
    Ok(Embeddings::new(embeddings))
  }

  #[cfg(not(feature = "neural"))]
  {
    let _ = texts;
    Err(anyhow::anyhow!("Neural features not enabled"))
  }
}

/// Compute and set embedding for an insight (convenience function)
pub async fn compute_for_insight(insight: &mut crate::insight::Insight) -> Result<()> {
  #[cfg(feature = "neural")]
  {
    let embedding_text = insight.get_embedding_text();
    let embedding = generate_embedding(&embedding_text).await?;
    
    insight.set_embedding(
      embedding.version,
      embedding.embedding,
      embedding_text,
    );
    
    Ok(())
  }

  #[cfg(not(feature = "neural"))]
  {
    let _ = insight;
    Ok(())
  }
}