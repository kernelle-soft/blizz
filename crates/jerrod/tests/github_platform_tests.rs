use anyhow::Result;
use jerrod::auth::{register_provider_factory, reset_provider_factory};
use jerrod::platform::github::GitHubPlatform;
use jerrod::platform::{GitPlatform, MergeRequestState, ReactionType};
use octocrab::Octocrab;
use sentinel::MockCredentialProvider;
use serde_json::json;
use serial_test::serial;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// Mock data helpers with complete GitHub API field coverage
mod github_mock_data {
  use serde_json::{json, Value};

  /// Creates a complete GitHub User object with all required fields
  pub fn default_user() -> Value {
    json!({
      "id": 123456,
      "login": "default-user",
      "name": "Default User",
      "avatar_url": "https://github.com/default-user.png",
      "gravatar_id": "",
      "node_id": "MDQ6VXNlcjEyMzQ1Ng==",
      "url": "https://api.github.com/users/default-user",
      "html_url": "https://github.com/default-user",
      "followers_url": "https://api.github.com/users/default-user/followers",
      "following_url": "https://api.github.com/users/default-user/following{/other_user}",
      "gists_url": "https://api.github.com/users/default-user/gists{/gist_id}",
      "starred_url": "https://api.github.com/users/default-user/starred{/owner}{/repo}",
      "subscriptions_url": "https://api.github.com/users/default-user/subscriptions",
      "organizations_url": "https://api.github.com/users/default-user/orgs",
      "repos_url": "https://api.github.com/users/default-user/repos",
      "events_url": "https://api.github.com/users/default-user/events{/privacy}",
      "received_events_url": "https://api.github.com/users/default-user/received_events",
      "type": "User",
      "site_admin": false
    })
  }

  /// Creates a complete GitHub Pull Request object with all required fields
  pub fn default_pull_request() -> Value {
    json!({
      "id": 123456789,
      "number": 123,
      "title": "Default Pull Request",
      "body": "Default pull request description",
      "state": "open",
      "locked": false,
      "merged": false,
      "merged_at": null,
      "merge_commit_sha": null,
      "draft": false,
      "html_url": "https://github.com/owner/repo/pull/123",
      "url": "https://api.github.com/repos/owner/repo/pulls/123",
      "node_id": "MDExOlB1bGxSZXF1ZXN0MTIzNDU2Nzg5",
      "diff_url": "https://github.com/owner/repo/pull/123.diff",
      "patch_url": "https://github.com/owner/repo/pull/123.patch",
      "issue_url": "https://api.github.com/repos/owner/repo/issues/123",
      "commits_url": "https://api.github.com/repos/owner/repo/pulls/123/commits",
      "review_comments_url": "https://api.github.com/repos/owner/repo/pulls/123/comments",
      "review_comment_url": "https://api.github.com/repos/owner/repo/pulls/comments{/number}",
      "comments_url": "https://api.github.com/repos/owner/repo/issues/123/comments",
      "statuses_url": "https://api.github.com/repos/owner/repo/statuses/abc123def456",
      "head": {
        "sha": "abc123def456",
        "ref": "feature-branch",
        "repo": {
          "id": 987654321,
          "name": "repo",
          "full_name": "owner/repo",
          "node_id": "MDEwOlJlcG9zaXRvcnk5ODc2NTQzMjE=",
          "html_url": "https://github.com/owner/repo",
          "url": "https://api.github.com/repos/owner/repo",
          "contents_url": "https://api.github.com/repos/owner/repo/contents/{+path}"
        }
      },
      "base": {
        "sha": "base123def456",
        "ref": "main",
        "repo": {
          "id": 987654321,
          "name": "repo",
          "full_name": "owner/repo",
          "node_id": "MDEwOlJlcG9zaXRvcnk5ODc2NTQzMjE=",
          "html_url": "https://github.com/owner/repo",
          "url": "https://api.github.com/repos/owner/repo",
          "contents_url": "https://api.github.com/repos/owner/repo/contents/{+path}"
        }
      },
      "user": default_user(),
      "assignee": null,
      "assignees": [],
      "requested_reviewers": [],
      "requested_teams": [],
      "labels": [],
      "milestone": null,
      "created_at": "2023-01-01T00:00:00Z",
      "updated_at": "2023-01-01T12:00:00Z",
      "closed_at": null,
      "author_association": "CONTRIBUTOR",
      "auto_merge": null,
      "active_lock_reason": null,
      "rebaseable": true,
      "squash_merge_commit_title": "COMMIT_OR_PR_TITLE",
      "squash_merge_commit_message": "COMMIT_MESSAGES",
      "merge_commit_title": "MERGE_MESSAGE",
      "merge_commit_message": "PR_TITLE"
    })
  }

