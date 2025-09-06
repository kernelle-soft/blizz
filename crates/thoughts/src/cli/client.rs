//! HTTP client for insights REST API
//!
//! This module provides a thin HTTP client wrapper that allows the CLI
//! to seamlessly work with both local and remote insights servers.

use anyhow::{anyhow, Result};
use reqwest::Client;

use std::time::Duration;
use tokio::time::timeout;

use crate::server::types::{
  AddInsightRequest, BaseResponse, GetInsightRequest, GetInsightResponse, InsightFilter,
  ListInsightsResponse, ListTopicsResponse, RemoveInsightRequest, UpdateInsightRequest,
};

/// HTTP method types for REST API calls
#[derive(Debug, Copy, Clone)]
enum HttpMethod {
  Get,
  Post,
  Put,
  Delete,
}

impl std::fmt::Display for HttpMethod {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let method_str = match self {
      HttpMethod::Get => "GET",
      HttpMethod::Post => "POST",
      HttpMethod::Put => "PUT",
      HttpMethod::Delete => "DELETE",
    };
    write!(f, "{method_str}")
  }
}

/// Configuration for the insights HTTP client
#[derive(Debug, Clone)]
pub struct ClientConfig {
  /// Base URL of the insights server (e.g., "http://localhost:3000")
  pub base_url: String,
  /// Request timeout in seconds
  pub timeout_secs: u64,
}

impl Default for ClientConfig {
  fn default() -> Self {
    Self { base_url: "http://localhost:3000".to_string(), timeout_secs: 30 }
  }
}

/// HTTP client for insights REST API
pub struct InsightsClient {
  client: Client,
  config: ClientConfig,
}

impl Default for InsightsClient {
  fn default() -> Self {
    Self::new()
  }
}

/// Helpers to handle HTTP response parsing and error handling
async fn parse_response<R>(
  response: reqwest::Response,
  method: HttpMethod,
  endpoint: &str,
) -> Result<R>
where
  R: serde::de::DeserializeOwned,
{
  if !response.status().is_success() {
    let error_text = response.text().await?;
    return Err(anyhow!("Failed {method} {endpoint}: {error_text}"));
  }

  let result: BaseResponse<R> = response.json().await?;
  Ok(result.data)
}

// Client Constructor
// ==================
impl InsightsClient {
  /// Create a new client with default configuration
  pub fn new() -> Self {
    Self::with_config(ClientConfig::default())
  }

  /// Create a new client with custom configuration
  pub fn with_config(config: ClientConfig) -> Self {
    let client = Client::builder()
      .timeout(Duration::from_secs(config.timeout_secs))
      .build()
      .expect("Failed to create HTTP client");

    Self { client, config }
  }
}

// Client Methods
// ==============
impl InsightsClient {
  /// Add a new insight
  pub async fn add_insight(
    &self,
    topic: &str,
    name: &str,
    overview: &str,
    details: &str,
  ) -> Result<()> {
    let request = AddInsightRequest {
      topic: topic.to_string(),
      name: name.to_string(),
      overview: overview.to_string(),
      details: details.to_string(),
    };

    self.post_json::<AddInsightRequest, ()>("/insights/add", &request).await
  }

  /// Get a specific insight
  pub async fn get_insight(
    &self,
    topic: &str,
    name: &str,
    overview_only: bool,
  ) -> Result<GetInsightResponse> {
    let request =
      GetInsightRequest { topic: topic.to_string(), name: name.to_string(), overview_only };

    self.post_json("/insights/get", &request).await
  }

  /// Update an existing insight
  pub async fn update_insight(
    &self,
    topic: &str,
    name: &str,
    overview: Option<&str>,
    details: Option<&str>,
  ) -> Result<()> {
    let request = UpdateInsightRequest {
      topic: topic.to_string(),
      name: name.to_string(),
      overview: overview.map(|s| s.to_string()),
      details: details.map(|s| s.to_string()),
    };

    self.put_json::<UpdateInsightRequest, ()>("/insights/update", &request).await
  }

  /// Remove an insight
  pub async fn remove_insight(&self, topic: &str, name: &str) -> Result<()> {
    let request = RemoveInsightRequest { topic: topic.to_string(), name: name.to_string() };

    self.delete_json::<RemoveInsightRequest, ()>("/insights/remove", &request).await
  }

  /// List all topics
  pub async fn list_topics(&self) -> Result<Vec<String>> {
    let response: ListTopicsResponse = self.get_json("/insights/list/topics").await?;
    Ok(response.topics)
  }

