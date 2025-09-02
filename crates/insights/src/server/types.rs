//! REST API types with schemars annotations for OpenAPI generation

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Base Response Structure
// ======================

/// Base response object for all API endpoints
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct BaseResponse<T> {
  /// API versioning information
  pub versioning: VersionInfo,

  /// Transaction ID for logging correlation
  pub transaction_id: Uuid,

  /// Optional error information
  #[serde(skip_serializing_if = "Vec::is_empty", default)]
  pub errors: Vec<ApiError>,

  /// Response data (generic for different endpoint types)
  #[serde(flatten)]
  pub data: T,
}

/// API versioning information
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct VersionInfo {
  /// The latest version of the API
  pub latest: String,

  /// The version of the API requested by the client
  pub requested: String,

  /// The version of the API that was used in producing the response
  pub resolved: String,
}

/// API error information
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ApiError {
  /// Error key, unique to the error source
  pub key: String,

  /// Human readable error message
  pub message: String,

  /// Error stack trace (if available)
  #[serde(default)]
  pub stack: Vec<String>,

  /// Additional error context
  #[serde(default)]
  pub context: serde_json::Value,
}

// Status/Version Endpoints
// =======================

/// Response for /version endpoint
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct VersionResponse {
  /// Current API version
  pub version: String,
}

/// Response for /api endpoint  
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ApiInfoResponse {
  /// Latest API version
  pub latest: String,

  /// Version information
  pub versions: ApiVersions,
}

/// API version details
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ApiVersions {
  /// Latest version
  pub latest: String,

  /// Currently active versions
  pub active: Vec<String>,
}

// Logs Endpoint
// =============

/// Response for /logs endpoint
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LogsResponse {
  /// JSON log entries
  pub logs: Vec<LogEntry>,
}

/// Individual log entry (re-exported from bentley)
pub type LogEntry = bentley::daemon_logs::LogEntry;

/// Request context information for logs (re-exported from bentley)
pub type LogContext = bentley::daemon_logs::LogContext;

// Insights Endpoints
// ==================

/// Request for /insights/add endpoint
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AddInsightRequest {
  /// Topic category
  pub topic: String,

  /// Insight name
  pub name: String,

  /// Brief overview
  pub overview: String,

  /// Detailed content
  pub details: String,
}

/// Request for /insights/update endpoint
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UpdateInsightRequest {
  /// Topic category
  pub topic: String,

  /// Insight name
  pub name: String,

  /// New overview (optional)
  pub overview: Option<String>,

  /// New details (optional)
  pub details: Option<String>,
}

/// Request for /insights/remove endpoint
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RemoveInsightRequest {
  /// Topic category
  pub topic: String,

  /// Insight name
  pub name: String,
}

/// Request for /insights/get endpoint
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetInsightRequest {
  /// Topic category
  pub topic: String,

  /// Insight name
  pub name: String,

  /// Return only overview (not details)
  #[serde(default)]
  pub overview_only: bool,
}

/// Response for /insights/get endpoint
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetInsightResponse {
  /// The requested insight
  pub insight: InsightData,
}

/// Full insight data
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct InsightData {
  /// Topic category
  pub topic: String,

  /// Insight name
  pub name: String,

  /// Brief overview
  pub overview: String,

  /// Detailed content
  pub details: String,

  /// Embedding version (if computed)
  pub embedding_version: Option<String>,

  /// When embedding was computed
  pub embedding_computed: Option<DateTime<Utc>>,
}

/// Request for /insights/list/insights endpoint
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListInsightsRequest {
  /// Optional filters (ANDed together)
  #[serde(default)]
  pub filters: Vec<InsightFilter>,
}

/// Filter for insight queries
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct InsightFilter {
  /// Metadata field name to filter on
  pub field: String,

  /// Expected value
  pub value: String,

  /// Comparison operation
  pub comparison: FilterComparison,
}

/// Filter comparison operations
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum FilterComparison {
  Equal,
  NotEqual,
  // Room for expansion: Contains, StartsWith, etc.
}

/// Response for /insights/list/insights endpoint
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListInsightsResponse {
  /// List of insights matching filters
  pub insights: Vec<InsightSummary>,
}

// Search Types
// ============

/// Search request data
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchRequest {
  /// Search terms (space-separated)
  pub terms: Vec<String>,
  
  /// Optional topic to restrict search to
  pub topic: Option<String>,
  
  /// Case-sensitive search
  #[serde(default)]
  pub case_sensitive: bool,
  
  /// Search only in overview sections
  #[serde(default)]
  pub overview_only: bool,
  
  /// Use exact term matching only
  #[serde(default)]
  pub exact: bool,
}

/// Search result data
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchResultData {
  /// Topic name
  pub topic: String,
  
  /// Insight name
  pub name: String,
  
  /// Overview content
  pub overview: String,
  
  /// Detail content
  pub details: String,
  
  /// Search score
  pub score: f32,
}