  /// Creates a complete GitHub Reaction object with all required fields  
  pub fn default_reaction() -> Value {
    json!({
      "id": 1,
      "node_id": "MDg6UmVhY3Rpb24x",
      "content": "+1",
      "user": default_user(),
      "created_at": "2023-01-01T00:00:00Z"
    })
  }

  /// Creates a complete GitHub Repository object with all required fields
  pub fn default_repository() -> Value {
    json!({
      "id": 987654321,
      "name": "repo",
      "full_name": "owner/repo",
      "description": "Default repository",
      "html_url": "https://github.com/owner/repo",
      "url": "https://api.github.com/repos/owner/repo",
      "node_id": "MDEwOlJlcG9zaXRvcnk5ODc2NTQzMjE=",
      "contents_url": "https://api.github.com/repos/owner/repo/contents/{+path}",
      "default_branch": "main",
      "private": false,
      "fork": false,
      "archived": false,
      "disabled": false,
      "owner": default_user()
    })
  }

  /// Deep merges partial overrides into a base JSON value
  pub fn merge_json(base: Value, overrides: Value) -> Value {
    match (base, overrides) {
      (Value::Object(mut base_map), Value::Object(override_map)) => {
        for (key, value) in override_map {
          match base_map.get(&key) {
            Some(base_value) if base_value.is_object() && value.is_object() => {
              base_map.insert(key, merge_json(base_value.clone(), value));
            }
            _ => {
              base_map.insert(key, value);
            }
          }
        }
        Value::Object(base_map)
      }
      (_, override_value) => override_value,
    }
  }

  /// Creates a mock user with specific overrides
  pub fn mock_user(overrides: Value) -> Value {
    merge_json(default_user(), overrides)
  }

  /// Creates a mock pull request with specific overrides
  pub fn mock_pull_request(overrides: Value) -> Value {
    merge_json(default_pull_request(), overrides)
  }

  /// Creates a mock reaction with specific overrides
  pub fn mock_reaction(overrides: Value) -> Value {
    merge_json(default_reaction(), overrides)
  }

  /// Creates a mock repository with specific overrides
  pub fn mock_repository(overrides: Value) -> Value {
    merge_json(default_repository(), overrides)
  }

  /// Creates a complete GitHub Comment object with all required fields
  pub fn default_comment() -> Value {
    json!({
      "id": 123456789,
      "node_id": "MDEyOklzc3VlQ29tbWVudDEyMzQ1Njc4OQ==",
      "url": "https://api.github.com/repos/owner/repo/issues/comments/123456789",
      "html_url": "https://github.com/owner/repo/issues/123#issuecomment-123456789",
      "body": "Default comment body",
      "user": default_user(),
      "created_at": "2023-01-01T00:00:00Z",
      "updated_at": "2023-01-01T00:00:00Z",
      "issue_url": "https://api.github.com/repos/owner/repo/issues/123",
      "author_association": "CONTRIBUTOR"
    })
  }

  /// Creates a mock comment with specific overrides
  pub fn mock_comment(overrides: Value) -> Value {
    merge_json(default_comment(), overrides)
  }

  /// Creates a complete GitHub File diff object with all required fields
  pub fn default_file_diff() -> Value {
    json!({
      "sha": "abc123def456",
      "filename": "src/example.rs",
      "status": "modified",
      "additions": 10,
      "deletions": 5,
      "changes": 15,
      "blob_url": "https://github.com/owner/repo/blob/abc123def456/src/example.rs",
      "raw_url": "https://github.com/owner/repo/raw/abc123def456/src/example.rs",
      "contents_url": "https://api.github.com/repos/owner/repo/contents/src/example.rs?ref=abc123def456",
      "patch": "@@ -1,3 +1,4 @@\n fn main() {\n+    println!(\"Hello\");\n }",
      "previous_filename": null
    })
  }

