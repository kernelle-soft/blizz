//! Insights endpoint handlers

use anyhow::{anyhow, Result};
use axum::{
  extract::{Extension, Json},
  response::Json as ResponseJson,
};
use chrono::Utc;
use uuid::Uuid;

use crate::server::types::{
  AddInsightRequest, ApiError, BaseResponse, GetInsightRequest, GetInsightResponse, InsightData,
  InsightSummary, ListInsightsResponse, ListTopicsResponse, RemoveInsightRequest, SearchRequest,
  SearchResponse, SearchResultData, UpdateInsightRequest,
};
use crate::server::{middleware::RequestContext, models::insight};

/// PUT /insights/update - Update an existing insight
pub async fn update_insight(
  Extension(context): Extension<RequestContext>,
  Json(request): Json<UpdateInsightRequest>,
) -> Result<ResponseJson<BaseResponse<()>>, (axum::http::StatusCode, ResponseJson<BaseResponse<()>>)> {
  let transaction_id = Uuid::new_v4();
  
  let mut insight_data = load_existing_insight(&request, transaction_id)?;
  update_insight_with_embedding(&context, &mut insight_data, &request, transaction_id).await
}

/// Load existing insight or return not found error
fn load_existing_insight(
  request: &UpdateInsightRequest,
  transaction_id: Uuid,
) -> Result<insight::Insight, (axum::http::StatusCode, ResponseJson<BaseResponse<()>>)> {
  insight::load(&request.topic, &request.name)
    .map_err(|e| create_insight_not_found_error(e, transaction_id))
}

/// Update insight and regenerate embedding
async fn update_insight_with_embedding(
  context: &RequestContext,
  insight_data: &mut insight::Insight,
  request: &UpdateInsightRequest,
  transaction_id: Uuid,
) -> Result<ResponseJson<BaseResponse<()>>, (axum::http::StatusCode, ResponseJson<BaseResponse<()>>)> {
  
  perform_insight_update(insight_data, request, transaction_id)?;
  attempt_embedding_update(context, insight_data).await;
  
  Ok(ResponseJson(BaseResponse::success((), transaction_id)))
}

/// Perform the actual insight update operation
fn perform_insight_update(
  insight_data: &mut insight::Insight,
  request: &UpdateInsightRequest,
  transaction_id: Uuid,
) -> Result<(), (axum::http::StatusCode, ResponseJson<BaseResponse<()>>)> {
  insight::update(insight_data, request.overview.as_deref(), request.details.as_deref())
    .map_err(|e| create_insight_update_error(e, transaction_id))
}

/// Attempt to update embedding (non-fatal if fails)
async fn attempt_embedding_update(context: &RequestContext, insight: &insight::Insight) {
  match generate_and_store_embedding(context, insight).await {
    Ok(_) => {
      log_embedding_update_success(context, insight).await;
    }
    Err(e) => {
      log_embedding_update_warning(context, e).await;
    }
  }
}

/// Log successful insight update with embedding
async fn log_embedding_update_success(context: &RequestContext, insight: &insight::Insight) {
  context
    .log_success(
      &format!("Successfully updated insight {}/{} with new embedding", insight.topic, insight.name),
      "insights-api",
    )
    .await;
}

/// Log embedding update warning (non-fatal)
async fn log_embedding_update_warning(context: &RequestContext, error: anyhow::Error) {
  context
    .log_warn(
      &format!("Insight updated but embedding update failed: {}", error),
      "insights-api",
    )
    .await;
}

/// Create error response for insight not found
fn create_insight_not_found_error(
  error: anyhow::Error,
  transaction_id: Uuid,
) -> (axum::http::StatusCode, ResponseJson<BaseResponse<()>>) {
  let api_error = ApiError::new("insight_not_found", &format!("Insight not found: {error}"));
  (
    axum::http::StatusCode::NOT_FOUND,
    ResponseJson(BaseResponse::<()>::error(vec![api_error], transaction_id)),
  )
}

