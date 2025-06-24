use crate::auth::get_gitlab_token;
use crate::platform::{
  Discussion, FileDiff, GitPlatform, MergeRequest, MergeRequestState, Note, Pipeline,
  PipelineStatus, ReactionType, Repository, User,
};
use anyhow::{anyhow, Result};
use reqwest::{
  header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE},
  Client,
};
use serde_json::{json, Value};

pub struct GitLabPlatform {
  client: Client,
  host: String,
  token: String,
}

impl GitLabPlatform {
  pub async fn new() -> Result<Self> {
    Self::new_with_host("https://gitlab.com").await
  }

  pub async fn new_with_host(host: &str) -> Result<Self> {
    let token = get_gitlab_token().await?;

    // Ensure we have a proper URL format
    let base_url =
      if host.starts_with("http") { host.to_string() } else { format!("https://{}", host) };

    bentley::info(&format!("Creating GitLab GraphQL client for host: {}", base_url));

    let client = Client::new();

    Ok(Self { client, host: base_url, token })
  }

  /// Make a GraphQL request to GitLab
  async fn graphql_request(&self, query: &str, variables: Value) -> Result<Value> {
    let url = format!("{}/api/graphql", self.host);
    let payload = json!({
      "query": query,
      "variables": variables
    });

    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", self.token))?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let response = self
      .client
      .post(&url)
      .headers(headers)
      .json(&payload)
      .send()
      .await
      .map_err(|e| anyhow!("GraphQL request failed: {:?}", e))?;

    if !response.status().is_success() {
      return Err(anyhow!("GraphQL request failed with status: {}", response.status()));
    }

    let json_response: Value =
      response.json().await.map_err(|e| anyhow!("Failed to parse GraphQL response: {:?}", e))?;

    // Check for GraphQL errors
    if let Some(errors) = json_response["errors"].as_array() {
      if !errors.is_empty() {
        let error_msg = errors[0]["message"].as_str().unwrap_or("Unknown GraphQL error");
        return Err(anyhow!("GraphQL error: {}", error_msg));
      }
    }

    Ok(json_response)
  }

  /// Get the project full path from owner and repo
  fn get_project_path(&self, owner: &str, repo: &str) -> String {
    format!("{}/{}", owner, repo)
  }

  /// Convert GitLab merge request state to our common type
  fn convert_mr_state(&self, state: &str) -> MergeRequestState {
    match state {
      "opened" => MergeRequestState::Open,
      "closed" => MergeRequestState::Closed,
      "merged" => MergeRequestState::Merged,
      "draft" => MergeRequestState::Draft,
      _ => MergeRequestState::Open, // Default fallback
    }
  }

  /// Convert GitLab pipeline status to our common type
  fn convert_pipeline_status(&self, status: &str) -> PipelineStatus {
    match status {
      "running" => PipelineStatus::Running,
      "success" => PipelineStatus::Success,
      "failed" => PipelineStatus::Failed,
      "canceled" => PipelineStatus::Canceled,
      "skipped" => PipelineStatus::Skipped,
      "pending" => PipelineStatus::Pending,
      _ => PipelineStatus::Pending, // Default fallback
    }
  }
}

#[async_trait::async_trait]
impl GitPlatform for GitLabPlatform {
  async fn get_repository(&self, owner: &str, repo: &str) -> Result<Repository> {
    let project_path = self.get_project_path(owner, repo);

    let query = r#"
      query($fullPath: ID!) {
        project(fullPath: $fullPath) {
          id
          name
          webUrl
        }
      }
    "#;

    let variables = json!({
      "fullPath": project_path
    });

    let response = self.graphql_request(query, variables).await?;

    let project =
      response["data"]["project"].as_object().ok_or_else(|| anyhow!("Project not found"))?;

    Ok(Repository {
      owner: owner.to_string(),
      name: repo.to_string(),
      full_name: project_path,
      url: project["webUrl"].as_str().unwrap_or_default().to_string(),
    })
  }

