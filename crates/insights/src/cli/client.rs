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

    let url = format!("{}/insights/add", self.config.base_url);
    let response = timeout(
      Duration::from_secs(self.config.timeout_secs),
      self.client.post(&url).json(&request).send(),
    )
    .await??;

    if !response.status().is_success() {
      let error_text = response.text().await?;
      return Err(anyhow!("Failed to add insight: {}", error_text));
    }

    let _result: BaseResponse<()> = response.json().await?;
    Ok(())
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

    let url = format!("{}/insights/get", self.config.base_url);
    let response = timeout(
      Duration::from_secs(self.config.timeout_secs),
      self.client.post(&url).json(&request).send(),
    )
    .await??;

    if !response.status().is_success() {
      let error_text = response.text().await?;
      return Err(anyhow!("Failed to get insight: {}", error_text));
    }

    let result: BaseResponse<GetInsightResponse> = response.json().await?;
    Ok(result.data)
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

    let url = format!("{}/insights/update", self.config.base_url);
    let response = timeout(
      Duration::from_secs(self.config.timeout_secs),
      self.client.put(&url).json(&request).send(),
    )
    .await??;

    if !response.status().is_success() {
      let error_text = response.text().await?;
      return Err(anyhow!("Failed to update insight: {}", error_text));
    }

    let _result: BaseResponse<()> = response.json().await?;
    Ok(())
  }

  /// Remove an insight
  pub async fn remove_insight(&self, topic: &str, name: &str) -> Result<()> {
    let request = RemoveInsightRequest { topic: topic.to_string(), name: name.to_string() };

    let url = format!("{}/insights/remove", self.config.base_url);
    let response = timeout(
      Duration::from_secs(self.config.timeout_secs),
      self.client.delete(&url).json(&request).send(),
    )
    .await??;

    if !response.status().is_success() {
      let error_text = response.text().await?;
      return Err(anyhow!("Failed to remove insight: {}", error_text));
    }

    let _result: BaseResponse<()> = response.json().await?;
    Ok(())
  }

  /// List all topics
  pub async fn list_topics(&self) -> Result<Vec<String>> {
    let url = format!("{}/insights/list/topics", self.config.base_url);
    let response =
      timeout(Duration::from_secs(self.config.timeout_secs), self.client.get(&url).send())
        .await??;

    if !response.status().is_success() {
      let error_text = response.text().await?;
      return Err(anyhow!("Failed to list topics: {}", error_text));
    }

    let result: BaseResponse<ListTopicsResponse> = response.json().await?;
    Ok(result.data.topics)
  }

  /// List insights with optional filtering
  pub async fn list_insights(&self, filters: Vec<InsightFilter>) -> Result<ListInsightsResponse> {
    // For now, we'll use GET without filters. TODO: Add query parameter support
    let url = format!("{}/insights/list/insights", self.config.base_url);
    let response =
      timeout(Duration::from_secs(self.config.timeout_secs), self.client.get(&url).send())
        .await??;

    if !response.status().is_success() {
      let error_text = response.text().await?;
      return Err(anyhow!("Failed to list insights: {}", error_text));
    }

    let result: BaseResponse<ListInsightsResponse> = response.json().await?;
    Ok(result.data)
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
    let url = format!("{}/logs", self.config.base_url);
    let response =
      timeout(Duration::from_secs(self.config.timeout_secs), self.client.get(&url).send())
        .await??;

    if !response.status().is_success() {
      return Err(anyhow!("Failed to get logs: HTTP {}", response.status()));
    }

    let logs_response: crate::server::types::BaseResponse<crate::server::types::LogsResponse> =
      response.json().await?;
    Ok(logs_response)
  }
}

/// Get the configured client (checks environment variables)
pub fn get_client() -> InsightsClient {
  let base_url =
    std::env::var("INSIGHTS_SERVER_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

  let timeout_secs = std::env::var("INSIGHTS_TIMEOUT_SECS")
    .unwrap_or_else(|_| "30".to_string())
    .parse()
    .unwrap_or(30);

  let config = ClientConfig { base_url, timeout_secs };

  InsightsClient::with_config(config)
}