/// Create error response for insight update failure
fn create_insight_update_error(
  error: anyhow::Error,
  transaction_id: Uuid,
) -> (axum::http::StatusCode, ResponseJson<BaseResponse<()>>) {
  let api_error = ApiError::new("insight_update_failed", &format!("Failed to update insight: {error}"));
  (
    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
    ResponseJson(BaseResponse::<()>::error(vec![api_error], transaction_id)),
  )
}

/// DELETE /insights/remove - Remove an insight
pub async fn remove_insight(
  Extension(context): Extension<RequestContext>,
  Json(request): Json<RemoveInsightRequest>,
) -> Result<ResponseJson<BaseResponse<()>>, (axum::http::StatusCode, ResponseJson<BaseResponse<()>>)> {
  let transaction_id = Uuid::new_v4();
  
  let insight_to_delete = load_insight_for_deletion(&request, transaction_id)?;
  delete_insight_with_embedding(&context, &insight_to_delete, &request, transaction_id).await
}

/// Load insight for deletion or return not found error
fn load_insight_for_deletion(
  request: &RemoveInsightRequest,
  transaction_id: Uuid,
) -> Result<insight::Insight, (axum::http::StatusCode, ResponseJson<BaseResponse<()>>)> {
  insight::load(&request.topic, &request.name)
    .map_err(|e| create_insight_not_found_error(e, transaction_id))
}

/// Delete insight and its embedding
async fn delete_insight_with_embedding(
  context: &RequestContext,
  insight_to_delete: &insight::Insight,
  request: &RemoveInsightRequest,
  transaction_id: Uuid,
) -> Result<ResponseJson<BaseResponse<()>>, (axum::http::StatusCode, ResponseJson<BaseResponse<()>>)> {
  
  perform_insight_deletion(insight_to_delete, transaction_id)?;
  attempt_embedding_deletion(context, request).await;
  
  Ok(ResponseJson(BaseResponse::success((), transaction_id)))
}

/// Perform the actual insight deletion operation
fn perform_insight_deletion(
  insight_to_delete: &insight::Insight,
  transaction_id: Uuid,
) -> Result<(), (axum::http::StatusCode, ResponseJson<BaseResponse<()>>)> {
  insight::delete(insight_to_delete)
    .map_err(|e| create_insight_removal_error(e, transaction_id))
}

/// Attempt to delete embedding (non-fatal if fails)
async fn attempt_embedding_deletion(context: &RequestContext, request: &RemoveInsightRequest) {
  match context.lancedb.delete_embedding(&request.topic, &request.name).await {
    Ok(_) => {
      log_embedding_deletion_success(context, request).await;
    }
    Err(e) => {
      log_embedding_deletion_warning(context, e).await;
    }
  }
}

/// Log successful insight deletion with embedding
async fn log_embedding_deletion_success(context: &RequestContext, request: &RemoveInsightRequest) {
  context
    .log_success(
      &format!("Successfully deleted insight {}/{} and its embedding", request.topic, request.name),
      "insights-api",
    )
    .await;
}

/// Log embedding deletion warning (non-fatal)
async fn log_embedding_deletion_warning(context: &RequestContext, error: anyhow::Error) {
  context
    .log_warn(
      &format!("Insight deleted but embedding deletion failed: {}", error),
      "insights-api",
    )
    .await;
}

/// Create error response for insight removal failure
fn create_insight_removal_error(
  error: anyhow::Error,
  transaction_id: Uuid,
) -> (axum::http::StatusCode, ResponseJson<BaseResponse<()>>) {
  let api_error = ApiError::new("insight_remove_failed", &format!("Failed to remove insight: {error}"));
  (
    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
    ResponseJson(BaseResponse::<()>::error(vec![api_error], transaction_id)),
  )
}