  async fn get_merge_request(&self, owner: &str, repo: &str, number: u64) -> Result<MergeRequest> {
    let project_path = self.get_project_path(owner, repo);

    let query = r#"
      query($fullPath: ID!, $iid: String!) {
        project(fullPath: $fullPath) {
          mergeRequest(iid: $iid) {
            id
            iid
            title
            description
            state
            webUrl
            createdAt
            updatedAt
            sourceBranch
            targetBranch
            author {
              id
              username
              name
              avatarUrl
            }
            assignees {
              nodes {
                id
                username
                name
                avatarUrl
              }
            }
          }
        }
      }
    "#;

    let variables = json!({
      "fullPath": project_path,
      "iid": number.to_string()
    });

    let response = self.graphql_request(query, variables).await?;

    let mr = response["data"]["project"]["mergeRequest"]
      .as_object()
      .ok_or_else(|| anyhow!("Merge request not found"))?;

    let state = self.convert_mr_state(mr["state"].as_str().unwrap_or("opened"));

    let author = &mr["author"];
    let assignee =
      mr["assignees"]["nodes"].as_array().and_then(|arr| arr.first()).map(|assignee| User {
        id: assignee["id"].as_str().unwrap_or_default().to_string(),
        username: assignee["username"].as_str().unwrap_or_default().to_string(),
        display_name: assignee["name"]
          .as_str()
          .unwrap_or(assignee["username"].as_str().unwrap_or("Unknown"))
          .to_string(),
        avatar_url: assignee["avatarUrl"].as_str().map(|s| s.to_string()),
      });

    let created_at = chrono::DateTime::parse_from_rfc3339(
      mr["createdAt"].as_str().unwrap_or("1970-01-01T00:00:00Z"),
    )?
    .with_timezone(&chrono::Utc);

    let updated_at = chrono::DateTime::parse_from_rfc3339(
      mr["updatedAt"].as_str().unwrap_or("1970-01-01T00:00:00Z"),
    )?
    .with_timezone(&chrono::Utc);

    Ok(MergeRequest {
      id: mr["id"].as_str().unwrap_or_default().to_string(),
      number,
      title: mr["title"].as_str().unwrap_or_default().to_string(),
      description: mr["description"].as_str().map(|s| s.to_string()),
      state,
      author: User {
        id: author["id"].as_str().unwrap_or_default().to_string(),
        username: author["username"].as_str().unwrap_or_default().to_string(),
        display_name: author["name"]
          .as_str()
          .unwrap_or(author["username"].as_str().unwrap_or("Unknown"))
          .to_string(),
        avatar_url: author["avatarUrl"].as_str().map(|s| s.to_string()),
      },
      assignee,
      source_branch: mr["sourceBranch"].as_str().unwrap_or_default().to_string(),
      target_branch: mr["targetBranch"].as_str().unwrap_or_default().to_string(),
      url: mr["webUrl"].as_str().unwrap_or_default().to_string(),
      created_at,
      updated_at,
    })
  }

