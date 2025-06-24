use crate::platform::{create_platform, GitPlatform, ReactionType};
use crate::session::load_current_session;
use anyhow::{anyhow, Result};

/// Configuration for acknowledgment reactions
#[derive(Debug, Clone)]
pub struct AcknowledgeConfig {
  pub reaction_type: ReactionType,
}

impl AcknowledgeConfig {
  /// Create config from CLI boolean flags
  pub fn from_flags(
    // 👍 flags
    thumbs_up: bool,
    ok: bool,
    yeah: bool,
    got_it: bool,
    // 👎 flags
    thumbs_down: bool,
    f_you: bool,
    // 😄 flags
    laugh: bool,
    smile: bool,
    // 🎉 flags
    hooray: bool,
    tada: bool,
    yay: bool,
    huzzah: bool,
    sarcastic_cheer: bool,
    // 😕 flags
    confused: bool,
    frown: bool,
    sad: bool,
    // ❤️ flags
    love: bool,
    heart: bool,
    favorite: bool,
    // 🚀 flags
    rocket: bool,
    zoom: bool,
    launch: bool,
    shipped: bool,
    sarcastic_ship_it: bool,
    // 👀 flags
    eyes: bool,
    looking: bool,
    surprise: bool,
  ) -> Self {
    // Array-based pattern matching - much cleaner than else-if chains!
    let flag_groups = [
      ([thumbs_up, ok, yeah, got_it].iter().any(|&f| f), ReactionType::ThumbsUp),
      ([thumbs_down, f_you].iter().any(|&f| f), ReactionType::ThumbsDown),
      ([laugh, smile].iter().any(|&f| f), ReactionType::Laugh),
      ([hooray, tada, yay, huzzah, sarcastic_cheer].iter().any(|&f| f), ReactionType::Hooray),
      ([confused, frown, sad].iter().any(|&f| f), ReactionType::Confused),
      ([love, heart, favorite].iter().any(|&f| f), ReactionType::Heart),
      ([rocket, zoom, launch, shipped, sarcastic_ship_it].iter().any(|&f| f), ReactionType::Rocket),
      ([eyes, looking, surprise].iter().any(|&f| f), ReactionType::Eyes),
    ];

    let reaction_type = flag_groups
      .iter()
      .find(|(is_set, _)| *is_set)
      .map(|(_, reaction)| reaction.clone())
      .unwrap_or(ReactionType::ThumbsUp); // Default to thumbs up

    Self { reaction_type }
  }

  /// Create config with a specific reaction type
  pub fn with_reaction(reaction_type: ReactionType) -> Self {
    Self { reaction_type }
  }
}

pub async fn handle(config: AcknowledgeConfig) -> Result<()> {
  let session = load_current_session()?;

  let current_thread_id =
    session.thread_queue.front().ok_or_else(|| anyhow!("No threads in queue"))?;

  // Use strategy pattern to create appropriate platform implementation
  let platform = create_platform(&session.platform).await?;

  let repo_parts: Vec<&str> = session.repository.full_name.split('/').collect();
  if repo_parts.len() != 2 {
    return Err(anyhow!("Invalid repository format: {}", session.repository.full_name));
  }

  let success = platform
    .add_reaction(repo_parts[0], repo_parts[1], current_thread_id, config.reaction_type.clone())
    .await?;

  if success {
    bentley::success(&format!("Added {} reaction to thread", config.reaction_type.emoji()));
  } else {
    bentley::warn("Failed to add reaction");
  }

  Ok(())
}
