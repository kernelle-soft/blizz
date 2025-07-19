use crate::platform::{Discussion, MergeRequest, Pipeline, Repository};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Default)]
pub struct ReviewSessionOptions {
  pub host: Option<String>, // For custom/self-hosted instances
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewSession {
  pub repository: Repository,
  pub merge_request: MergeRequest,
  pub platform: String,
  #[serde(default)]
  pub host: Option<String>, // For custom/self-hosted instances
  pub thread_queue: VecDeque<String>,
  pub unresolved_threads: Vec<String>,
  pub discussions: std::collections::HashMap<String, Discussion>,
  pub pipelines: Vec<Pipeline>,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl ReviewSession {
  /// Create a new review session
  pub fn new(
    repository: Repository,
    merge_request: MergeRequest,
    platform: String,
    discussions: Vec<Discussion>,
    pipelines: Vec<Pipeline>,
    options: ReviewSessionOptions,
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
      host: options.host,
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
    eprintln!("[DEBUG] SessionManager::new: starting");
    
    let base_dir = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
      eprintln!("[DEBUG] SessionManager::new: using KERNELLE_DIR = {}", kernelle_dir);
      std::path::PathBuf::from(kernelle_dir)
    } else {
      let home_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
      eprintln!("[DEBUG] SessionManager::new: using home_dir = {:?}", home_dir);
      let base = home_dir.join(".kernelle");
      eprintln!("[DEBUG] SessionManager::new: computed base_dir = {:?}", base);
      base
    };

    eprintln!("[DEBUG] SessionManager::new: final base_dir = {:?}", base_dir);

    // Ensure the base directory exists
    eprintln!("[DEBUG] SessionManager::new: creating base directory");
    std::fs::create_dir_all(&base_dir)?;
    eprintln!("[DEBUG] SessionManager::new: base directory created successfully");

    eprintln!("[DEBUG] SessionManager::new: completed successfully");
    Ok(Self { base_dir, current_session_path: None })
  }

  pub fn with_session_context(
    &mut self,
    platform: &str,
    repository_path: &str,
    mr_number: u64,
  ) -> Result<()> {
    eprintln!("[DEBUG] with_session_context: platform={}, repo={}, mr={}", platform, repository_path, mr_number);
    eprintln!("[DEBUG] base_dir: {:?}", self.base_dir);
    
    let platform_dir = match platform {
      "github" => "github",
      "gitlab" => "gitlab",
      _ => return Err(anyhow::anyhow!("Unsupported platform: {}", platform)),
    };

    // Sanitize repository path for cross-platform compatibility
    // Keep forward slashes (legitimate in repo paths) but replace other problematic characters
    let sanitized_repo_path = repository_path
      .replace(['\\', ':', '*', '?', '"', '<', '>', '|'], "_")
      .trim_matches('/')
      .to_string();

    eprintln!("[DEBUG] sanitized_repo_path: '{}'", sanitized_repo_path);

    if sanitized_repo_path.is_empty() {
      return Err(anyhow::anyhow!("Invalid repository path: '{}'", repository_path));
    }

    let session_dir = self
      .base_dir
      .join("code-reviews")
      .join(platform_dir)
      .join(&sanitized_repo_path)
      .join(mr_number.to_string());

    eprintln!("[DEBUG] target session_dir: {:?}", session_dir);

    // Create intermediate directories step by step for better error reporting
    let code_reviews_dir = self.base_dir.join("code-reviews");
    eprintln!("[DEBUG] creating code_reviews_dir: {:?}", code_reviews_dir);
    std::fs::create_dir_all(&code_reviews_dir)
      .map_err(|e| anyhow::anyhow!("Failed to create code-reviews directory {:?}: {}", code_reviews_dir, e))?;

    let platform_dir_path = code_reviews_dir.join(platform_dir);
    eprintln!("[DEBUG] creating platform_dir_path: {:?}", platform_dir_path);
    std::fs::create_dir_all(&platform_dir_path)
      .map_err(|e| anyhow::anyhow!("Failed to create platform directory {:?}: {}", platform_dir_path, e))?;

    let repo_dir = platform_dir_path.join(&sanitized_repo_path);
    eprintln!("[DEBUG] creating repo_dir: {:?}", repo_dir);
    std::fs::create_dir_all(&repo_dir)
      .map_err(|e| anyhow::anyhow!("Failed to create repository directory {:?}: {}", repo_dir, e))?;

    // Final session directory creation
    eprintln!("[DEBUG] creating final session_dir: {:?}", session_dir);
    std::fs::create_dir_all(&session_dir)
      .map_err(|e| anyhow::anyhow!("Failed to create session directory {:?}: {}", session_dir, e))?;
    
    // Verify the directory was actually created and is accessible
    eprintln!("[DEBUG] verifying session_dir exists: {:?}", session_dir);
    if !session_dir.exists() {
      return Err(anyhow::anyhow!("Session directory was not created successfully: {:?}", session_dir));
    }

    eprintln!("[DEBUG] session_dir verified, setting current_session_path");
    self.current_session_path = Some(session_dir);
    eprintln!("[DEBUG] with_session_context completed successfully");
    Ok(())
  }

  fn get_session_path(&self) -> Result<&std::path::PathBuf> {
    self
      .current_session_path
      .as_ref()
      .ok_or_else(|| anyhow::anyhow!("No session context set. Call with_session_context() first."))
  }

  fn get_legacy_session_path(&self) -> std::path::PathBuf {
    self.base_dir.join("code-reviews").join("session.json")
  }

  #[allow(dead_code)]
  pub fn session_exists(&self) -> bool {
    if let Ok(session_path) = self.get_session_path() {
      session_path.join("session.json").exists()
    } else {
      self.get_legacy_session_path().exists()
    }
  }

  pub fn save_session(&self, session: &ReviewSession) -> Result<()> {
    eprintln!("[DEBUG] save_session: starting");
    
    let session_path = self.get_session_path()?;
    eprintln!("[DEBUG] save_session: session_path = {:?}", session_path);
    
    let session_file = session_path.join("session.json");
    eprintln!("[DEBUG] save_session: session_file = {:?}", session_file);

    // Ensure the session directory exists with robust cross-platform directory creation
    eprintln!("[DEBUG] save_session: checking if session_path exists");
    if !session_path.exists() {
      eprintln!("[DEBUG] save_session: session_path doesn't exist, creating it");
      std::fs::create_dir_all(session_path)?;
      eprintln!("[DEBUG] save_session: session_path created successfully");
    } else {
      eprintln!("[DEBUG] save_session: session_path already exists");
    }
    
    // Ensure parent directory exists for the session file
    if let Some(parent) = session_file.parent() {
      eprintln!("[DEBUG] save_session: checking parent directory: {:?}", parent);
      if !parent.exists() {
        eprintln!("[DEBUG] save_session: parent directory doesn't exist, creating it");
        std::fs::create_dir_all(parent)?;
        eprintln!("[DEBUG] save_session: parent directory created successfully");
      } else {
        eprintln!("[DEBUG] save_session: parent directory already exists");
      }
    }

    eprintln!("[DEBUG] save_session: serializing session data");
    let json = serde_json::to_string_pretty(session)?;
    eprintln!("[DEBUG] save_session: writing to file: {:?}", session_file);
    std::fs::write(session_file, json)?;
    eprintln!("[DEBUG] save_session: file written successfully");
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

/// Centralized session discovery that can find sessions automatically
pub struct SessionDiscovery {
  base_dir: std::path::PathBuf,
}

impl SessionDiscovery {
  pub fn new() -> Result<Self> {
    let base_dir = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
      std::path::PathBuf::from(kernelle_dir)
    } else {
      dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".kernelle")
    };

    Ok(Self { base_dir })
  }

  /// Find any existing session automatically
  pub fn find_any_session(&self) -> Result<Option<SessionManager>> {
    // First check legacy location
    let legacy_path = self.base_dir.join("code-reviews").join("session.json");
    if legacy_path.exists() {
      bentley::info("Found legacy session, will be migrated on load");
      let manager = SessionManager::new()?;
      return Ok(Some(manager));
    }

    // Then search organized structure
    let code_reviews_dir = self.base_dir.join("code-reviews");
    if !code_reviews_dir.exists() {
      return Ok(None);
    }

    // Search through platforms (github, gitlab)
    for platform_entry in std::fs::read_dir(&code_reviews_dir)? {
      let platform_entry = platform_entry?;
      if !platform_entry.file_type()?.is_dir() {
        continue;
      }

      let platform_name = platform_entry.file_name().to_string_lossy().to_string();
      if platform_name == "session.json" {
        continue; // Skip legacy file
      }

      let platform_dir = platform_entry.path();

      // Search through organization/user directories
      for org_entry in std::fs::read_dir(&platform_dir)? {
        let org_entry = org_entry?;
        if !org_entry.file_type()?.is_dir() {
          continue;
        }

        let org_dir = org_entry.path();

        // Search through repository directories
        for repo_entry in std::fs::read_dir(&org_dir)? {
          let repo_entry = repo_entry?;
          if !repo_entry.file_type()?.is_dir() {
            continue;
          }

          let repo_dir = repo_entry.path();

          // Search through MR number directories
          for mr_entry in std::fs::read_dir(&repo_dir)? {
            let mr_entry = mr_entry?;
            if !mr_entry.file_type()?.is_dir() {
              continue;
            }

            let session_file = mr_entry.path().join("session.json");
            if session_file.exists() {
              // Found a session! Set up manager with this context
              let mut manager = SessionManager::new()?;

              // Extract context from path
              let mr_number: u64 = mr_entry
                .file_name()
                .to_string_lossy()
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid MR number in path"))?;

              let repo_name = repo_entry.file_name().to_string_lossy().to_string();
              let org_name = org_entry.file_name().to_string_lossy().to_string();
              let repository_path = format!("{org_name}/{repo_name}");

              manager.with_session_context(&platform_name, &repository_path, mr_number)?;

              bentley::info(&format!(
                "Found session: {repository_path} #{mr_number} ({platform_name})"
              ));

              return Ok(Some(manager));
            }
          }
        }
      }
    }

    Ok(None)
  }

  /// Get a SessionManager configured for the found session, or None if no session exists
  pub fn get_session_manager(&self) -> Result<Option<SessionManager>> {
    self.find_any_session()
  }
}

/// Convenience function to get a configured SessionManager or error if no session exists
pub fn get_session_manager() -> Result<SessionManager> {
  let discovery = SessionDiscovery::new()?;
  discovery
    .get_session_manager()?
    .ok_or_else(|| anyhow::anyhow!("No active review session found. Use 'jerrod start' to begin."))
}

/// Convenience function to load the current session or error if none exists
pub fn load_current_session() -> Result<ReviewSession> {
  let mut manager = get_session_manager()?;
  manager
    .load_session()?
    .ok_or_else(|| anyhow::anyhow!("No active review session found. Use 'jerrod start' to begin."))
}
