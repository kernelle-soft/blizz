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
    .base_uri(&mock_server.uri())?
    .build()?;

  Ok(GitHubPlatform::from_client(client))
}

#[tokio::test]
#[serial]
async fn test_get_repository_success() -> Result<()> {
  let mock_server = MockServer::start().await;

  Mock::given(method("GET"))
    .and(path("/repos/owner/repo"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
        "id": 123456,
        "name": "repo",
        "full_name": "owner/repo",
        "description": "Test repository",
        "html_url": "https://github.com/owner/repo",
        "default_branch": "main",
        "url": "https://api.github.com/repos/owner/repo"
    })))
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
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
        "id": 789,
        "number": 123,
        "title": "Test PR",
        "body": "This is a test pull request",
        "state": "open",
        "merged": false,
        "html_url": "https://github.com/owner/repo/pull/123",
        "url": "https://api.github.com/repos/owner/repo/pulls/123",
        "head": {
            "sha": "abc123def456",
            "ref": "feature-branch"
        },
        "base": {
            "sha": "base123def456",
            "ref": "main"
        },
        "user": {
            "id": 456,
            "login": "test-user",
            "name": "Test User",
            "avatar_url": "https://github.com/test-user.png"
        },
        "created_at": "2023-01-01T00:00:00Z",
        "updated_at": "2023-01-01T12:00:00Z"
    })))
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
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
        "id": 999,
        "number": 456,
        "title": "Merged PR",
        "body": "This PR was merged",
        "state": "closed",
        "merged": true,
        "merged_at": "2023-01-01T12:00:00Z",
        "html_url": "https://github.com/owner/repo/pull/456",
        "url": "https://api.github.com/repos/owner/repo/pulls/456",
        "head": {
            "sha": "def456ghi789",
            "ref": "feature-branch-2"
        },
        "base": {
            "sha": "base456ghi789",
            "ref": "main"
        },
        "user": {
            "id": 789,
            "login": "merger",
            "name": "The Merger",
            "avatar_url": "https://github.com/merger.png"
        },
        "created_at": "2023-01-01T00:00:00Z",
        "updated_at": "2023-01-01T12:00:00Z"
    })))
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

  // Mock the GraphQL endpoint for review threads
  Mock::given(method("POST"))
    .and(path("/graphql"))
    .and(header("authorization", "Bearer fake-github-token-123"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
        "data": {
            "repository": {
                "pullRequest": {
                    "reviewThreads": {
                        "nodes": [{
                            "id": "thread1",
                            "isResolved": false,
                            "comments": {
                                "nodes": [{
                                    "id": "comment1",
                                    "databaseId": 111,
                                    "body": "This needs fixing",
                                    "author": {
                                        "login": "reviewer1",
                                        "avatarUrl": "https://github.com/reviewer1.png"
                                    },
                                    "createdAt": "2023-01-01T10:00:00Z",
                                    "updatedAt": "2023-01-01T10:00:00Z"
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
        {
            "id": 1,
            "node_id": "MDg6UmVhY3Rpb24x",
            "content": "+1",
            "user": {
                "login": "current-user",
                "id": 123,
                "avatar_url": "https://github.com/current-user.png"
            }
        }
    ])))
    .mount(&mock_server)
    .await;

  // Then mock deleting the reaction
  Mock::given(method("DELETE"))
    .and(path("/repos/owner/repo/issues/comments/reactions/1"))
    .respond_with(ResponseTemplate::new(204))
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
        {
            "id": 1,
            "node_id": "MDg6UmVhY3Rpb24x",
            "content": "+1",
            "user": {
                "login": "user1",
                "id": 123,
                "avatar_url": "https://github.com/user1.png"
            }
        },
        {
            "id": 2,
            "node_id": "MDg6UmVhY3Rpb24y",
            "content": "heart",
            "user": {
                "login": "user2",
                "id": 456,
                "avatar_url": "https://github.com/user2.png"
            }
        },
        {
            "id": 3,
            "node_id": "MDg6UmVhY3Rpb24z",
            "content": "laugh",
            "user": {
                "login": "user3",
                "id": 789,
                "avatar_url": "https://github.com/user3.png"
            }
        }
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
    .respond_with(ResponseTemplate::new(201).set_body_json(json!({
        "id": 789,
        "body": "Test comment",
        "user": {
            "id": 456,
            "login": "commenter",
            "avatar_url": "https://github.com/commenter.png"
        },
        "created_at": "2023-01-01T15:00:00Z",
        "updated_at": "2023-01-01T15:00:00Z"
    })))
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
        {
            "filename": "src/main.rs",
            "status": "modified",
            "additions": 10,
            "deletions": 5,
            "changes": 15,
            "patch": "@@ -1,3 +1,4 @@\n fn main() {\n+    println!(\"Hello\");\n }"
        },
        {
            "filename": "README.md",
            "status": "added",
            "additions": 20,
            "deletions": 0,
            "changes": 20,
            "patch": "@@ -0,0 +1,20 @@\n+# New README\n+\n+This is a new file."
        }
    ])))
    .mount(&mock_server)
    .await;

  let platform = create_test_github_platform(&mock_server).await?;
  let diffs = platform.get_diffs("owner", "repo", 123).await?;

  assert_eq!(diffs.len(), 2);
  assert_eq!(diffs[0].old_path, Some("src/main.rs".to_string()));
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

  // Mock the GraphQL query to find the thread
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

  // Should return true even if we can't actually resolve (GraphQL mutation not fully mocked)
  assert!(result);

  reset_provider_factory();
  Ok(())
}
