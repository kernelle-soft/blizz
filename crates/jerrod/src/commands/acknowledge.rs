use crate::platform::{create_platform, PlatformOptions, ReactionType};
use crate::session::load_current_session;
use anyhow::{anyhow, Result};

/// Configuration for acknowledgment reactions
#[derive(Debug, Clone)]
pub struct AcknowledgeConfig {
  pub reaction_type: ReactionType,
}

/// Raw flag input for acknowledge reactions
#[derive(Debug, Default)]
pub struct AcknowledgeFlags {
  // 👍 flags
  pub thumbs_up: bool,
  pub ok: bool,
  pub yeah: bool,
  pub got_it: bool,
  // 👎 flags
  pub thumbs_down: bool,
  pub f_you: bool,
  // 😄 flags
  pub laugh: bool,
  pub smile: bool,
  // 🎉 flags
  pub hooray: bool,
  pub tada: bool,
  pub yay: bool,
  pub huzzah: bool,
  pub sarcastic_cheer: bool,
  // 😕 flags
  pub confused: bool,
  pub frown: bool,
  pub sad: bool,
  // ❤️ flags
  pub love: bool,
  pub heart: bool,
  pub favorite: bool,
  // 🚀 flags
  pub rocket: bool,
  pub zoom: bool,
  pub launch: bool,
  pub shipped: bool,
  pub sarcastic_ship_it: bool,
  // 👀 flags
  pub eyes: bool,
  pub looking: bool,
  pub surprise: bool,
}

impl AcknowledgeConfig {
  /// Create config from CLI boolean flags
  pub fn from_flags(flags: AcknowledgeFlags) -> Self {
    // Array-based pattern matching - much cleaner than else-if chains!
    let flag_groups = [
      (
        [flags.thumbs_up, flags.ok, flags.yeah, flags.got_it].iter().any(|&f| f),
        ReactionType::ThumbsUp,
      ),
      ([flags.thumbs_down, flags.f_you].iter().any(|&f| f), ReactionType::ThumbsDown),
      ([flags.laugh, flags.smile].iter().any(|&f| f), ReactionType::Laugh),
      (
        [flags.hooray, flags.tada, flags.yay, flags.huzzah, flags.sarcastic_cheer]
          .iter()
          .any(|&f| f),
        ReactionType::Hooray,
      ),
      ([flags.confused, flags.frown, flags.sad].iter().any(|&f| f), ReactionType::Confused),
      ([flags.love, flags.heart, flags.favorite].iter().any(|&f| f), ReactionType::Heart),
      (
        [flags.rocket, flags.zoom, flags.launch, flags.shipped, flags.sarcastic_ship_it]
          .iter()
          .any(|&f| f),
        ReactionType::Rocket,
      ),
      ([flags.eyes, flags.looking, flags.surprise].iter().any(|&f| f), ReactionType::Eyes),
    ];

    let reaction_type = flag_groups
      .iter()
      .find(|(is_set, _)| *is_set)
      .map(|(_, reaction)| reaction.clone())
      .unwrap_or(ReactionType::ThumbsUp); // Default to thumbs up

    Self { reaction_type }
  }

  /// Create config with a specific reaction type
  #[allow(dead_code)]
  pub fn with_reaction(reaction_type: ReactionType) -> Self {
    Self { reaction_type }
  }
}

pub async fn handle(config: AcknowledgeConfig) -> Result<()> {
  let session = load_current_session()?;

  let current_thread_id =
    session.thread_queue.front().ok_or_else(|| anyhow!("No threads in queue"))?;

  // Use strategy pattern to create appropriate platform implementation
  let platform = create_platform(&session.platform, PlatformOptions { host: session.host }).await?;

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