/// DELETE /insights/clear - Clear all insights
pub async fn clear_insights(
) -> Result<ResponseJson<BaseResponse<()>>, (axum::http::StatusCode, ResponseJson<BaseResponse<()>>)> {
  let transaction_id = Uuid::new_v4();

  // TODO: Implement clear insights using existing logic
  Ok(ResponseJson(BaseResponse::success((), transaction_id)))
}

/// DELETE /insights/index - Re-index all insights (delete existing index and rebuild)
pub async fn reindex(
  Extension(context): Extension<RequestContext>,
) -> Result<ResponseJson<BaseResponse<()>>, (axum::http::StatusCode, ResponseJson<BaseResponse<()>>)> {
  let transaction_id = Uuid::new_v4();

  context.log_info("Starting insight re-indexing process", "insights-api").await;

  // Spawn fire-and-forget task to handle re-indexing
  tokio::spawn(async move {
    if let Err(e) = perform_reindexing(context.clone()).await {
      context.log_error(&format!("Re-indexing failed: {e}"), "insights-api").await;
    }
  });

  // Return immediately - don't wait for re-indexing to complete
  Ok(ResponseJson(BaseResponse::success((), transaction_id)))
}

/// Perform the actual re-indexing process (fire-and-forget)
async fn perform_reindexing(context: RequestContext) -> Result<()> {
  let all_insights = load_all_insights_for_reindexing(&context).await?;
  clear_existing_embeddings(&context).await?;
  let stats = process_insights_for_embedding(&context, &all_insights).await;
  log_reindexing_completion(&context, &stats).await;
  Ok(())
}

/// Load all insights from filesystem for re-indexing
async fn load_all_insights_for_reindexing(context: &RequestContext) -> Result<Vec<insight::Insight>> {
  context.log_info("Loading all insights for re-indexing", "insights-reindex").await;
  
  let all_insights = insight::get_insights(None)
    .map_err(|e| {
      // Log error but let caller handle the Result
      tokio::spawn({
        let context = context.clone();
        let error_msg = format!("Failed to load insights: {e}");
        async move {
          context.log_error(&error_msg, "insights-reindex").await;
        }
      });
      e
    })?;

  let total_insights = all_insights.len();
  context
    .log_info(&format!("Found {total_insights} insights to re-index"), "insights-reindex")
    .await;
    
  Ok(all_insights)
}

/// Clear existing embeddings from LanceDB to start fresh
async fn clear_existing_embeddings(context: &RequestContext) -> Result<()> {
  context.log_info("Clearing existing LanceDB table", "insights-reindex").await;

  let table_names = context.lancedb.connection.table_names().execute().await?;
  if table_names.contains(&context.lancedb.table_name) {
    let table = context.lancedb.get_table().await?;
    table.delete("id IS NOT NULL").await?;
    context.log_info("Cleared existing embeddings from LanceDB", "insights-reindex").await;
  }
  
  Ok(())
}

/// Statistics for tracking re-indexing progress
#[derive(Debug, Default)]
struct ReindexingStats {
  embedded: usize,
  errors: usize,
  total: usize,
}

/// Process all insights for embedding generation
async fn process_insights_for_embedding(context: &RequestContext, insights: &[insight::Insight]) -> ReindexingStats {
  let mut stats = ReindexingStats {
    total: insights.len(),
    ..Default::default()
  };
  
  for (index, insight) in insights.iter().enumerate() {
    log_progress_if_needed(context, index, &stats).await;
    
    match generate_and_store_embedding(context, insight).await {
      Ok(_) => stats.embedded += 1,
      Err(e) => {
        stats.errors += 1;
        context.log_warn(
          &format!("Failed to generate embedding for {}/{}: {}", insight.topic, insight.name, e),
          "insights-reindex"
        ).await;
      }
    }
    
    // Rate limiting to prevent overwhelming embedding service
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
  }
  
  stats
}

