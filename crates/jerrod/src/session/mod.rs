use crate::platform::{Discussion, MergeRequest, Pipeline, Repository};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewSession {
  pub repository: Repository,
  pub merge_request: MergeRequest,
  pub platform: String,
      pub thread_queue: VecDeque<String>,
    pub unresolved_threads: Vec<String>,
    pub discussions: std::collections::HashMap<String, Discussion>,
  pub pipelines: Vec<Pipeline>,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl ReviewSession {
  pub fn new(
    repository: Repository,
    merge_request: MergeRequest,
    platform: String,
    discussions: Vec<Discussion>,
    pipelines: Vec<Pipeline>,
  ) -> Self {
    let mut discussion_map = std::collections::HashMap::new();
    let mut thread_queue = VecDeque::new();

    for discussion in discussions {
      thread_queue.push_back(discussion.id.clone());
      discussion_map.insert(discussion.id.clone(), discussion);
    }

    let now = chrono::Utc::now();

    Self {
      repository,
      merge_request,
      platform,
      thread_queue,
      unresolved_threads: Vec::new(),
      discussions: discussion_map,
      pipelines,
      created_at: now,
      updated_at: now,
    }
  }

  /// Get the next thread in the queue without removing it
  pub fn peek_next_thread(&self) -> Option<&Discussion> {
    self.thread_queue.front().and_then(|id| self.discussions.get(id))
  }

  /// Remove and return the next thread from the queue
  pub fn pop_thread(&mut self, mark_unresolved: bool) -> Option<Discussion> {
    if let Some(thread_id) = self.thread_queue.pop_front() {
      if mark_unresolved {
        self.unresolved_threads.push(thread_id.clone());
      }
      self.updated_at = chrono::Utc::now();
      self.discussions.get(&thread_id).cloned()
    } else {
      None
    }
  }

  /// Get the number of threads remaining in the queue
  pub fn threads_remaining(&self) -> usize {
    self.thread_queue.len()
  }

  /// Check if there are any unresolved threads
  pub fn has_unresolved_threads(&self) -> bool {
    !self.unresolved_threads.is_empty()
  }
}

/// Manages review session persistence and lifecycle
pub struct SessionManager {
  session_dir: std::path::PathBuf,
}

impl SessionManager {
  pub fn new() -> Result<Self> {
    let base_dir = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
      std::path::PathBuf::from(kernelle_dir)
    } else {
      dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join("kernelle")
    };
    
    let session_dir = base_dir.join("code-reviews");
    std::fs::create_dir_all(&session_dir)?;
    Ok(Self { session_dir })
  }

  pub fn session_exists(&self) -> bool {
    self.session_dir.join("session.json").exists()
  }

  pub fn save_session(&self, session: &ReviewSession) -> Result<()> {
    let session_file = self.session_dir.join("session.json");
    let json = serde_json::to_string_pretty(session)?;
    std::fs::write(session_file, json)?;
    Ok(())
  }

  pub fn load_session(&self) -> Result<Option<ReviewSession>> {
    let session_file = self.session_dir.join("session.json");
    if !session_file.exists() {
      return Ok(None);
    }

    let json = std::fs::read_to_string(session_file)?;
    let session: ReviewSession = serde_json::from_str(&json)?;
    Ok(Some(session))
  }

  pub fn clear_session(&self) -> Result<()> {
    let session_file = self.session_dir.join("session.json");
    if session_file.exists() {
      std::fs::remove_file(session_file)?;
    }
    Ok(())
  }
}
