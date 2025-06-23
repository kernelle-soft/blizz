use anyhow::{anyhow, Result};
use crate::session::load_current_session;
use crate::platform::{ReactionType, GitPlatform, github::GitHubPlatform};

/// Configuration for acknowledgment reactions
#[derive(Debug, Clone)]
pub struct AcknowledgeConfig {
  pub reaction_type: ReactionType,
}

impl AcknowledgeConfig {
  /// Create config from CLI boolean flags
  pub fn from_flags(
    // 👍 flags
    thumbs_up: bool, ok: bool, yeah: bool, got_it: bool,
    // 👎 flags  
    thumbs_down: bool, f_you: bool,
    // 😄 flags
    laugh: bool, smile: bool,
    // 🎉 flags
    hooray: bool, tada: bool, yay: bool, huzzah: bool, sarcastic_cheer: bool,
    // 😕 flags
    confused: bool, frown: bool, sad: bool,
    // ❤️ flags
    love: bool, heart: bool, favorite: bool,
    // 🚀 flags
    rocket: bool, zoom: bool, launch: bool, shipped: bool, sarcastic_ship_it: bool,
    // 👀 flags
    eyes: bool, looking: bool, surprise: bool,
  ) -> Self {
    let reaction_type = if thumbs_up || ok || yeah || got_it {
      ReactionType::ThumbsUp
    } else if thumbs_down || f_you {
      ReactionType::ThumbsDown
    } else if laugh || smile {
      ReactionType::Laugh
    } else if hooray || tada || yay || huzzah || sarcastic_cheer {
      ReactionType::Hooray
    } else if confused || frown || sad {
      ReactionType::Confused
    } else if love || heart || favorite {
      ReactionType::Heart
    } else if rocket || zoom || launch || shipped || sarcastic_ship_it {
      ReactionType::Rocket
    } else if eyes || looking || surprise {
      ReactionType::Eyes
    } else {
      // Default to thumbs up
      ReactionType::ThumbsUp
    };

    Self { reaction_type }
  }

  /// Create config with a specific reaction type
  pub fn with_reaction(reaction_type: ReactionType) -> Self {
    Self { reaction_type }
  }
}

pub async fn handle(config: AcknowledgeConfig) -> Result<()> {
  let session = load_current_session()?;

  if session.platform != "github" {
    return Err(anyhow!("Reaction system currently only supported for GitHub"));
  }

  let current_thread_id = session.thread_queue.front()
    .ok_or_else(|| anyhow!("No threads in queue"))?;

  let github = GitHubPlatform::new().await?;

  let repo_parts: Vec<&str> = session.repository.full_name.split('/').collect();
  if repo_parts.len() != 2 {
    return Err(anyhow!("Invalid repository format"));
  }

  let success = github.add_reaction(
    repo_parts[0],
    repo_parts[1], 
    current_thread_id,
    config.reaction_type.clone()
  ).await?;

  if success {
    bentley::success(&format!("Added {} reaction to thread", config.reaction_type.emoji()));
  } else {
    bentley::warn("Failed to add reaction");
  }

  Ok(())
} 