/// Log progress periodically during processing
async fn log_progress_if_needed(context: &RequestContext, index: usize, stats: &ReindexingStats) {
  if (index + 1) % 10 == 0 || index == stats.total - 1 {
    context
      .log_info(
        &format!(
          "Re-indexing progress: {}/{} (embedded: {}, errors: {})",
          index + 1, stats.total, stats.embedded, stats.errors
        ),
        "insights-reindex",
      )
      .await;
  }
}

/// Log final completion statistics
async fn log_reindexing_completion(context: &RequestContext, stats: &ReindexingStats) {
  context
    .log_success(
      &format!(
        "Re-indexing completed: {}/{} insights embedded successfully, {} errors",
        stats.embedded, stats.total, stats.errors
      ),
      "insights-reindex",
    )
    .await;
}

/// Generate embedding for an insight and store it in LanceDB
async fn generate_and_store_embedding(
  context: &RequestContext,
  insight: &insight::Insight,
) -> Result<()> {
  let embedding_text = format!("{} {} {} {}", insight.topic, insight.name, insight.overview, insight.details);

  // Generate embedding using the embedding service
  let embedding = crate::server::services::embeddings::create_embedding(&embedding_text)
    .await
    .map_err(|e| anyhow!("Failed to generate embedding: {}", e))?;

  // Create insight with embedding data
  let insight_with_embedding = insight::Insight {
    topic: insight.topic.clone(),
    name: insight.name.clone(),
    overview: insight.overview.clone(),
    details: insight.details.clone(),
    embedding_version: Some("gte-base-en-v1.5".to_string()),
    embedding: Some(embedding.clone()),
    embedding_text: Some(embedding_text),
    embedding_computed: Some(chrono::Utc::now()),
  };

  // Store in LanceDB
  context.lancedb.store_embedding(&insight_with_embedding).await?;

  // Update the insight file with embedding metadata
  insight::save_existing(&insight_with_embedding)?;

  Ok(())
}

/// Perform vector similarity search using LanceDB
async fn perform_vector_search(
  context: &RequestContext,
  request: &SearchRequest,
) -> Result<Vec<SearchResultData>> {
  // Create search query text from terms
  let query_text = request.terms.join(" ");

  // Generate embedding for the search query
  let query_embedding = crate::server::services::embeddings::create_embedding(&query_text)
    .await
    .map_err(|e| anyhow!("Failed to generate query embedding: {}", e))?;

  let limit = 1e12 as usize;
  let threshold = Some(0.5); // with normalized cosine similarity, this captures "more similar than different"

  // Perform vector search in LanceDB
  let similar_results = context.lancedb
    .search_similar(&query_embedding, limit, threshold)
    .await?;

  // Convert LanceDB results to SearchResultData format
  let mut search_results = Vec::new();

  for result in similar_results {
    // Load the full insight to get complete details
    match insight::load(&result.topic, &result.name) {
      Ok(_full_insight) => {
        search_results.push(SearchResultData {
          topic: result.topic,
          name: result.name,
          overview: result.overview,
          details: result.details,
          score: result.similarity,
        });
      }
      Err(e) => {
        // Log warning but continue with partial data
        context.log_warn(
          &format!("Failed to load full insight {}/{}: {}", result.topic, result.name, e),
          "insights-search"
        ).await;
      }
    }
  }

  // Sort by similarity score (highest first)
  search_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

  Ok(search_results)
}



/// GET /insights/list/topics - List all topics
pub async fn list_topics() -> Result<
  ResponseJson<BaseResponse<ListTopicsResponse>>,
  (axum::http::StatusCode, ResponseJson<BaseResponse<()>>),
> {
  let transaction_id = Uuid::new_v4();

  match insight::get_topics() {
    Ok(topics) => {
      let response = ListTopicsResponse { topics };
      Ok(ResponseJson(BaseResponse::success(response, transaction_id)))
    }
    Err(e) => {
      let error = ApiError::new("topics_list_failed", &format!("Failed to list topics: {e}"));
      Err((
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        ResponseJson(BaseResponse::<()>::error(vec![error], transaction_id)),
      ))
    }
  }
}