  /// Creates a mock file diff with specific overrides
  pub fn mock_file_diff(overrides: Value) -> Value {
    merge_json(default_file_diff(), overrides)
  }
}

// Helper to create a GitHub platform with mocked HTTP client
async fn create_test_github_platform(mock_server: &MockServer) -> Result<GitHubPlatform> {
  // Set up mock credentials
  register_provider_factory(|| {
    Box::new(MockCredentialProvider::new().with_credential(
      "github",
      "token",
      "fake-github-token-123",
    ))
  });

  // Create octocrab client pointing to our mock server
  let client = Octocrab::builder()
    .personal_token("fake-github-token-123")
    .base_uri(mock_server.uri())?
    .build()?;

  Ok(GitHubPlatform::from_client(client))
}

#[tokio::test]
#[serial]
async fn test_get_repository_success() -> Result<()> {
  let mock_server = MockServer::start().await;

  Mock::given(method("GET"))
    .and(path("/repos/owner/repo"))
    .respond_with(ResponseTemplate::new(200).set_body_json(github_mock_data::mock_repository(
      json!({
        "name": "repo",
        "full_name": "owner/repo",
        "description": "Test repository",
        "owner": github_mock_data::mock_user(json!({
          "login": "owner"
        }))
      }),
    )))
    .mount(&mock_server)
    .await;

  let platform = create_test_github_platform(&mock_server).await?;
  let repo = platform.get_repository("owner", "repo").await?;

  assert_eq!(repo.owner, "owner");
  assert_eq!(repo.name, "repo");
  assert_eq!(repo.full_name, "owner/repo");
  assert_eq!(repo.url, "https://github.com/owner/repo");

  reset_provider_factory();
  Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_repository_not_found() -> Result<()> {
  let mock_server = MockServer::start().await;

  Mock::given(method("GET"))
    .and(path("/repos/owner/nonexistent"))
    .respond_with(ResponseTemplate::new(404))
    .mount(&mock_server)
    .await;

  let platform = create_test_github_platform(&mock_server).await?;
  let result = platform.get_repository("owner", "nonexistent").await;

  assert!(result.is_err());

  reset_provider_factory();
  Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_merge_request_success() -> Result<()> {
  let mock_server = MockServer::start().await;

  Mock::given(method("GET"))
    .and(path("/repos/owner/repo/pulls/123"))
    .respond_with(ResponseTemplate::new(200).set_body_json(github_mock_data::mock_pull_request(
      json!({
        "id": 789,
        "number": 123,
        "title": "Test PR",
        "body": "This is a test pull request",
        "user": github_mock_data::mock_user(json!({
          "id": 456,
          "login": "test-user",
          "name": "Test User"
        }))
      }),
    )))
    .mount(&mock_server)
    .await;

  let platform = create_test_github_platform(&mock_server).await?;
  let mr = platform.get_merge_request("owner", "repo", 123).await?;

  assert_eq!(mr.id, "789");
  assert_eq!(mr.number, 123);
  assert_eq!(mr.title, "Test PR");
  assert_eq!(mr.description, Some("This is a test pull request".to_string()));
  assert_eq!(mr.state, MergeRequestState::Open);
  assert_eq!(mr.url, "https://github.com/owner/repo/pull/123");
  assert_eq!(mr.author.username, "test-user");

  reset_provider_factory();
  Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_merge_request_merged() -> Result<()> {
  let mock_server = MockServer::start().await;

  Mock::given(method("GET"))
    .and(path("/repos/owner/repo/pulls/456"))
    .respond_with(ResponseTemplate::new(200).set_body_json(github_mock_data::mock_pull_request(
      json!({
        "id": 999,
        "number": 456,
        "title": "Merged PR",
        "body": "This PR was merged",
        "state": "closed",
        "merged": true,
        "merged_at": "2023-01-01T12:00:00Z",
        "head": {
          "sha": "def456ghi789",
          "ref": "feature-branch-2"
        },
        "base": {
          "sha": "base456ghi789",
          "ref": "main"
        },
        "user": github_mock_data::mock_user(json!({
          "id": 789,
          "login": "merger",
          "name": "The Merger"
        }))
      }),
    )))
    .mount(&mock_server)
    .await;

  let platform = create_test_github_platform(&mock_server).await?;
  let mr = platform.get_merge_request("owner", "repo", 456).await?;

  assert_eq!(mr.state, MergeRequestState::Merged);

  reset_provider_factory();
  Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_discussions_with_graphql() -> Result<()> {
  let mock_server = MockServer::start().await;

  // Mock issue comments endpoint
  Mock::given(method("GET"))
    .and(path("/repos/owner/repo/issues/123/comments"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!([
      github_mock_data::mock_comment(json!({
        "id": 111,
        "body": "This needs fixing",
        "user": github_mock_data::mock_user(json!({
          "login": "reviewer1"
        }))
      }))
    ])))
    .mount(&mock_server)
    .await;

  // Mock pull request comments endpoint
  Mock::given(method("GET"))
    .and(path("/repos/owner/repo/pulls/123/comments"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
    .mount(&mock_server)
    .await;

  // Mock reactions endpoint for each comment
  Mock::given(method("GET"))
    .and(path("/repos/owner/repo/issues/comments/111/reactions"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
    .mount(&mock_server)
    .await;

  // Mock the GraphQL endpoint for review threads
  Mock::given(method("POST"))
    .and(path("/graphql"))
    .and(header("authorization", "Bearer fake-github-token-123"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
        "data": {
            "repository": {
                "pullRequest": {
                    "reviewThreads": {
                        "nodes": []
                    }
                }
            }
        }
    })))
    .mount(&mock_server)
    .await;

  let platform = create_test_github_platform(&mock_server).await?;
  let discussions = platform.get_discussions("owner", "repo", 123).await?;

  assert_eq!(discussions.len(), 1);
  assert_eq!(discussions[0].id, "111");
  assert_eq!(discussions[0].notes.len(), 1);
  assert_eq!(discussions[0].notes[0].body, "This needs fixing");
  assert_eq!(discussions[0].notes[0].author.username, "reviewer1");
  assert!(!discussions[0].resolved);

  reset_provider_factory();
  Ok(())
}

#[tokio::test]
#[serial]
async fn test_add_reaction_thumbs_up() -> Result<()> {
  let mock_server = MockServer::start().await;

  Mock::given(method("POST"))
    .and(path("/repos/owner/repo/issues/comments/123/reactions"))
    .and(header("authorization", "Bearer fake-github-token-123"))
    .respond_with(ResponseTemplate::new(201).set_body_json(json!({
        "id": 1,
        "content": "+1"
    })))
    .mount(&mock_server)
    .await;

  let platform = create_test_github_platform(&mock_server).await?;
  let result = platform.add_reaction("owner", "repo", "123", ReactionType::ThumbsUp).await?;

  assert!(result);

  reset_provider_factory();
  Ok(())
}

#[tokio::test]
#[serial]
async fn test_add_reaction_heart() -> Result<()> {
  let mock_server = MockServer::start().await;

  Mock::given(method("POST"))
    .and(path("/repos/owner/repo/issues/comments/456/reactions"))
    .and(header("authorization", "Bearer fake-github-token-123"))
    .respond_with(ResponseTemplate::new(201).set_body_json(json!({
        "id": 2,
        "content": "heart"
    })))
    .mount(&mock_server)
    .await;

  let platform = create_test_github_platform(&mock_server).await?;
  let result = platform.add_reaction("owner", "repo", "456", ReactionType::Heart).await?;

  assert!(result);

  reset_provider_factory();
  Ok(())
}

#[tokio::test]
#[serial]
async fn test_remove_reaction_success() -> Result<()> {
  let mock_server = MockServer::start().await;

  // First mock getting existing reactions
  Mock::given(method("GET"))
    .and(path("/repos/owner/repo/issues/comments/123/reactions"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!([
      github_mock_data::mock_reaction(json!({
        "user": github_mock_data::mock_user(json!({
          "login": "current-user",
          "id": 123
        }))
      }))
    ])))
    .mount(&mock_server)
    .await;

  // Then mock deleting the reaction - GitHub API format
  Mock::given(method("DELETE"))
    .and(path("/repos/owner/repo/issues/comments/123/reactions/1"))
    .respond_with(ResponseTemplate::new(204).set_body_string(""))
    .mount(&mock_server)
    .await;

  let platform = create_test_github_platform(&mock_server).await?;
  let result = platform.remove_reaction("owner", "repo", "123", ReactionType::ThumbsUp).await?;

  assert!(result);

  reset_provider_factory();
  Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_reactions() -> Result<()> {
  let mock_server = MockServer::start().await;

  Mock::given(method("GET"))
    .and(path("/repos/owner/repo/issues/comments/123/reactions"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!([
      github_mock_data::mock_reaction(json!({
        "id": 1,
        "content": "+1",
        "user": github_mock_data::mock_user(json!({
          "login": "user1",
          "id": 123
        }))
      })),
      github_mock_data::mock_reaction(json!({
        "id": 2,
        "node_id": "MDg6UmVhY3Rpb24y",
        "content": "heart",
        "user": github_mock_data::mock_user(json!({
          "login": "user2",
          "id": 456
        }))
      })),
      github_mock_data::mock_reaction(json!({
        "id": 3,
        "node_id": "MDg6UmVhY3Rpb24z",
        "content": "laugh",
        "user": github_mock_data::mock_user(json!({
          "login": "user3",
          "id": 789
        }))
      }))
    ])))
    .mount(&mock_server)
    .await;

  let platform = create_test_github_platform(&mock_server).await?;
  let reactions = platform.get_reactions("owner", "repo", "123").await?;

  assert_eq!(reactions.len(), 3);
  assert!(reactions.contains(&ReactionType::ThumbsUp));
  assert!(reactions.contains(&ReactionType::Heart));
  assert!(reactions.contains(&ReactionType::Laugh));

  reset_provider_factory();
  Ok(())
}

#[tokio::test]
#[serial]
async fn test_add_comment() -> Result<()> {
  let mock_server = MockServer::start().await;

  Mock::given(method("POST"))
    .and(path("/repos/owner/repo/issues/123/comments"))
    .respond_with(ResponseTemplate::new(201).set_body_json(github_mock_data::mock_comment(json!({
      "id": 789,
      "body": "Test comment",
      "user": github_mock_data::mock_user(json!({
        "id": 456,
        "login": "commenter"
      })),
      "created_at": "2023-01-01T15:00:00Z",
      "updated_at": "2023-01-01T15:00:00Z"
    }))))
    .mount(&mock_server)
    .await;

  let platform = create_test_github_platform(&mock_server).await?;
  let note = platform.add_comment("owner", "repo", "123", "Test comment").await?;

  assert_eq!(note.id, "789");
  assert_eq!(note.body, "Test comment");
  assert_eq!(note.author.username, "commenter");

  reset_provider_factory();
  Ok(())
}

#[tokio::test]
#[serial]
async fn test_format_comment_url() -> Result<()> {
  let mock_server = MockServer::start().await;
  let platform = create_test_github_platform(&mock_server).await?;

  let url = platform.format_comment_url("https://github.com/owner/repo/pull/123", "456");
  assert_eq!(url, "https://github.com/owner/repo/pull/123#issuecomment-456");

  reset_provider_factory();
  Ok(())
}

#[tokio::test]
#[serial]
async fn test_format_merge_request_url() -> Result<()> {
  let mock_server = MockServer::start().await;
  let platform = create_test_github_platform(&mock_server).await?;

  let url = platform.format_merge_request_url("owner", "repo", 123);
  assert_eq!(url, "https://github.com/owner/repo/pull/123");

  reset_provider_factory();
  Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_diffs() -> Result<()> {
  let mock_server = MockServer::start().await;

  Mock::given(method("GET"))
    .and(path("/repos/owner/repo/pulls/123/files"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!([
      github_mock_data::mock_file_diff(json!({
        "filename": "src/main.rs",
        "patch": "@@ -1,3 +1,4 @@\n fn main() {\n+    println!(\"Hello\");\n }",
        "previous_filename": "src/main.rs"
      })),
      github_mock_data::mock_file_diff(json!({
        "filename": "README.md",
        "status": "added",
        "additions": 20,
        "deletions": 0,
        "changes": 20,
        "patch": "@@ -0,0 +1,20 @@\n+# New README\n+\n+This is a new file.",
        "previous_filename": null
      }))
    ])))
    .mount(&mock_server)
    .await;

  let platform = create_test_github_platform(&mock_server).await?;
  let diffs = platform.get_diffs("owner", "repo", 123).await?;

  assert_eq!(diffs.len(), 2);
  assert_eq!(diffs[0].old_path, None); // modified file has same old/new path
  assert_eq!(diffs[0].new_path, "src/main.rs");
  assert_eq!(diffs[0].diff, "@@ -1,3 +1,4 @@\n fn main() {\n+    println!(\"Hello\");\n }");
  assert_eq!(diffs[1].new_path, "README.md");

  reset_provider_factory();
  Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_pipelines_returns_empty() -> Result<()> {
  let mock_server = MockServer::start().await;
  let platform = create_test_github_platform(&mock_server).await?;

  // GitHub platform doesn't implement pipelines yet, should return empty
  let pipelines = platform.get_pipelines("owner", "repo", "abc123").await?;
  assert!(pipelines.is_empty());

  reset_provider_factory();
  Ok(())
}

#[tokio::test]
#[serial]
async fn test_resolve_discussion() -> Result<()> {
  let mock_server = MockServer::start().await;

  // Create a temporary session file for the test in the legacy location
  use std::fs;
  let home_dir = dirs::home_dir().expect("Could not find home directory");
  let session_dir = home_dir.join(".kernelle").join("code-reviews");
  fs::create_dir_all(&session_dir).ok();

  // Create a proper ReviewSession structure based on the actual session module
  let session_data = json!({
    "repository": {
      "owner": "owner",
      "name": "repo",
      "full_name": "owner/repo",
      "url": "https://github.com/owner/repo",
      "default_branch": "main"
    },
    "merge_request": {
      "id": "123",
      "number": 123,
      "title": "Test PR",
      "description": "Test description",
      "state": "Open",
      "source_branch": "feature-branch",
      "target_branch": "main",
      "url": "https://github.com/owner/repo/pull/123",
      "author": {
        "id": "1",
        "username": "testuser",
        "display_name": "Test User",
        "avatar_url": "https://github.com/testuser.png"
      },
      "assignee": null,
      "created_at": "2023-01-01T10:00:00Z",
      "updated_at": "2023-01-01T10:00:00Z"
    },
    "platform": "github",
    "thread_queue": [],
    "unresolved_threads": [],
    "discussions": {},
    "pipelines": [],
    "created_at": "2023-01-01T10:00:00Z",
    "updated_at": "2023-01-01T10:00:00Z"
  });

  let session_file = session_dir.join("session.json");
  fs::write(&session_file, serde_json::to_string_pretty(&session_data)?)?;

  // Mock the GraphQL query to find and resolve the thread
  Mock::given(method("POST"))
    .and(path("/graphql"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
        "data": {
            "repository": {
                "pullRequest": {
                    "reviewThreads": {
                        "nodes": [{
                            "id": "thread123",
                            "isResolved": false,
                            "comments": {
                                "nodes": [{
                                    "id": "comment456",
                                    "databaseId": 456
                                }]
                            }
                        }]
                    }
                }
            }
        }
    })))
    .mount(&mock_server)
    .await;

  let platform = create_test_github_platform(&mock_server).await?;
  let result = platform.resolve_discussion("owner", "repo", "456").await?;

  // Should return false when GraphQL mutation can't complete (expected in mock environment)
  assert!(!result);

  // Cleanup session file
  fs::remove_file(&session_file).ok();
  fs::remove_dir_all(session_dir).ok();

  reset_provider_factory();
  Ok(())
}
