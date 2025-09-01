//! Logs endpoint handler

use axum::{response::Json, http::StatusCode, extract::Extension};
use uuid::Uuid;

use crate::server::{
    middleware::RequestContext,
    types::{ApiError, BaseResponse, LogEntry, LogsResponse, LogContext}
};

/// GET /logs - Get all logs using request context
pub async fn get_logs_with_context(
    Extension(context): Extension<RequestContext>
) -> Result<Json<BaseResponse<LogsResponse>>, (StatusCode, Json<BaseResponse<()>>)> {
    let transaction_id = Uuid::new_v4();
    
    context.log_info("Retrieving server logs", "logs-api").await;
    
    match context.logger.get_logs(Some(100), None).await { // Limit to last 100 entries
        Ok(log_entries) => {
            let logs: Vec<LogEntry> = log_entries
                .into_iter()
                .map(|entry| {
                    // Parse context from bentley's contextualized message format
                    let (clean_message, context) = extract_context_from_message(&entry.message);
                    
                    LogEntry {
                        timestamp: entry.timestamp,
                        level: entry.level,
                        message: clean_message,
                        component: entry.component,
                        context,
                    }
                })
                .collect();
            
            context.log_success(&format!("Retrieved {} log entries", logs.len()), "logs-api").await;
            let response = LogsResponse { logs };
            Ok(Json(BaseResponse::success(response, transaction_id)))
        }
        Err(e) => {
            context.log_error(&format!("Failed to read logs: {}", e), "logs-api").await;
            let error = ApiError::new("logs_read_failed", &format!("Failed to read logs: {}", e));
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(BaseResponse::<()>::error(vec![error], transaction_id))
            ))
        }
    }
}

/// Extract context from bentley's contextualized message format
/// Parses messages like "[uuid] METHOD /path - message (User-Agent: ..., Status: ..., Duration: ...)"
fn extract_context_from_message(message: &str) -> (String, Option<LogContext>) {
    use regex::Regex;
    
    // Pattern to match: [request_id] METHOD /path - actual_message (optional context)
    let re = Regex::new(r"^\[([^]]+)\]\s+(\w+)\s+(/[^\s]*)\s+-\s+(.+?)(?:\s+\((.+)\))?$").unwrap();
    
    if let Some(captures) = re.captures(message) {
        let request_id = captures.get(1).map(|m| m.as_str().to_string());
        let method = captures.get(2).map(|m| m.as_str().to_string());
        let path = captures.get(3).map(|m| m.as_str().to_string());
        let clean_message = captures.get(4).map(|m| m.as_str()).unwrap_or(message).to_string();
        
        // Parse optional context from parentheses
        let mut user_agent = None;
        let mut status_code = None;
        let mut duration_ms = None;
        
        if let Some(context_str) = captures.get(5) {
            let context_parts = context_str.as_str();
            
            // Parse User-Agent
            if let Some(ua_match) = Regex::new(r"User-Agent:\s*([^,]+)").unwrap().captures(context_parts) {
                user_agent = ua_match.get(1).map(|m| m.as_str().trim().to_string());
            }
            
            // Parse Status
            if let Some(status_match) = Regex::new(r"Status:\s*(\d+)").unwrap().captures(context_parts) {
                status_code = status_match.get(1).and_then(|m| m.as_str().parse().ok());
            }
            
            // Parse Duration
            if let Some(duration_match) = Regex::new(r"Duration:\s*([\d.]+)ms").unwrap().captures(context_parts) {
                duration_ms = duration_match.get(1).and_then(|m| m.as_str().parse().ok());
            }
        }
        
        let context = if request_id.is_some() || method.is_some() || path.is_some() || user_agent.is_some() || status_code.is_some() || duration_ms.is_some() {
            Some(LogContext {
                request_id,
                method,
                path,
                user_agent,
                duration_ms,
                status_code,
            })
        } else {
            None
        };
        
        (clean_message, context)
    } else {
        // If pattern doesn't match, return original message with no context
        (message.to_string(), None)
    }
}