  async fn get_discussions(&self, owner: &str, repo: &str, number: u64) -> Result<Vec<Discussion>> {
    let project_path = self.get_project_path(owner, repo);

    let query = r#"
      query($fullPath: ID!, $iid: String!) {
        project(fullPath: $fullPath) {
          mergeRequest(iid: $iid) {
            discussions {
              nodes {
                id
                resolved
                resolvable
                notes {
                  nodes {
                    id
                    body
                    author {
                      id
                      username
                      name
                      avatarUrl
                    }
                    createdAt
                    updatedAt
                    system
                    position {
                      newPath
                      newLine
                    }
                  }
                }
              }
            }
          }
        }
      }
    "#;

    let variables = json!({
      "fullPath": project_path,
      "iid": number.to_string()
    });

    let response = self.graphql_request(query, variables).await?;

    let discussions = response["data"]["project"]["mergeRequest"]["discussions"]["nodes"]
      .as_array()
      .ok_or_else(|| anyhow!("Failed to get discussions"))?;

    let mut result = Vec::new();
    for discussion in discussions {
      let empty_notes = vec![];
      let notes_array = discussion["notes"]["nodes"].as_array().unwrap_or(&empty_notes);

      // Skip system notes and resolved discussions
      let non_system_notes: Vec<_> =
        notes_array.iter().filter(|note| !note["system"].as_bool().unwrap_or(false)).collect();

      if non_system_notes.is_empty() {
        continue;
      }

      let is_resolved = discussion["resolved"].as_bool().unwrap_or(false);
      if is_resolved {
        bentley::info(&format!(
          "Skipping resolved discussion {}",
          discussion["id"].as_str().unwrap_or("unknown")
        ));
        continue;
      }

      // Extract position info from first non-system note
      let (file_path, line_number) = if let Some(first_note) = non_system_notes.first() {
        if let Some(position) = first_note["position"].as_object() {
          (
            position["newPath"].as_str().map(|s| s.to_string()),
            position["newLine"].as_i64().map(|l| l as u32),
          )
        } else {
          (None, None)
        }
      } else {
        (None, None)
      };

      let notes: Vec<Note> = non_system_notes
        .into_iter()
        .map(|note| {
          let author = &note["author"];
          let created_at = chrono::DateTime::parse_from_rfc3339(
            note["createdAt"].as_str().unwrap_or("1970-01-01T00:00:00Z"),
          )
          .unwrap_or_else(|_| chrono::Utc::now().into())
          .with_timezone(&chrono::Utc);

          let updated_at = chrono::DateTime::parse_from_rfc3339(
            note["updatedAt"].as_str().unwrap_or("1970-01-01T00:00:00Z"),
          )
          .unwrap_or_else(|_| chrono::Utc::now().into())
          .with_timezone(&chrono::Utc);

          Note {
            id: note["id"].as_str().unwrap_or_default().to_string(),
            author: User {
              id: author["id"].as_str().unwrap_or_default().to_string(),
              username: author["username"].as_str().unwrap_or_default().to_string(),
              display_name: author["name"]
                .as_str()
                .unwrap_or(author["username"].as_str().unwrap_or("Unknown"))
                .to_string(),
              avatar_url: author["avatarUrl"].as_str().map(|s| s.to_string()),
            },
            body: note["body"].as_str().unwrap_or_default().to_string(),
            created_at,
            updated_at,
          }
        })
        .collect();

      if !notes.is_empty() {
        result.push(Discussion {
          id: discussion["id"].as_str().unwrap_or_default().to_string(),
          resolved: false, // We already filtered resolved ones
          resolvable: discussion["resolvable"].as_bool().unwrap_or(false),
          file_path,
          line_number,
          notes,
        });
      }
    }

    bentley::info(&format!("Total discussions found: {}", result.len()));
    Ok(result)
  }

  async fn get_diffs(&self, owner: &str, repo: &str, number: u64) -> Result<Vec<FileDiff>> {
    let project_path = self.get_project_path(owner, repo);

    let query = r#"
      query($fullPath: ID!, $iid: String!) {
        project(fullPath: $fullPath) {
          mergeRequest(iid: $iid) {
            diffRefs {
              headSha
              baseSha
            }
          }
        }
      }
    "#;

    let variables = json!({
      "fullPath": project_path,
      "iid": number.to_string()
    });

    let _response = self.graphql_request(query, variables).await?;

    // Note: GitLab GraphQL doesn't expose individual file diffs directly
    // This would require REST API fallback or external git diff
    bentley::info("GitLab file diffs not yet implemented via GraphQL");
    Ok(vec![])
  }

  async fn get_pipelines(&self, owner: &str, repo: &str, sha: &str) -> Result<Vec<Pipeline>> {
    let project_path = self.get_project_path(owner, repo);

    let query = r#"
      query($fullPath: ID!, $sha: String!) {
        project(fullPath: $fullPath) {
          pipelines(sha: $sha) {
            nodes {
              id
              status
              ref
              sha
              webUrl
              createdAt
              updatedAt
            }
          }
        }
      }
    "#;

    let variables = json!({
      "fullPath": project_path,
      "sha": sha
    });

    let response = self.graphql_request(query, variables).await?;

    let empty_pipelines = vec![];
    let pipelines =
      response["data"]["project"]["pipelines"]["nodes"].as_array().unwrap_or(&empty_pipelines);

    let mut result = Vec::new();
    for pipeline in pipelines {
      let created_at = chrono::DateTime::parse_from_rfc3339(
        pipeline["createdAt"].as_str().unwrap_or("1970-01-01T00:00:00Z"),
      )
      .unwrap_or_else(|_| chrono::Utc::now().into())
      .with_timezone(&chrono::Utc);

      let updated_at = chrono::DateTime::parse_from_rfc3339(
        pipeline["updatedAt"].as_str().unwrap_or("1970-01-01T00:00:00Z"),
      )
      .unwrap_or_else(|_| chrono::Utc::now().into())
      .with_timezone(&chrono::Utc);

      result.push(Pipeline {
        id: pipeline["id"].as_str().unwrap_or_default().to_string(),
        status: self.convert_pipeline_status(pipeline["status"].as_str().unwrap_or("pending")),
        ref_name: pipeline["ref"].as_str().unwrap_or_default().to_string(),
        sha: pipeline["sha"].as_str().unwrap_or_default().to_string(),
        url: pipeline["webUrl"].as_str().map(|s| s.to_string()),
        created_at,
        updated_at,
      });
    }

    Ok(result)
  }

