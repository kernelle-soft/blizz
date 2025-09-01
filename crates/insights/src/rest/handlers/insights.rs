//! Insights endpoint handlers

use axum::{extract::{Json, Query}, response::Json as ResponseJson, http::StatusCode};
use uuid::Uuid;
use chrono::Utc;

use crate::commands::{self};
use crate::insight;
use crate::rest::types::{
    AddInsightRequest, ApiError, BaseResponse, GetInsightRequest, GetInsightResponse,
    InsightData, InsightSummary, ListInsightsRequest, ListInsightsResponse, 
    ListTopicsResponse, RemoveInsightRequest, UpdateInsightRequest,
};

/// POST /insights/add - Add a new insight
pub async fn add_insight(
    Json(request): Json<AddInsightRequest>
) -> Result<ResponseJson<BaseResponse<()>>, (StatusCode, ResponseJson<BaseResponse<()>>)> {
    let transaction_id = Uuid::new_v4();
    
    // Create and save insight directly (no HTTP client recursion!)
    let new_insight = insight::Insight {
        topic: request.topic,
        name: request.name,
        overview: request.overview,
        details: request.details,
        embedding_version: None,
        embedding: None,
        embedding_text: None,
        embedding_computed: None,
    };
    
    match insight::save(&new_insight) {
        Ok(()) => Ok(ResponseJson(BaseResponse::success((), transaction_id))),
        Err(e) => {
            let error = ApiError::new("insight_add_failed", &format!("Failed to add insight: {}", e));
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                ResponseJson(BaseResponse::<()>::error(vec![error], transaction_id))
            ))
        }
    }
}

/// POST /insights/get - Get a specific insight
pub async fn get_insight(
    Json(request): Json<GetInsightRequest>
) -> Result<ResponseJson<BaseResponse<GetInsightResponse>>, (StatusCode, ResponseJson<BaseResponse<()>>)> {
    let transaction_id = Uuid::new_v4();
    
    match insight::load(&request.topic, &request.name) {
        Ok(insight_data) => {
            let insight = InsightData {
                topic: insight_data.topic,
                name: insight_data.name,
                overview: insight_data.overview,
                details: if request.overview_only { String::new() } else { insight_data.details },
                embedding_version: insight_data.embedding_version,
                embedding_computed: insight_data.embedding_computed,
            };
            let response = GetInsightResponse { insight };
            Ok(ResponseJson(BaseResponse::success(response, transaction_id)))
        }
        Err(e) => {
            let error = ApiError::new("insight_get_failed", &format!("Failed to get insight: {}", e));
            Err((
                StatusCode::NOT_FOUND,
                ResponseJson(BaseResponse::<()>::error(vec![error], transaction_id))
            ))
        }
    }
}

/// PUT /insights/update - Update an existing insight  
pub async fn update_insight(
    Json(request): Json<UpdateInsightRequest>
) -> Result<ResponseJson<BaseResponse<()>>, (StatusCode, ResponseJson<BaseResponse<()>>)> {
    let transaction_id = Uuid::new_v4();
    
    // Call library function directly (no HTTP client recursion!)
    // First load the existing insight
    let mut insight_data = match insight::load(&request.topic, &request.name) {
        Ok(insight) => insight,
        Err(e) => {
            let error = ApiError::new("insight_not_found", &format!("Insight not found: {}", e));
            return Err((
                StatusCode::NOT_FOUND,
                ResponseJson(BaseResponse::<()>::error(vec![error], transaction_id))
            ));
        }
    };
    
    // Then update it
    match insight::update(&mut insight_data, request.overview.as_deref(), request.details.as_deref()) {
        Ok(()) => Ok(ResponseJson(BaseResponse::success((), transaction_id))),
        Err(e) => {
            let error = ApiError::new("insight_update_failed", &format!("Failed to update insight: {}", e));
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                ResponseJson(BaseResponse::<()>::error(vec![error], transaction_id))
            ))
        }
    }
}

