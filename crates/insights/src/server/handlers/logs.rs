//! Logs endpoint handler

use axum::{response::Json, http::StatusCode, extract::Extension};
use uuid::Uuid;

use crate::server::{
    middleware::RequestContext,
    types::{ApiError, BaseResponse, LogEntry, LogsResponse}
};

/// GET /logs - Get all logs using request context
pub async fn get_logs_with_context(
    Extension(context): Extension<RequestContext>
) -> Result<Json<BaseResponse<LogsResponse>>, (StatusCode, Json<BaseResponse<()>>)> {
    let transaction_id = Uuid::new_v4();
    
    context.log_info("Retrieving server logs", "logs-api").await;
    
    match context.logger.get_logs(Some(100), None).await { // Limit to last 100 entries
        Ok(log_entries) => {
            // No need to parse - bentley now natively provides structured context!
            let logs: Vec<LogEntry> = log_entries;
            
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