/// GET /insights/list/insights - List insights with optional filtering  
pub async fn list_insights() -> Result<
  ResponseJson<BaseResponse<ListInsightsResponse>>,
  (axum::http::StatusCode, ResponseJson<BaseResponse<()>>),
> {
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
          created_at: insight.embedding_computed.unwrap_or_else(Utc::now),
          updated_at: insight.embedding_computed.unwrap_or_else(Utc::now),
        })
        .collect();

      let response = ListInsightsResponse { insights: insight_summaries };
      Ok(ResponseJson(BaseResponse::success(response, transaction_id)))
    }
    Err(e) => {
      let error = ApiError::new("insights_list_failed", &format!("Failed to list insights: {e}"));
      Err((
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        ResponseJson(BaseResponse::<()>::error(vec![error], transaction_id)),
      ))
    }
  }
}

/// POST /insights/add - Add a new insight
#[axum::debug_handler]
pub async fn add_insight(
  Extension(context): Extension<RequestContext>,
  Json(request): Json<AddInsightRequest>,
) -> Result<ResponseJson<BaseResponse<()>>, (axum::http::StatusCode, ResponseJson<BaseResponse<()>>)> {
  let transaction_id = Uuid::new_v4();
  
  log_insight_addition_start(&context, &request).await;
  let new_insight = create_insight_from_request(request);
  
  save_insight_with_embedding(&context, &new_insight, transaction_id).await
}

/// Log the start of insight addition operation
async fn log_insight_addition_start(context: &RequestContext, request: &AddInsightRequest) {
  context
    .log_info(&format!("Adding insight {}/{}", request.topic, request.name), "insights-api")
    .await;
}

/// Create a new insight from the API request
fn create_insight_from_request(request: AddInsightRequest) -> insight::Insight {
  insight::Insight {
    topic: request.topic,
    name: request.name,
    overview: request.overview,
    details: request.details,
    embedding_version: None,
    embedding: None,
    embedding_text: None,
    embedding_computed: None,
  }
}

/// Save insight and attempt to generate embedding
async fn save_insight_with_embedding(
  context: &RequestContext,
  new_insight: &insight::Insight,
  transaction_id: Uuid,
) -> Result<ResponseJson<BaseResponse<()>>, (axum::http::StatusCode, ResponseJson<BaseResponse<()>>)> {
  
  insight::save(new_insight)
    .map_err(|e| create_insight_save_error(context, new_insight, e, transaction_id))?;
    
  attempt_embedding_generation(context, new_insight).await;
  
  Ok(ResponseJson(BaseResponse::success((), transaction_id)))
}

/// Attempt to generate and store embedding (non-fatal if fails)
async fn attempt_embedding_generation(context: &RequestContext, insight: &insight::Insight) {
  match generate_and_store_embedding(context, insight).await {
    Ok(_) => {
      log_embedding_success(context, insight).await;
    }
    Err(e) => {
      log_embedding_warning(context, e).await;
    }
  }
}

/// Log successful insight addition with embedding
async fn log_embedding_success(context: &RequestContext, insight: &insight::Insight) {
  context
    .log_success(
      &format!("Successfully added insight {}/{} with embedding", insight.topic, insight.name),
      "insights-api",
    )
    .await;
}

/// Log embedding generation warning (non-fatal)
async fn log_embedding_warning(context: &RequestContext, error: anyhow::Error) {
  context
    .log_warn(
      &format!("Insight saved but embedding storage failed: {}", error),
      "insights-api",
    )
    .await;
}

/// Create error response for insight save failure
fn create_insight_save_error(
  context: &RequestContext,
  insight: &insight::Insight,
  error: anyhow::Error,
  transaction_id: Uuid,
) -> (axum::http::StatusCode, ResponseJson<BaseResponse<()>>) {
  // Spawn async logging to avoid blocking the error response
  tokio::spawn({
    let context = context.clone();
    let topic = insight.topic.clone();
    let name = insight.name.clone();
    let error_msg = format!("Failed to add insight {}/{}: {}", topic, name, error);
    async move {
      context.log_error(&error_msg, "insights-api").await;
    }
  });
  
  let api_error = ApiError::new("insight_add_failed", &format!("Failed to add insight: {error}"));
  (
    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
    ResponseJson(BaseResponse::<()>::error(vec![api_error], transaction_id)),
  )
}

