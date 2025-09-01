//! Status and version endpoint handlers

use axum::{response::Json, http::StatusCode};
use uuid::Uuid;

use crate::rest::types::{
    ApiInfoResponse, ApiVersions, BaseResponse, VersionResponse,
};

/// GET /status - Health check endpoint
pub async fn status() -> StatusCode {
    StatusCode::OK
}

/// GET /version - Returns current API version
pub async fn version() -> Json<BaseResponse<VersionResponse>> {
    let transaction_id = Uuid::new_v4();
    let version = env!("CARGO_PKG_VERSION");
    let response = VersionResponse {
        version: version.to_string(),
    };
    
    Json(BaseResponse::success(response, transaction_id))
}

/// GET /api - Returns API information and supported versions
pub async fn api_info() -> Json<BaseResponse<ApiInfoResponse>> {
    let transaction_id = Uuid::new_v4();
    let version = env!("CARGO_PKG_VERSION");
    let response = ApiInfoResponse {
        latest: version.to_string(),
        versions: ApiVersions {
            latest: version.to_string(),
            active: vec![version.to_string()],
        },
    };
    
    Json(BaseResponse::success(response, transaction_id))
}