  async fn add_comment(
    &self,
    owner: &str,
    repo: &str,
    discussion_id: &str,
    text: &str,
  ) -> Result<Note> {
    let _project_path = self.get_project_path(owner, repo);

    let mutation = r#"
      mutation($noteableId: NoteableID!, $body: String!) {
        createNote(input: {noteableId: $noteableId, body: $body}) {
          note {
            id
            body
            author {
              id
              username
              name
              avatarUrl
            }
            createdAt
            updatedAt
          }
        }
      }
    "#;

    let variables = json!({
      "noteableId": discussion_id,
      "body": text
    });

    let response = self.graphql_request(mutation, variables).await?;

    let note = response["data"]["createNote"]["note"]
      .as_object()
      .ok_or_else(|| anyhow!("Failed to create note"))?;

    let author = &note["author"];
    let created_at = chrono::DateTime::parse_from_rfc3339(
      note["createdAt"].as_str().unwrap_or("1970-01-01T00:00:00Z"),
    )
    .unwrap_or_else(|_| chrono::Utc::now().into())
    .with_timezone(&chrono::Utc);

    let updated_at = chrono::DateTime::parse_from_rfc3339(
      note["updatedAt"].as_str().unwrap_or("1970-01-01T00:00:00Z"),
    )
    .unwrap_or_else(|_| chrono::Utc::now().into())
    .with_timezone(&chrono::Utc);

    Ok(Note {
      id: note["id"].as_str().unwrap_or_default().to_string(),
      author: User {
        id: author["id"].as_str().unwrap_or_default().to_string(),
        username: author["username"].as_str().unwrap_or_default().to_string(),
        display_name: author["name"]
          .as_str()
          .unwrap_or(author["username"].as_str().unwrap_or("Unknown"))
          .to_string(),
        avatar_url: author["avatarUrl"].as_str().map(|s| s.to_string()),
      },
      body: note["body"].as_str().unwrap_or_default().to_string(),
      created_at,
      updated_at,
    })
  }

  async fn resolve_discussion(
    &self,
    _owner: &str,
    _repo: &str,
    discussion_id: &str,
  ) -> Result<bool> {
    bentley::info(&format!("Attempting to resolve GitLab discussion: {}", discussion_id));

    let mutation = r#"
      mutation($discussionId: DiscussionID!, $resolved: Boolean!) {
        discussionToggleResolve(input: {id: $discussionId, resolve: $resolved}) {
          discussion {
            id
            resolved
          }
        }
      }
    "#;

    let variables = json!({
      "discussionId": discussion_id,
      "resolved": true
    });

    let response = self.graphql_request(mutation, variables).await?;

    if let Some(discussion) = response["data"]["discussionToggleResolve"]["discussion"].as_object()
    {
      let is_resolved = discussion["resolved"].as_bool().unwrap_or(false);
      if is_resolved {
        bentley::success(&format!("Successfully resolved GitLab discussion {}", discussion_id));
        return Ok(true);
      }
    }

    bentley::warn("Discussion resolution may not have completed successfully");
    Ok(false)
  }

  async fn add_reaction(
    &self,
    _owner: &str,
    _repo: &str,
    comment_id: &str,
    reaction: ReactionType,
  ) -> Result<bool> {
    bentley::info(&format!("Adding {} reaction to comment {}", reaction.emoji(), comment_id));

    let mutation = r#"
      mutation($awardableId: AwardableID!, $name: String!) {
        awardEmojiAdd(input: {awardableId: $awardableId, name: $name}) {
          awardEmoji {
            name
          }
        }
      }
    "#;

    let emoji_name = reaction.gitlab_name();
    let variables = json!({
      "awardableId": comment_id,
      "name": emoji_name
    });

    let response = self.graphql_request(mutation, variables).await?;

    if response["data"]["awardEmojiAdd"]["awardEmoji"].is_object() {
      bentley::success(&format!("Added {} reaction successfully", reaction.emoji()));
      Ok(true)
    } else {
      bentley::warn("Failed to add reaction");
      Ok(false)
    }
  }