/// POST /insights/get - Get a specific insight
pub async fn get_insight(
  Extension(context): Extension<RequestContext>,
  Json(request): Json<GetInsightRequest>,
) -> Result<
  ResponseJson<BaseResponse<GetInsightResponse>>,
  (axum::http::StatusCode, ResponseJson<BaseResponse<()>>),
> {
  let transaction_id = Uuid::new_v4();

  context
    .log_info(&format!("Retrieving insight {}/{}", request.topic, request.name), "insights-api")
    .await;

  match insight::load(&request.topic, &request.name) {
    Ok(insight_data) => {
      context
        .log_success(
          &format!("Successfully retrieved insight {}/{}", request.topic, request.name),
          "insights-api",
        )
        .await;

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
      context
        .log_warn(
          &format!("Insight {}/{} not found: {}", request.topic, request.name, e),
          "insights-api",
        )
        .await;
      let error = ApiError::new("insight_get_failed", &format!("Failed to get insight: {e}"));
      Err((
        axum::http::StatusCode::NOT_FOUND,
        ResponseJson(BaseResponse::<()>::error(vec![error], transaction_id)),
      ))
    }
  }
}

/// POST /insights/search - Search insights
pub async fn search_insights(
  Extension(context): Extension<RequestContext>,
  Json(request): Json<SearchRequest>,
) -> Result<ResponseJson<BaseResponse<SearchResponse>>, (axum::http::StatusCode, ResponseJson<BaseResponse<()>>)>
{
  let transaction_id = Uuid::new_v4();
  
  log_search_start(&context, &request).await;
  let search_options = build_search_options(&request);
  
  let mut all_results = perform_term_search(&context, &request, &search_options, transaction_id).await?;
  
  let should_finalize = add_embedding_search_results(&context, &request, &mut all_results).await;
  
  if should_finalize {
    Ok(ResponseJson(finalize_search_results(&context, &request, all_results, transaction_id).await))
  } else {
    // No embeddings available - return results as-is
    let response_data = SearchResponse { count: all_results.len(), results: all_results };
    Ok(ResponseJson(BaseResponse::success(response_data, transaction_id)))
  }
}

/// Log the start of a search operation
async fn log_search_start(context: &RequestContext, request: &SearchRequest) {
  context
    .log_info(
      &format!("Searching insights: terms={:?}, topic={:?}", request.terms, request.topic),
      "insights-api",
    )
    .await;
}

/// Build search options from the request
fn build_search_options(request: &SearchRequest) -> crate::server::services::search::SearchOptions {
  crate::server::services::search::SearchOptions {
    topic: request.topic.clone(),
    case_sensitive: request.case_sensitive,
    overview_only: request.overview_only,
    exact: request.exact,
    semantic: request.semantic,
  }
}

/// Perform term-based search and return results
async fn perform_term_search(
  context: &RequestContext,
  request: &SearchRequest,
  search_options: &crate::server::services::search::SearchOptions,
  transaction_id: Uuid,
) -> Result<Vec<SearchResultData>, (axum::http::StatusCode, ResponseJson<BaseResponse<()>>)> {
  
  let search_results = crate::server::services::search::search(&request.terms, search_options)
    .map_err(|e| {
      let error_response = create_search_error_response(
        &format!("Term search failed: {e}"),
        transaction_id
      );
      tokio::spawn({
        let context = context.clone();
        let terms = request.terms.clone();
        let error = format!("Term search failed for {:?}: {}", terms, e);
        async move {
          context.log_error(&error, "insights-api").await;
        }
      });
      error_response
    })?;

  context
    .log_info(&format!("Term search found {} results for {:?}", search_results.len(), request.terms), "insights-api")
    .await;

  let term_results = convert_search_results_to_api_format(search_results);
  Ok(term_results)
}

