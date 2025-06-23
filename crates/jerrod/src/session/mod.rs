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
  base_dir: std::path::PathBuf,
  current_session_path: Option<std::path::PathBuf>,
}

impl SessionManager {
  pub fn new() -> Result<Self> {
    let base_dir = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
      std::path::PathBuf::from(kernelle_dir)
    } else {
      dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".kernelle")
    };
    
    Ok(Self { 
      base_dir,
      current_session_path: None,
    })
  }

  pub fn with_session_context(
    &mut self, 
    platform: &str, 
    repository_path: &str, 
    mr_number: u64
  ) -> Result<()> {
    let platform_dir = match platform {
      "github" => "github",
      "gitlab" => "gitlab", 
      _ => return Err(anyhow::anyhow!("Unsupported platform: {}", platform)),
    };

    let session_dir = self.base_dir
      .join("code-reviews")
      .join(platform_dir)
      .join(repository_path)
      .join(mr_number.to_string());
    
    std::fs::create_dir_all(&session_dir)?;
    self.current_session_path = Some(session_dir);
    Ok(())
  }

  fn get_session_path(&self) -> Result<&std::path::PathBuf> {
    self.current_session_path
      .as_ref()
      .ok_or_else(|| anyhow::anyhow!("No session context set. Call with_session_context() first."))
  }

  fn get_legacy_session_path(&self) -> std::path::PathBuf {
    self.base_dir.join("code-reviews").join("session.json")
  }

  pub fn session_exists(&self) -> bool {
    if let Ok(session_path) = self.get_session_path() {
      session_path.join("session.json").exists()
    } else {
      self.get_legacy_session_path().exists()
    }
  }

  pub fn save_session(&self, session: &ReviewSession) -> Result<()> {
    let session_file = self.get_session_path()?.join("session.json");
    let json = serde_json::to_string_pretty(session)?;
    std::fs::write(session_file, json)?;
    Ok(())
  }

  pub fn load_session(&mut self) -> Result<Option<ReviewSession>> {
    if let Ok(session_path) = self.get_session_path() {
      let session_file = session_path.join("session.json");
      if session_file.exists() {
        let json = std::fs::read_to_string(session_file)?;
        let session: ReviewSession = serde_json::from_str(&json)?;
        return Ok(Some(session));
      }
    }

    let legacy_path = self.get_legacy_session_path();
    if legacy_path.exists() {
      let json = std::fs::read_to_string(&legacy_path)?;
      let session: ReviewSession = serde_json::from_str(&json)?;
      
      self.with_session_context(
        &session.platform,
        &session.repository.full_name,
        session.merge_request.number,
      )?;
      
      let new_session_file = self.get_session_path()?.join("session.json");
      let json = serde_json::to_string_pretty(&session)?;
      std::fs::write(new_session_file, json)?;
      
      std::fs::remove_file(legacy_path)?;
      bentley::info("Migrated session to new organized structure");
      
      return Ok(Some(session));
    }

    Ok(None)
  }

  pub fn clear_session(&self) -> Result<()> {
    if let Ok(session_path) = self.get_session_path() {
      let session_file = session_path.join("session.json");
      if session_file.exists() {
        std::fs::remove_file(session_file)?;
      }
    }

    let legacy_path = self.get_legacy_session_path();
    if legacy_path.exists() {
      std::fs::remove_file(legacy_path)?;
    }
    
    Ok(())
  }
}
