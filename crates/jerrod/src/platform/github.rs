use crate::platform::{
  Discussion, FileDiff, GitPlatform, MergeRequest, MergeRequestState, Note, Pipeline, ReactionType, Repository,
  User,
};
use crate::auth::get_github_token;
use anyhow::{anyhow, Result};
use octocrab::Octocrab;

pub struct GitHubPlatform {
  #[allow(dead_code)]
  client: Octocrab,
}

impl GitHubPlatform {
  /// Create a new GitHub platform client with authentication
  pub async fn new() -> Result<Self> {
    let token = get_github_token().await?;
    let client = Octocrab::builder().personal_token(token).build()?;
    Ok(Self { client })
  }

  /// Create a GitHub platform client from an existing Octocrab instance
  #[allow(dead_code)]
  pub fn from_client(client: Octocrab) -> Self {
    Self { client }
  }
}

#[async_trait::async_trait]
impl GitPlatform for GitHubPlatform {
  async fn get_repository(&self, owner: &str, repo: &str) -> Result<Repository> {
    let repo_data = self.client.repos(owner, repo).get().await?;
    
    Ok(Repository {
      owner: owner.to_string(),
      name: repo.to_string(),
      full_name: repo_data.full_name.unwrap_or_else(|| format!("{}/{}", owner, repo)),
      url: repo_data.html_url.map(|u| u.to_string()).unwrap_or_else(|| format!("https://github.com/{}/{}", owner, repo)),
    })
  }

  async fn get_merge_request(&self, owner: &str, repo: &str, number: u64) -> Result<MergeRequest> {
    let pr = self.client.pulls(owner, repo).get(number).await?;
    
    let state = match pr.state {
      Some(octocrab::models::IssueState::Open) => {
        if pr.draft == Some(true) {
          MergeRequestState::Draft
        } else {
          MergeRequestState::Open
        }
      }
      Some(octocrab::models::IssueState::Closed) => {
        if pr.merged_at.is_some() {
          MergeRequestState::Merged
        } else {
          MergeRequestState::Closed
        }
      }
      Some(_) => MergeRequestState::Open, // Handle any other states
      None => MergeRequestState::Open,
    };

    let author = if let Some(user) = pr.user {
      User {
        id: user.id.to_string(),
        username: user.login.clone(),
        display_name: user.name.clone().unwrap_or(user.login.clone()),
        avatar_url: Some(user.avatar_url.to_string()),
      }
    } else {
      User {
        id: "unknown".to_string(),
        username: "unknown".to_string(),
        display_name: "Unknown User".to_string(),
        avatar_url: None,
      }
    };

    let assignee = pr.assignee.map(|assignee| User {
      id: assignee.id.to_string(),
      username: assignee.login.clone(),
      display_name: assignee.name.unwrap_or(assignee.login.clone()),
      avatar_url: Some(assignee.avatar_url.to_string()),
    });

    Ok(MergeRequest {
      id: pr.id.to_string(),
      number,
      title: pr.title.unwrap_or_else(|| format!("Pull Request #{}", number)),
      description: pr.body,
      state,
      author,
      assignee,
      source_branch: pr.head.ref_field,
      target_branch: pr.base.ref_field,
      url: pr.html_url.map(|u| u.to_string()).unwrap_or_else(|| format!("https://github.com/{}/{}/pull/{}", owner, repo, number)),
      created_at: pr.created_at.unwrap_or_else(chrono::Utc::now),
      updated_at: pr.updated_at.unwrap_or_else(chrono::Utc::now),
    })
  }

