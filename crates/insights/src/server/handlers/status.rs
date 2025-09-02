//! Status and version endpoint handlers

use axum::{http::StatusCode, response::Json};
use uuid::Uuid;

use crate::server::models::insight;
use crate::server::types::{ApiInfoResponse, ApiVersions, BaseResponse, VersionResponse, StatusResponse};

/// GET /status - Health check endpoint
pub async fn status() -> Result<Json<BaseResponse<StatusResponse>>, StatusCode> {
  let transaction_id = Uuid::new_v4();
  let version = env!("CARGO_PKG_VERSION");
  
  // Get the current insights root path the server is using
  match insight::get_insights_root() {
    Ok(insights_root) => {
      let response = StatusResponse {
        status: "healthy".to_string(),
        insights_root: insights_root.to_string_lossy().to_string(),
        version: version.to_string(),
      };
      Ok(Json(BaseResponse::success(response, transaction_id)))
    }
    Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR)
  }
}

/// GET /version - Returns current API version
pub async fn version() -> Json<BaseResponse<VersionResponse>> {
  let transaction_id = Uuid::new_v4();
  let version = env!("CARGO_PKG_VERSION");
  let response = VersionResponse { version: version.to_string() };

  Json(BaseResponse::success(response, transaction_id))
}

/// GET /api - Returns API information and supported versions
pub async fn api_info() -> Json<BaseResponse<ApiInfoResponse>> {
  let transaction_id = Uuid::new_v4();
  let version = env!("CARGO_PKG_VERSION");
  let response = ApiInfoResponse {
    latest: version.to_string(),
    versions: ApiVersions { latest: version.to_string(), active: vec![version.to_string()] },
  };

  Json(BaseResponse::success(response, transaction_id))
}
