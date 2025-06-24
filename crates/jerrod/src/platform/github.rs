use crate::auth::get_github_token;
use crate::platform::{
  Discussion, FileDiff, GitPlatform, MergeRequest, MergeRequestState, Note, Pipeline, ReactionType,
  Repository, User,
};
use anyhow::{anyhow, Result};
use octocrab::Octocrab;
use serde_json::json;

/// Options for GitHub platform creation
#[derive(Debug, Clone, Default)]
pub struct GitHubPlatformOptions {
  /// Custom host (empty string uses github.com)
  pub host: String,
}

pub struct GitHubPlatform {
  #[allow(dead_code)]
  client: Octocrab,
}

impl GitHubPlatform {
  /// Create a new GitHub platform instance with options
  pub async fn new(options: GitHubPlatformOptions) -> Result<Self> {
    let token = get_github_token().await?;

    let client = if options.host.is_empty() {
      // Use default GitHub (github.com)
      Octocrab::builder().personal_token(token).build()?
    } else {
      // Use custom host (e.g., GitHub Enterprise)
      let base_url = if options.host.starts_with("http") {
        format!("{}/api/v3", options.host.trim_end_matches('/'))
      } else {
        format!("https://{}/api/v3", options.host)
      };

      Octocrab::builder().personal_token(token).base_uri(&base_url)?.build()?
    };

    Ok(Self { client })
  }

  /// Create a GitHub platform client from an existing Octocrab instance
  #[allow(dead_code)]
  pub fn from_client(client: Octocrab) -> Self {
    Self { client }
  }

  async fn get_review_threads_resolution(
    &self,
    owner: &str,
    repo: &str,
    pr_number: u64,
  ) -> Result<std::collections::HashMap<String, bool>> {
    let cache_buster = chrono::Utc::now().timestamp_millis();

    let payload = json!({
      "query": r#"
        query($owner: String!, $repo: String!, $prNumber: Int!) {
          repository(owner: $owner, name: $repo) {
            pullRequest(number: $prNumber) {
              reviewThreads(last: 100) {
                nodes {
                  id
                  isResolved
                  comments(last: 50) {
                    nodes {
                      id
                    }
                  }
                }
              }
            }
          }
        }
      "#,
      "variables": {
        "owner": owner,
        "repo": repo,
        "prNumber": pr_number
      }
    });

    bentley::info(&format!("Cache-busting GraphQL query with timestamp: {}", cache_buster));

    let response: serde_json::Value =
      self.client.graphql(&payload).await.map_err(|e| anyhow!("GraphQL query failed: {:?}", e))?;

    // Build a map from comment ID to thread resolution status
    let mut comment_resolution_map = std::collections::HashMap::new();

    if let Some(threads) =
      response["data"]["repository"]["pullRequest"]["reviewThreads"]["nodes"].as_array()
    {
      for thread in threads {
        if let (Some(is_resolved), Some(comments)) =
          (thread["isResolved"].as_bool(), thread["comments"]["nodes"].as_array())
        {
          // Map each comment in this thread to the thread's resolution status
          for comment in comments {
            if let Some(comment_id) = comment["id"].as_str() {
              comment_resolution_map.insert(comment_id.to_string(), is_resolved);
            }
          }
        }
      }
    }

    Ok(comment_resolution_map)
  }

  /// Add a reply to a review comment within its conversation thread
  pub async fn add_review_comment_reply(
    &self,
    owner: &str,
    repo: &str,
    pr_number: u64,
    comment_id: &str,
    text: &str,
  ) -> Result<Note> {
    let comment_id: u64 =
      comment_id.parse().map_err(|_| anyhow!("Invalid comment ID: {}", comment_id))?;

    let url =
      format!("/repos/{}/{}/pulls/{}/comments/{}/replies", owner, repo, pr_number, comment_id);

    let request_body = serde_json::json!({
      "body": text
    });

    let response = self
      .client
      ._post(&url, Some(&request_body))
      .await
      .map_err(|e| anyhow!("Failed to post review comment reply: {:?}", e))?;

    if !response.status().is_success() {
      return Err(anyhow!("Failed to create review comment reply: HTTP {}", response.status()));
    }

    let now = chrono::Utc::now();
    Ok(Note {
      id: "reply_created".to_string(), // Placeholder ID
      author: User {
        id: "current_user".to_string(),
        username: "current_user".to_string(),
        display_name: "Current User".to_string(),
        avatar_url: None,
      },
      body: text.to_string(),
      created_at: now,
      updated_at: now,
    })
  }