  async fn get_discussions(
    &self,
    owner: &str,
    repo: &str,
    number: u64,
  ) -> Result<Vec<Discussion>> {
    let mut discussions = Vec::new();

    // Fetch regular issue comments (top-level comments on the PR)
    let issue_comments = self.client.issues(owner, repo).list_comments(number).send().await?;
    
    for comment in issue_comments {
      // Skip comments that have emoji reactions (already processed/acknowledged)
      if let Ok(reactions) = self.get_reactions(owner, repo, &comment.id.to_string()).await {
        if !reactions.is_empty() {
          bentley::info(&format!("Skipping comment {} with emoji reactions", comment.id));
          continue;
        }
      }

      let author = User {
        id: comment.user.id.to_string(),
        username: comment.user.login.clone(),
        display_name: comment.user.name.clone().unwrap_or(comment.user.login.clone()),
        avatar_url: Some(comment.user.avatar_url.to_string()),
      };

      let note = Note {
        id: comment.id.to_string(),
        author,
        body: comment.body.unwrap_or_default(),
        created_at: comment.created_at,
        updated_at: comment.updated_at.unwrap_or(comment.created_at),
      };

      discussions.push(Discussion {
        id: comment.id.to_string(),
        resolved: false, // Issue comments don't have resolved status
        resolvable: false,
        file_path: None,
        line_number: None,
        notes: vec![note],
      });
    }

    // Fetch pull request review comments (inline code comments)
    let review_comments = self.client.pulls(owner, repo).list_comments(Some(number)).send().await?;
    
    for comment in review_comments {
      if let Some(user) = comment.user {
        // Skip review comments that have emoji reactions (already processed/acknowledged)
        if let Ok(reactions) = self.get_reactions(owner, repo, &comment.id.to_string()).await {
          if !reactions.is_empty() {
            bentley::info(&format!("Skipping review comment {} with emoji reactions", comment.id));
            continue;
          }
        }

        let author = User {
          id: user.id.to_string(),
          username: user.login.clone(),
          display_name: user.name.clone().unwrap_or(user.login.clone()),
          avatar_url: Some(user.avatar_url.to_string()),
        };

        let note = Note {
          id: comment.id.to_string(),
          author,
          body: comment.body,
          created_at: comment.created_at,
          updated_at: comment.updated_at,
        };

        discussions.push(Discussion {
          id: comment.id.to_string(),
          resolved: false, // We'll need to check conversation status separately
          resolvable: true, // Review comments can be part of conversations
          file_path: Some(comment.path),
          line_number: comment.line.map(|line| line as u32),
          notes: vec![note],
        });
      }
    }

    Ok(discussions)
  }

  async fn get_diffs(&self, owner: &str, repo: &str, number: u64) -> Result<Vec<FileDiff>> {
    let files = self.client.pulls(owner, repo).list_files(number).await?;
    
    let mut diffs = Vec::new();
    for file in files {
      diffs.push(FileDiff {
        old_path: if file.previous_filename.as_ref() != Some(&file.filename) {
          file.previous_filename.or_else(|| Some("/dev/null".to_string()))
        } else {
          None
        },
        new_path: file.filename,
        diff: file.patch.unwrap_or_default(),
      });
    }
    
    Ok(diffs)
  }

  async fn get_pipelines(&self, _owner: &str, _repo: &str, _sha: &str) -> Result<Vec<Pipeline>> {
    // TODO: Implement workflow runs fetching - the Octocrab API for this is complex
    bentley::info("GitHub workflows fetch not yet implemented");
    Ok(vec![])
  }

  async fn add_comment(
    &self,
    owner: &str,
    repo: &str,
    discussion_id: &str,
    text: &str,
  ) -> Result<Note> {
    // For GitHub, discussion_id is the PR number for issue comments
    // or comment_id for replies (which GitHub doesn't support directly)
    
    // Parse as PR number first
    let pr_number: u64 = discussion_id.parse()
      .map_err(|_| anyhow!("Invalid PR number: {}", discussion_id))?;
    
    // Create the comment using GitHub Issues API
    let comment = self.client
      .issues(owner, repo)
      .create_comment(pr_number, text)
      .await
      .map_err(|e| anyhow!("Failed to create comment: {:?}", e))?;
    
    // Convert the response to our Note format
    let author = User {
      id: comment.user.id.to_string(),
      username: comment.user.login.clone(),
      display_name: comment.user.name.clone().unwrap_or(comment.user.login.clone()),
      avatar_url: Some(comment.user.avatar_url.to_string()),
    };

    Ok(Note {
      id: comment.id.to_string(),
      author,
      body: comment.body.unwrap_or_default(),
      created_at: comment.created_at,
      updated_at: comment.updated_at.unwrap_or(comment.created_at),
    })
  }

  async fn resolve_discussion(
    &self,
    owner: &str,
    repo: &str,
    discussion_id: &str,
  ) -> Result<bool> {
    // GitHub uses GraphQL for resolving conversations
    // We need to use the review thread ID, not the comment ID
    
    // For now, we'll implement a simpler approach since GitHub's conversation resolution
    // is specifically for review comments on pull requests, not issue comments
    // Issue comments don't have a "resolved" state like GitLab threads do
    
    bentley::info(&format!(
      "GitHub conversation resolution requested for comment {} in {}/{}",
      discussion_id, owner, repo
    ));
    
    // GitHub doesn't support resolving issue-level comments like GitLab does
    // Only review comments on pull requests can be resolved, and that requires
    // the review thread ID, not the comment ID
    
    // For our workflow, we'll use reactions as the resolution mechanism instead
    bentley::info("GitHub uses reactions for comment state tracking instead of resolution");
    bentley::info("Use 'jerrod acknowledge' or comment flags to mark comments as handled");
    
    Ok(true) // Return true to indicate the operation was handled (via our reaction system)
  }

