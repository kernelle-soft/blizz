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
    let response = VersionResponse {
        version: "1.0.0".to_string(),
    };
    
    Json(BaseResponse::success(response, transaction_id))
}

/// GET /api - Returns API information and supported versions
pub async fn api_info() -> Json<BaseResponse<ApiInfoResponse>> {
    let transaction_id = Uuid::new_v4();
    let response = ApiInfoResponse {
        latest: "1.0.0".to_string(),
        versions: ApiVersions {
            latest: "1.0.0".to_string(),
            active: vec!["1.0.0".to_string()],
        },
    };
    
    Json(BaseResponse::success(response, transaction_id))
}