/// Search response data
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchResponse {
  /// Search results
  pub results: Vec<SearchResultData>,
  
  /// Number of results
  pub count: usize,
}

/// Response for /insights/list/topics endpoint
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListTopicsResponse {
  /// List of available topics
  pub topics: Vec<String>,
}

/// Summary information about an insight
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct InsightSummary {
  /// Topic category
  pub topic: String,

  /// Insight name
  pub name: String,

  /// Brief overview
  pub overview: String,

  /// Creation timestamp
  pub created_at: DateTime<Utc>,

  /// Last modified timestamp
  pub updated_at: DateTime<Utc>,
}

// Helper Functions
// ================

impl<T> BaseResponse<T> {
  /// Create a successful response
  pub fn success(data: T, transaction_id: Uuid) -> Self {
    let version = env!("CARGO_PKG_VERSION");
    Self {
      versioning: VersionInfo {
        latest: version.to_string(),
        requested: version.to_string(),
        resolved: version.to_string(),
      },
      transaction_id,
      errors: Vec::new(),
      data,
    }
  }

  /// Create an error response
  pub fn error(errors: Vec<ApiError>, transaction_id: Uuid) -> BaseResponse<()> {
    let version = env!("CARGO_PKG_VERSION");
    BaseResponse {
      versioning: VersionInfo {
        latest: version.to_string(),
        requested: version.to_string(),
        resolved: version.to_string(),
      },
      transaction_id,
      errors,
      data: (),
    }
  }
}

impl ApiError {
  /// Create a new API error
  pub fn new(key: &str, message: &str) -> Self {
    Self {
      key: key.to_string(),
      message: message.to_string(),
      stack: Vec::new(),
      context: serde_json::Value::Null,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use uuid::Uuid;

  #[test]
  fn test_base_response_success() {
    let transaction_id = Uuid::new_v4();
    let data = "test data".to_string();

    let response = BaseResponse::success(data.clone(), transaction_id);

    assert_eq!(response.transaction_id, transaction_id);
    assert!(response.errors.is_empty());
    assert_eq!(response.data, data);
    assert!(!response.versioning.latest.is_empty());
    assert_eq!(response.versioning.latest, response.versioning.requested);
    assert_eq!(response.versioning.latest, response.versioning.resolved);
  }

  #[test]
  fn test_base_response_error() {
    let transaction_id = Uuid::new_v4();
    let errors = vec![
      ApiError::new("test_error", "Test error message"),
      ApiError::new("another_error", "Another error message"),
    ];

    let response = BaseResponse::error(errors.clone(), transaction_id);

    assert_eq!(response.transaction_id, transaction_id);
    assert_eq!(response.errors.len(), 2);
    assert_eq!(response.errors[0].key, "test_error");
    assert_eq!(response.errors[0].message, "Test error message");
    assert_eq!(response.errors[1].key, "another_error");
    assert_eq!(response.errors[1].message, "Another error message");
  }

  #[test]
  fn test_api_error_new() {
    let error = ApiError::new("validation_error", "Invalid input provided");

    assert_eq!(error.key, "validation_error");
    assert_eq!(error.message, "Invalid input provided");
    assert!(error.stack.is_empty());
    assert!(error.context.is_null());
  }

  #[test]
  fn test_api_error_with_stack_and_context() {
    let mut error = ApiError::new("database_error", "Connection failed");
    error.stack = vec!["src/db.rs:42".to_string(), "src/handler.rs:15".to_string()];
    error.context = serde_json::json!({"connection_string": "postgres://localhost:5432/test"});

    assert_eq!(error.key, "database_error");
    assert_eq!(error.message, "Connection failed");
    assert_eq!(error.stack.len(), 2);
    assert_eq!(error.stack[0], "src/db.rs:42");
    assert!(!error.context.is_null());
  }

  #[test]
  fn test_filter_comparison_serialization() {
    use serde_json;

    let equal = FilterComparison::Equal;
    let json = serde_json::to_string(&equal).unwrap();
    assert_eq!(json, "\"equal\"");

    let not_equal = FilterComparison::NotEqual;
    let json = serde_json::to_string(&not_equal).unwrap();
    assert_eq!(json, "\"not-equal\"");
  }

  #[test]
  fn test_search_request_defaults() {
    let request = SearchRequest {
      terms: vec!["test".to_string()],
      topic: None,
      case_sensitive: false,
      overview_only: false,
      exact: false,
    };

    // These should all be false by default due to #[serde(default)]
    assert!(!request.case_sensitive);
    assert!(!request.overview_only);
    assert!(!request.exact);
  }

  #[test]
  fn test_get_insight_request_defaults() {
    let request = GetInsightRequest {
      topic: "test".to_string(),
      name: "test".to_string(),
      overview_only: false, // Should default to false
    };

    assert!(!request.overview_only);
  }
}
