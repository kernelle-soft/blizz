use anyhow::{anyhow, Result};
use crate::session::SessionManager;
use crate::platform::{ReactionType, GitPlatform, github::GitHubPlatform};

pub async fn handle(
  thumbs_up: bool,
  ok: bool,
  yeah: bool,
  got_it: bool,
  thumbs_down: bool,
  f_you: bool,
  laugh: bool,
  smile: bool,
  hooray: bool,
  tada: bool,
  yay: bool,
  huzzah: bool,
  sarcastic_cheer: bool,
  confused: bool,
  frown: bool,
  sad: bool,
  love: bool,
  heart: bool,
  favorite: bool,
  rocket: bool,
  zoom: bool,
  launch: bool,
  shipped: bool,
  sarcastic_ship_it: bool,
  eyes: bool,
  looking: bool,
  surprise: bool,
) -> Result<()> {
  // Determine which reaction to use based on flags
  let reaction = if thumbs_up || ok || yeah || got_it {
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
    // Default to eyes if no flags specified
    ReactionType::ThumbsUp
  };

  let session_manager = SessionManager::new()?;
  let session = session_manager.load_session()?
    .ok_or_else(|| anyhow!("No active review session found"))?;

  if session.platform != "github" {
    return Err(anyhow!("Reaction system currently only supported for GitHub"));
  }

  // Get the current thread
  let current_thread_id = session.thread_queue.front()
    .ok_or_else(|| anyhow!("No threads in queue"))?;

  // Create GitHub client with credential lookup
  let github = GitHubPlatform::new().await?;

  let repo_parts: Vec<&str> = session.repository.full_name.split('/').collect();
  if repo_parts.len() != 2 {
    return Err(anyhow!("Invalid repository format"));
  }

  let success = github.add_reaction(
    repo_parts[0],
    repo_parts[1], 
    current_thread_id,
    reaction.clone()
  ).await?;

  if success {
    bentley::success(&format!("Added {} reaction to thread", reaction.emoji()));
  } else {
    bentley::warn("Failed to add reaction");
  }

  Ok(())
} 