/// DELETE /insights/remove - Remove an insight
pub async fn remove_insight(
    Json(request): Json<RemoveInsightRequest>
) -> Result<ResponseJson<BaseResponse<()>>, (StatusCode, ResponseJson<BaseResponse<()>>)> {
    let transaction_id = Uuid::new_v4();
    
    // Call library function directly (no HTTP client recursion!)
    let insight_to_delete = match insight::load(&request.topic, &request.name) {
        Ok(insight_data) => insight_data,
        Err(e) => {
            let error = ApiError::new("insight_not_found", &format!("Insight not found: {}", e));
            return Err((
                StatusCode::NOT_FOUND,
                ResponseJson(BaseResponse::<()>::error(vec![error], transaction_id))
            ));
        }
    };
    
    match insight::delete(&insight_to_delete) {
        Ok(()) => Ok(ResponseJson(BaseResponse::success((), transaction_id))),
        Err(e) => {
            let error = ApiError::new("insight_remove_failed", &format!("Failed to remove insight: {}", e));
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                ResponseJson(BaseResponse::<()>::error(vec![error], transaction_id))
            ))
        }
    }
}

/// DELETE /insights/clear - Clear all insights
pub async fn clear_insights() -> Result<ResponseJson<BaseResponse<()>>, (StatusCode, ResponseJson<BaseResponse<()>>)> {
    let transaction_id = Uuid::new_v4();
    
    // TODO: Implement clear insights using existing logic
    Ok(ResponseJson(BaseResponse::success((), transaction_id)))
}

/// DELETE /insights/index - Re-index all insights  
pub async fn reindex() -> Result<ResponseJson<BaseResponse<()>>, (StatusCode, ResponseJson<BaseResponse<()>>)> {
    let transaction_id = Uuid::new_v4();
    
    // TODO: Implement re-indexing using existing logic
    Ok(ResponseJson(BaseResponse::success((), transaction_id)))
}

/// GET /insights/list/topics - List all topics
pub async fn list_topics() -> Result<ResponseJson<BaseResponse<ListTopicsResponse>>, (StatusCode, ResponseJson<BaseResponse<()>>)> {
    let transaction_id = Uuid::new_v4();
    
    match insight::get_topics() {
        Ok(topics) => {
            let response = ListTopicsResponse { topics };
            Ok(ResponseJson(BaseResponse::success(response, transaction_id)))
        }
        Err(e) => {
            let error = ApiError::new("topics_list_failed", &format!("Failed to list topics: {}", e));
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                ResponseJson(BaseResponse::<()>::error(vec![error], transaction_id))
            ))
        }
    }
}

/// GET /insights/list/insights - List insights with optional filtering  
pub async fn list_insights() -> Result<ResponseJson<BaseResponse<ListInsightsResponse>>, (StatusCode, ResponseJson<BaseResponse<()>>)> {
    let transaction_id = Uuid::new_v4();
    
    // For now, ignore filters and get all insights - we can add filtering later
    match insight::get_insights(None) {
        Ok(insights) => {
            let insight_summaries: Vec<InsightSummary> = insights
                .into_iter()
                .map(|insight| InsightSummary {
                    topic: insight.topic,
                    name: insight.name,
                    overview: insight.overview,
                    created_at: insight.embedding_computed.unwrap_or_else(|| Utc::now()),
                    updated_at: insight.embedding_computed.unwrap_or_else(|| Utc::now()),
                })
                .collect();
            
            let response = ListInsightsResponse {
                insights: insight_summaries,
            };
            Ok(ResponseJson(BaseResponse::success(response, transaction_id)))
        }
        Err(e) => {
            let error = ApiError::new("insights_list_failed", &format!("Failed to list insights: {}", e));
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                ResponseJson(BaseResponse::<()>::error(vec![error], transaction_id))
            ))
        }
    }
}

/// GET /insights/search - Search insights
pub async fn search_insights() -> Result<ResponseJson<BaseResponse<()>>, (StatusCode, ResponseJson<BaseResponse<()>>)> {
    let transaction_id = Uuid::new_v4();
    
    // TODO: Implement using existing search logic from search.rs
    // TODO: Define search request/response types
    Ok(ResponseJson(BaseResponse::success((), transaction_id)))
}