  /// Resolve a discussion thread using GraphQL (requires PR number for thread mapping)
  pub async fn resolve_discussion_with_pr(
    &self,
    owner: &str,
    repo: &str,
    pr_number: u64,
    comment_id: &str,
  ) -> Result<bool> {
    bentley::info(&format!(
      "Attempting to resolve comment {} in PR #{} via GraphQL",
      comment_id, pr_number
    ));

    // First, get all review threads and build the comment-to-thread mapping
    let query = r#"
      query($owner: String!, $repo: String!, $prNumber: Int!) {
        repository(owner: $owner, name: $repo) {
          pullRequest(number: $prNumber) {
            reviewThreads(last: 100) {
              nodes {
                id
                isResolved
                comments(last: 50) {
                  nodes {
                    id
                    databaseId
                  }
                }
              }
            }
          }
        }
      }
    "#;

    bentley::info("Fetching review threads to find comment...");
    let query_payload = serde_json::json!({
      "query": query,
      "variables": {
        "owner": owner,
        "repo": repo,
        "prNumber": pr_number
      }
    });

    let response: serde_json::Value = self
      .client
      .graphql(&query_payload)
      .await
      .map_err(|e| anyhow!("Failed to fetch review threads: {:?}", e))?;

    // Parse the response to find which thread contains our comment
    let threads = response["data"]["repository"]["pullRequest"]["reviewThreads"]["nodes"]
      .as_array()
      .ok_or_else(|| anyhow!("Invalid response format"))?;

    let mut target_thread_id: Option<String> = None;
    let comment_id_num: u64 =
      comment_id.parse().map_err(|_| anyhow!("Invalid comment ID: {}", comment_id))?;

    for thread in threads {
      let thread_id = thread["id"].as_str().unwrap_or("");
      let comments_array = thread["comments"]["nodes"].as_array().cloned().unwrap_or_default();

      for comment in &comments_array {
        if let Some(database_id) = comment["databaseId"].as_u64() {
          if database_id == comment_id_num {
            target_thread_id = Some(thread_id.to_string());
            bentley::info(&format!("Found comment {} in thread {}", comment_id, thread_id));
            break;
          }
        }
      }

      if target_thread_id.is_some() {
        break;
      }
    }

    let thread_id = target_thread_id
      .ok_or_else(|| anyhow!("Comment {} not found in any review thread", comment_id))?;

    // Now resolve the thread using GraphQL mutation
    let mutation = r#"
      mutation($threadId: ID!) {
        resolveReviewThread(input: {threadId: $threadId}) {
          thread {
            id
            isResolved
          }
        }
      }
    "#;

    bentley::info(&format!("Resolving thread {} via GraphQL mutation...", thread_id));

    let mutation_payload = serde_json::json!({
      "query": mutation,
      "variables": {
        "threadId": thread_id
      }
    });

    let mutation_response: serde_json::Value = self
      .client
      .graphql(&mutation_payload)
      .await
      .map_err(|e| anyhow!("Failed to resolve thread: {:?}", e))?;

    // Check if the mutation was successful
    if let Some(errors) = mutation_response["errors"].as_array() {
      if !errors.is_empty() {
        let error_msg = errors[0]["message"].as_str().unwrap_or("Unknown GraphQL error");
        bentley::warn(&format!("GraphQL mutation failed: {}", error_msg));
        return Ok(false);
      }
    }

    if let Some(thread_data) =
      mutation_response["data"]["resolveReviewThread"]["thread"].as_object()
    {
      let is_resolved = thread_data["isResolved"].as_bool().unwrap_or(false);
      if is_resolved {
        bentley::success(&format!("Successfully resolved thread {} via GraphQL", thread_id));
        return Ok(true);
      }
    }

    bentley::warn("GraphQL mutation completed but thread may not be resolved");
    Ok(false)
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
      url: repo_data
        .html_url
        .map(|u| u.to_string())
        .unwrap_or_else(|| format!("https://github.com/{}/{}", owner, repo)),
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
      url: pr
        .html_url
        .map(|u| u.to_string())
        .unwrap_or_else(|| format!("https://github.com/{}/{}/pull/{}", owner, repo, number)),
      created_at: pr.created_at.unwrap_or_else(chrono::Utc::now),
      updated_at: pr.updated_at.unwrap_or_else(chrono::Utc::now),
    })
  }

  async fn get_discussions(&self, owner: &str, repo: &str, number: u64) -> Result<Vec<Discussion>> {
    let mut discussions = Vec::new();

    // Cache-busting: Add timestamp to force fresh data
    let cache_buster = chrono::Utc::now().timestamp_millis();
    bentley::info(&format!("Cache-busting API calls with timestamp: {}", cache_buster));

    // Fetch regular issue comments (top-level comments on the PR)
    // Add per_page and cache-busting parameters to force fresh data
    let issue_comments = self
      .client
      .issues(owner, repo)
      .list_comments(number)
      .per_page(100) // Force pagination to bypass cache
      .send()
      .await?;

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

    // Get all review thread resolution statuses for this PR
    let thread_resolution_map =
      self.get_review_threads_resolution(owner, repo, number).await.unwrap_or_default();

    // Get review comments (inline comments on the diff)
    // Force fresh data by using pagination parameters
    let review_comments = self
      .client
      .pulls(owner, repo)
      .list_comments(Some(number))
      .per_page(100) // Force pagination to bypass cache
      .send()
      .await?;

    for comment in review_comments {
      if let Some(user) = comment.user {
        // For review comments (inline diff comments), check if they're resolved using GraphQL
        // We use GitHub's conversation resolution status instead of emoji reactions
        let is_resolved = thread_resolution_map.get(&comment.node_id).copied().unwrap_or(false);

        if is_resolved {
          bentley::info(&format!("Skipping resolved review comment {}", comment.id));
          continue;
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
          resolved: false, // We set this to false since we've already filtered resolved ones
          resolvable: true, // Review comments can be part of conversations
          file_path: Some(comment.path),
          line_number: comment.line.map(|line| line as u32),
          notes: vec![note],
        });
      }
    }

    bentley::info(&format!("Total discussions found after cache-busting: {}", discussions.len()));
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
    let pr_number: u64 =
      discussion_id.parse().map_err(|_| anyhow!("Invalid PR number: {}", discussion_id))?;

    // Create the comment using GitHub Issues API
    let comment = self
      .client
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

  async fn resolve_discussion(&self, owner: &str, repo: &str, discussion_id: &str) -> Result<bool> {
    // Load the current session to get the PR number
    match crate::session::load_current_session() {
      Ok(session) => {
        // Use the proper GraphQL resolution with PR number from session
        match self
          .resolve_discussion_with_pr(owner, repo, session.merge_request.number, discussion_id)
          .await
        {
          Ok(true) => Ok(true),
          Ok(false) => {
            bentley::warn(
              "GraphQL thread resolution failed - this may be expected for some comment types",
            );
            Ok(false)
          }
          Err(e) => {
            bentley::warn(&format!("GraphQL thread resolution error: {}", e));
            Ok(false)
          }
        }
      }
      Err(_) => {
        bentley::warn("No active session found - cannot resolve thread without PR context");
        Ok(false)
      }
    }
  }

  async fn add_reaction(
    &self,
    owner: &str,
    repo: &str,
    comment_id: &str,
    reaction: ReactionType,
  ) -> Result<bool> {
    let comment_id: u64 =
      comment_id.parse().map_err(|_| anyhow!("Invalid comment ID: {}", comment_id))?;

    // First try as a review comment, then fall back to issue comment
    // GitHub API endpoints:
    // - Review comments: POST /repos/{owner}/{repo}/pulls/comments/{comment_id}/reactions
    // - Issue comments: POST /repos/{owner}/{repo}/issues/comments/{comment_id}/reactions

    let review_comment_url =
      format!("/repos/{}/{}/pulls/comments/{}/reactions", owner, repo, comment_id);
    let issue_comment_url =
      format!("/repos/{}/{}/issues/comments/{}/reactions", owner, repo, comment_id);

    let reaction_body = serde_json::json!({
      "content": reaction.github_name()
    });

    // Try review comment first
    match self.client._post(&review_comment_url, Some(&reaction_body)).await {
      Ok(response) => {
        if response.status().is_success() {
          bentley::info(&format!(
            "Added {} reaction to review comment {}",
            reaction.emoji(),
            comment_id
          ));
          return Ok(true);
        }
        // Fall through to try issue comment (no debug message needed)
      }
      Err(_) => {
        // Fall through to try issue comment (no debug message needed)
      }
    }

    // Try issue comment
    match self.client._post(&issue_comment_url, Some(&reaction_body)).await {
      Ok(response) => {
        if response.status().is_success() {
          Ok(true)
        } else {
          Err(anyhow!(
            "Failed to add reaction {} to comment {}: HTTP {}",
            reaction.emoji(),
            comment_id,
            response.status()
          ))
        }
      }
      Err(e) => {
        // Check if this is a permission error and provide helpful guidance
        let error_msg = format!("{:?}", e);
        if error_msg.contains("403") || error_msg.contains("Resource not accessible") {
          Err(anyhow!(
            "Failed to add reaction {} to comment {}: Permission denied.\n\n\
            Possible causes:\n\
            1. Token missing 'public_repo' scope (or 'repo' for private repos)\n\
            2. Token type mismatch (fine-grained vs classic)\n\
            3. Organization-level restrictions on personal access tokens\n\
            4. Repository-level access restrictions\n\n\
                         Repository: {}/{}\n\
             Comment ID: {}\n\n\
             Original error: {:?}",
            reaction.emoji(),
            comment_id,
            owner,
            repo,
            comment_id,
            e
          ))
        } else {
          Err(anyhow!(
            "Failed to add reaction {} to comment {}: {:?}",
            reaction.emoji(),
            comment_id,
            e
          ))
        }
      }
    }
  }

  async fn remove_reaction(
    &self,
    owner: &str,
    repo: &str,
    comment_id: &str,
    reaction: ReactionType,
  ) -> Result<bool> {
    let comment_id: u64 =
      comment_id.parse().map_err(|_| anyhow!("Invalid comment ID: {}", comment_id))?;

    // GitHub API requires the reaction ID to delete, which we'd need to fetch first
    // For now, we'll implement a simpler approach by getting all reactions and finding ours
    let reactions =
      self.client.issues(owner, repo).list_comment_reactions(comment_id).send().await?;

    // Convert to octocrab's reaction type for comparison
    let octocrab_reaction = match reaction.github_name() {
      "+1" => octocrab::models::reactions::ReactionContent::PlusOne,
      "-1" => octocrab::models::reactions::ReactionContent::MinusOne,
      "laugh" => octocrab::models::reactions::ReactionContent::Laugh,
      "hooray" => octocrab::models::reactions::ReactionContent::Hooray,
      "confused" => octocrab::models::reactions::ReactionContent::Confused,
      "heart" => octocrab::models::reactions::ReactionContent::Heart,
      "rocket" => octocrab::models::reactions::ReactionContent::Rocket,
      "eyes" => octocrab::models::reactions::ReactionContent::Eyes,
      _ => return Ok(false),
    };

    // Find our reaction (assuming we're the authenticated user)
    for reaction_item in reactions {
      if reaction_item.content == octocrab_reaction {
        // Try to delete this reaction
        let delete_result = self
          .client
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

  async fn get_reactions(
    &self,
    owner: &str,
    repo: &str,
    comment_id: &str,
  ) -> Result<Vec<ReactionType>> {
    let comment_id: u64 =
      comment_id.parse().map_err(|_| anyhow!("Invalid comment ID: {}", comment_id))?;

    let reactions =
      self.client.issues(owner, repo).list_comment_reactions(comment_id).send().await?;

    let mut result = Vec::new();
    for reaction in reactions {
      match reaction.content {
        octocrab::models::reactions::ReactionContent::PlusOne => {
          result.push(ReactionType::ThumbsUp)
        }
        octocrab::models::reactions::ReactionContent::MinusOne => {
          result.push(ReactionType::ThumbsDown)
        }
        octocrab::models::reactions::ReactionContent::Laugh => result.push(ReactionType::Laugh),
        octocrab::models::reactions::ReactionContent::Hooray => result.push(ReactionType::Hooray),
        octocrab::models::reactions::ReactionContent::Confused => {
          result.push(ReactionType::Confused)
        }
        octocrab::models::reactions::ReactionContent::Heart => result.push(ReactionType::Heart),
        octocrab::models::reactions::ReactionContent::Rocket => result.push(ReactionType::Rocket),
        octocrab::models::reactions::ReactionContent::Eyes => result.push(ReactionType::Eyes),
      }
    }

    Ok(result)
  }

  async fn add_review_comment_reply(
    &self,
    owner: &str,
    repo: &str,
    pr_number: u64,
    comment_id: &str,
    text: &str,
  ) -> Result<Note> {
    // Delegate to the existing implementation
    self.add_review_comment_reply(owner, repo, pr_number, comment_id, text).await
  }

  fn format_comment_url(&self, mr_url: &str, comment_id: &str) -> String {
    format!("{}#issuecomment-{}", mr_url, comment_id)
  }

  fn format_merge_request_url(&self, owner: &str, repo: &str, number: u64) -> String {
    format!("https://github.com/{}/{}/pull/{}", owner, repo, number)
  }
}