  async fn add_reaction(
    &self,
    owner: &str,
    repo: &str,
    comment_id: &str,
    reaction: ReactionType,
  ) -> Result<bool> {
    let comment_id: u64 = comment_id.parse()
      .map_err(|_| anyhow!("Invalid comment ID: {}", comment_id))?;
    
    // Convert to octocrab's reaction type
    let octocrab_reaction = match reaction.github_name() {
      "eyes" => octocrab::models::reactions::ReactionContent::Eyes,
      "heavy_check_mark" => octocrab::models::reactions::ReactionContent::Hooray, // closest match
      "question" => octocrab::models::reactions::ReactionContent::Confused,
      "memo" => octocrab::models::reactions::ReactionContent::Rocket, // closest match
      _ => return Err(anyhow!("Unsupported reaction type")),
    };

    self.client
      .issues(owner, repo)
      .create_comment_reaction(comment_id, octocrab_reaction)
      .await
      .map_err(|e| {
        // Check if this is a permission error and provide helpful guidance
        let error_msg = format!("{:?}", e);
        if error_msg.contains("403") || error_msg.contains("Resource not accessible") {
          anyhow!(
            "Failed to add reaction {} to comment {}: Permission denied.\n\n\
            Possible causes:\n\
            1. Token missing 'public_repo' scope (or 'repo' for private repos)\n\
            2. Token type mismatch (fine-grained vs classic)\n\
            3. Organization-level restrictions on personal access tokens\n\
            4. Repository-level access restrictions\n\n\
                         Repository: TravelSizedLions/kernelle\n\
             Comment ID: {}\n\n\
             Original error: {:?}",
             reaction.emoji(), comment_id, comment_id, e
          )
        } else {
          anyhow!("Failed to add reaction {} to comment {}: {:?}", reaction.emoji(), comment_id, e)
        }
      })?;
    
    Ok(true)
  }

  async fn remove_reaction(
    &self,
    owner: &str,
    repo: &str,
    comment_id: &str,
    reaction: ReactionType,
  ) -> Result<bool> {
    let comment_id: u64 = comment_id.parse()
      .map_err(|_| anyhow!("Invalid comment ID: {}", comment_id))?;
    
    // GitHub API requires the reaction ID to delete, which we'd need to fetch first
    // For now, we'll implement a simpler approach by getting all reactions and finding ours
    let reactions = self.client
      .issues(owner, repo)
      .list_comment_reactions(comment_id)
      .send()
      .await?;
    
    // Convert to octocrab's reaction type for comparison
    let octocrab_reaction = match reaction.github_name() {
      "eyes" => octocrab::models::reactions::ReactionContent::Eyes,
      "heavy_check_mark" => octocrab::models::reactions::ReactionContent::Hooray,
      "question" => octocrab::models::reactions::ReactionContent::Confused,
      "memo" => octocrab::models::reactions::ReactionContent::Rocket,
      _ => return Ok(false),
    };

    // Find our reaction (assuming we're the authenticated user)
    for reaction_item in reactions {
      if reaction_item.content == octocrab_reaction {
        // Try to delete this reaction
        let delete_result = self.client
          .issues(owner, repo)
          .delete_comment_reaction(comment_id, reaction_item.id)
          .await;
        
        match delete_result {
          Ok(_) => return Ok(true),
          Err(e) => bentley::warn(&format!("Failed to remove reaction: {}", e)),
        }
      }
    }
    
    Ok(false)
  }

  async fn get_reactions(&self, owner: &str, repo: &str, comment_id: &str) -> Result<Vec<ReactionType>> {
    let comment_id: u64 = comment_id.parse()
      .map_err(|_| anyhow!("Invalid comment ID: {}", comment_id))?;
    
    let reactions = self.client
      .issues(owner, repo)
      .list_comment_reactions(comment_id)
      .send()
      .await?;
    
    let mut result = Vec::new();
    for reaction in reactions {
      match reaction.content {
        octocrab::models::reactions::ReactionContent::Eyes => result.push(ReactionType::Eyes),
        octocrab::models::reactions::ReactionContent::Hooray => result.push(ReactionType::CheckMark),
        octocrab::models::reactions::ReactionContent::Confused => result.push(ReactionType::Question),
        octocrab::models::reactions::ReactionContent::Rocket => result.push(ReactionType::Memo),
        _ => {} // Ignore other reactions
      }
    }
    
    Ok(result)
  }
}