  /// List insights with optional filtering
  pub async fn list_insights(&self, _filters: Vec<InsightFilter>) -> Result<ListInsightsResponse> {
    // TODO: Add query parameter support
    self.get_json("/insights/list/insights").await
  }

  /// Check if the server is reachable
  pub async fn health_check(&self) -> Result<()> {
    let url = format!("{}/status", self.config.base_url);
    let response = timeout(
      Duration::from_secs(5), // Shorter timeout for health check
      self.client.get(&url).send(),
    )
    .await??;

    if response.status().is_success() {
      Ok(())
    } else {
      Err(anyhow!("Server health check failed: {}", response.status()))
    }
  }

  /// Get server logs
  pub async fn get_logs(
    &self,
  ) -> Result<crate::server::types::BaseResponse<crate::server::types::LogsResponse>> {
    // Return the full BaseResponse for backwards compatibility
    let url = format!("{}/logs", self.config.base_url);
    let response = self.execute_with_timeout(|| self.client.get(&url).send()).await?;

    if !response.status().is_success() {
      return Err(anyhow!("Failed to get logs: HTTP {}", response.status()));
    }

    Ok(response.json().await?)
  }

  /// Search insights
  pub async fn search_insights(
    &self,
    terms: Vec<String>,
    topic: Option<String>,
    case_sensitive: bool,
    overview_only: bool,
    exact: bool,
    semantic: bool,
  ) -> Result<crate::server::types::SearchResponse> {
    use crate::server::types::SearchRequest;

    let request = SearchRequest { terms, topic, case_sensitive, overview_only, exact, semantic };
    self.post_json("/insights/search", &request).await
  }

  /// Re-index all insights (fire-and-forget)
  pub async fn reindex_insights(&self) -> Result<()> {
    self.delete_without_body::<()>("/insights/index").await
  }
}

// HTTP Request Helpers
// ====================
impl InsightsClient {
  /// Helper to execute HTTP requests with timeout
  async fn execute_with_timeout<F, Fut>(&self, request_fn: F) -> Result<reqwest::Response>
  where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
  {
    timeout(Duration::from_secs(self.config.timeout_secs), request_fn()).await?.map_err(Into::into)
  }

  /// Helper to make a POST request with JSON body and return parsed response data
  async fn post_json<T, R>(&self, endpoint: &str, request: &T) -> Result<R>
  where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
  {
    let url = format!("{}{}", self.config.base_url, endpoint);
    let response =
      self.execute_with_timeout(|| self.client.post(&url).json(request).send()).await?;

    parse_response(response, HttpMethod::Post, endpoint).await
  }

  /// Helper to make a PUT request with JSON body and return parsed response data
  async fn put_json<T, R>(&self, endpoint: &str, request: &T) -> Result<R>
  where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
  {
    let url = format!("{}{}", self.config.base_url, endpoint);
    let response = self.execute_with_timeout(|| self.client.put(&url).json(request).send()).await?;

    parse_response(response, HttpMethod::Put, endpoint).await
  }

  /// Helper to make a DELETE request with JSON body and return parsed response data
  async fn delete_json<T, R>(&self, endpoint: &str, request: &T) -> Result<R>
  where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
  {
    let url = format!("{}{}", self.config.base_url, endpoint);
    let response =
      self.execute_with_timeout(|| self.client.delete(&url).json(request).send()).await?;

    parse_response(response, HttpMethod::Delete, endpoint).await
  }

  /// Helper to make a GET request and return parsed response data
  async fn get_json<R>(&self, endpoint: &str) -> Result<R>
  where
    R: serde::de::DeserializeOwned,
  {
    let url = format!("{}{}", self.config.base_url, endpoint);
    let response = self.execute_with_timeout(|| self.client.get(&url).send()).await?;

    parse_response(response, HttpMethod::Get, endpoint).await
  }

  /// Helper to make a DELETE request without body and return parsed response data
  async fn delete_without_body<R>(&self, endpoint: &str) -> Result<R>
  where
    R: serde::de::DeserializeOwned,
  {
    let url = format!("{}{}", self.config.base_url, endpoint);
    let response = self.execute_with_timeout(|| self.client.delete(&url).send()).await?;

    parse_response(response, HttpMethod::Delete, endpoint).await
  }
}

/// Get the configured client (checks environment variables)
pub fn get_client() -> InsightsClient {
  let base_url =
    std::env::var("INSIGHTS_SERVER_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

  let timeout_secs =
    std::env::var("INSIGHTS_TIMEOUT_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(30);

  let config = ClientConfig { base_url, timeout_secs };

  InsightsClient::with_config(config)
}
