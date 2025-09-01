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
    #[serde(skip_serializing_if = "Vec::is_empty")]
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

/// Individual log entry
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LogEntry {
    /// Log timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Log level
    pub level: String,
    
    /// Log message
    pub message: String,
    
    /// Component that generated the log
    pub component: String,
}

// Insights Endpoints
// ==================

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
        Self {
            versioning: VersionInfo {
                latest: "1.0.0".to_string(),
                requested: "1.0.0".to_string(), 
                resolved: "1.0.0".to_string(),
            },
            transaction_id,
            errors: Vec::new(),
            data,
        }
    }
    
    /// Create an error response
    pub fn error(errors: Vec<ApiError>, transaction_id: Uuid) -> BaseResponse<()> {
        BaseResponse {
            versioning: VersionInfo {
                latest: "1.0.0".to_string(),
                requested: "1.0.0".to_string(),
                resolved: "1.0.0".to_string(),
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
