//! Logs endpoint handler

use axum::{response::Json, http::StatusCode};
use bentley::daemon_logs::DaemonLogs;
use std::path::Path;
use uuid::Uuid;

use crate::rest::types::{ApiError, BaseResponse, LogEntry, LogsResponse};

/// GET /logs - Get all logs since server start
pub async fn get_logs() -> Result<Json<BaseResponse<LogsResponse>>, (StatusCode, Json<BaseResponse<()>>)> {
    let transaction_id = Uuid::new_v4();
    
    // TODO: This should use a shared DaemonLogs instance
    // For now, we'll create a temporary one to demonstrate the structure
    let logs_path = get_logs_path();
    
    match DaemonLogs::new(&logs_path) {
        Ok(daemon_logs) => {
            match daemon_logs.get_logs(None, None).await {
                Ok(log_entries) => {
                    let logs: Vec<LogEntry> = log_entries
                        .into_iter()
                        .map(|entry| LogEntry {
                            timestamp: entry.timestamp,
                            level: entry.level,
                            message: entry.message,
                            component: entry.component,
                        })
                        .collect();
                    
                    let response = LogsResponse { logs };
                    Ok(Json(BaseResponse::success(response, transaction_id)))
                }
                Err(e) => {
                    let error = ApiError::new("logs_read_failed", &format!("Failed to read logs: {e}"));
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(BaseResponse::<()>::error(vec![error], transaction_id))
                    ))
                }
            }
        }
        Err(e) => {
            let error = ApiError::new("logs_init_failed", &format!("Failed to initialize logs: {e}"));
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(BaseResponse::<()>::error(vec![error], transaction_id))
            ))
        }
    }
}

/// Get the path to the logs file
/// TODO: This should be shared configuration
fn get_logs_path() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| Path::new("/tmp").to_path_buf())
        .join(".insights")
        .join("rest_server.logs.jsonl")
}