  async fn remove_reaction(
    &self,
    _owner: &str,
    _repo: &str,
    comment_id: &str,
    reaction: ReactionType,
  ) -> Result<bool> {
    bentley::info(&format!("Removing {} reaction from comment {}", reaction.emoji(), comment_id));

    let mutation = r#"
      mutation($awardableId: AwardableID!, $name: String!) {
        awardEmojiRemove(input: {awardableId: $awardableId, name: $name}) {
          awardEmoji {
            name
          }
        }
      }
    "#;

    let emoji_name = reaction.gitlab_name();
    let variables = json!({
      "awardableId": comment_id,
      "name": emoji_name
    });

    let response = self.graphql_request(mutation, variables).await?;

    if response["data"]["awardEmojiRemove"]["awardEmoji"].is_object() {
      bentley::success(&format!("Removed {} reaction successfully", reaction.emoji()));
      Ok(true)
    } else {
      bentley::warn("Failed to remove reaction (may not exist)");
      Ok(false)
    }
  }

  async fn get_reactions(
    &self,
    _owner: &str,
    _repo: &str,
    comment_id: &str,
  ) -> Result<Vec<ReactionType>> {
    let query = r#"
      query($noteId: NoteID!) {
        note(id: $noteId) {
          awardEmoji {
            nodes {
              name
            }
          }
        }
      }
    "#;

    let variables = json!({
      "noteId": comment_id
    });

    let response = self.graphql_request(query, variables).await?;

    let empty_emojis = vec![];
    let emojis =
      response["data"]["note"]["awardEmoji"]["nodes"].as_array().unwrap_or(&empty_emojis);

    let mut reactions = Vec::new();
    for emoji in emojis {
      if let Some(name) = emoji["name"].as_str() {
        if let Some(reaction) = self.gitlab_name_to_reaction(name) {
          reactions.push(reaction);
        }
      }
    }

    Ok(reactions)
  }

  async fn add_review_comment_reply(
    &self,
    owner: &str,
    repo: &str,
    _pr_number: u64,
    comment_id: &str,
    text: &str,
  ) -> Result<Note> {
    // In GitLab, this is the same as add_comment - replying to a discussion
    self.add_comment(owner, repo, comment_id, text).await
  }

  fn format_comment_url(&self, mr_url: &str, comment_id: &str) -> String {
    // Parse comment_id to extract note ID
    let parts: Vec<&str> = comment_id.split('_').collect();
    if parts.len() >= 3 {
      format!("{}#note_{}", mr_url, parts[2])
    } else {
      mr_url.to_string()
    }
  }

  fn format_merge_request_url(&self, owner: &str, repo: &str, number: u64) -> String {
    let base_url = if self.host.starts_with("http") {
      self.host.clone()
    } else {
      format!("https://{}", self.host)
    };

    format!("{}/{}/{}/-/merge_requests/{}", base_url, owner, repo, number)
  }
}

impl GitLabPlatform {
  /// Convert GitLab emoji name to our ReactionType
  #[allow(dead_code)]
  fn gitlab_name_to_reaction(&self, name: &str) -> Option<ReactionType> {
    match name {
      "thumbsup" => Some(ReactionType::ThumbsUp),
      "thumbsdown" => Some(ReactionType::ThumbsDown),
      "smile" => Some(ReactionType::Laugh),
      "tada" => Some(ReactionType::Hooray),
      "confused" => Some(ReactionType::Confused),
      "heart" => Some(ReactionType::Heart),
      "rocket" => Some(ReactionType::Rocket),
      "eyes" => Some(ReactionType::Eyes),
      _ => None,
    }
  }

  /// Convert our ReactionType to GitLab emoji name
  #[allow(dead_code)]
  fn reaction_to_gitlab_name(&self, reaction: &ReactionType) -> &str {
    match reaction {
      ReactionType::ThumbsUp => "thumbsup",
      ReactionType::ThumbsDown => "thumbsdown",
      ReactionType::Laugh => "smile",
      ReactionType::Hooray => "tada",
      ReactionType::Confused => "confused",
      ReactionType::Heart => "heart",
      ReactionType::Rocket => "rocket",
      ReactionType::Eyes => "eyes",
    }
  }
}

// GitLab GraphQL implementation complete - no additional data structures needed