/// Convert internal SearchResult to API SearchResultData format
fn convert_search_results_to_api_format(search_results: Vec<crate::server::services::search::SearchResult>) -> Vec<SearchResultData> {
  search_results
    .into_iter()
    .map(|result| SearchResultData {
      topic: result.topic,
      name: result.name,
      overview: result.overview,
      details: result.details,
      score: result.score,
    })
    .collect()
}

/// Add embedding search results if appropriate, returns true if should continue with finalization
async fn add_embedding_search_results(
  context: &RequestContext,
  request: &SearchRequest,
  all_results: &mut Vec<SearchResultData>,
) -> bool {
  
  // Skip embedding search if using exact or semantic-only modes
  if request.exact || request.semantic {
    return true;
  }

  // Check if embeddings exist and perform search
  match check_embeddings_availability(context, request).await {
    EmbeddingAvailability::Available => {
      if let Ok(embedding_results) = perform_vector_search(context, request).await {
        context
          .log_info(&format!("Embedding search found {} results for {:?}", embedding_results.len(), request.terms), "insights-api")
          .await;
        all_results.extend(embedding_results);
      } else {
        context
          .log_error(&format!("Embedding search failed for {:?}", request.terms), "insights-api")
          .await;
      }
      true
    }
    EmbeddingAvailability::Unavailable => {
      // No embeddings available - skip finalization and return results as-is
      false
    }
    EmbeddingAvailability::Error => {
      // Continue without embeddings on error
      true
    }
  }
}

/// Check if embeddings are available for search
async fn check_embeddings_availability(context: &RequestContext, request: &SearchRequest) -> EmbeddingAvailability {
  match context.lancedb.has_embeddings().await {
    Ok(true) => {
      context
        .log_info(&format!("Starting embedding search for {:?} (embeddings exist)", request.terms), "insights-api")
        .await;
      EmbeddingAvailability::Available
    }
    Ok(false) => {
      context
        .log_info(&format!("Skipping embedding search for {:?} (no embeddings in database)", request.terms), "insights-api")
        .await;
      EmbeddingAvailability::Unavailable
    }
    Err(e) => {
      context
        .log_error(&format!("Failed to check for embeddings: {}", e), "insights-api")
        .await;
      EmbeddingAvailability::Error
    }
  }
}

/// Embedding availability status
enum EmbeddingAvailability {
  Available,
  Unavailable,
  Error,
}

/// Sort, deduplicate, and create final response
async fn finalize_search_results(
  context: &RequestContext,
  request: &SearchRequest,
  mut all_results: Vec<SearchResultData>,
  transaction_id: Uuid,
) -> BaseResponse<SearchResponse> {
  
  // Sort and deduplicate results
  all_results.sort_by(|a, b| {
    b.score
      .partial_cmp(&a.score)
      .unwrap_or(std::cmp::Ordering::Equal)
      .then_with(|| a.topic.cmp(&b.topic).then_with(|| a.name.cmp(&b.name)))
  });

  all_results.dedup_by(|a, b| a.topic == b.topic && a.name == b.name);

  context
    .log_success(
      &format!("Search completed: found {} results for {:?}", all_results.len(), request.terms),
      "insights-api",
    )
    .await;

  let response_data = SearchResponse { count: all_results.len(), results: all_results };
  BaseResponse::success(response_data, transaction_id)
}

/// Create a standardized error response for search failures
fn create_search_error_response(
  message: &str,
  transaction_id: Uuid,
) -> (axum::http::StatusCode, ResponseJson<BaseResponse<()>>) {
  let error = ApiError::new("search_failed", message);
  (
    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
    ResponseJson(BaseResponse::<()>::error(vec![error], transaction_id)),
  )
}
