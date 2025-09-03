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

  // Call library function directly (no HTTP client recursion!)
  // First load the existing insight
  let mut insight_data = match insight::load(&request.topic, &request.name) {
    Ok(insight) => insight,
    Err(e) => {
      let error = ApiError::new("insight_not_found", &format!("Insight not found: {e}"));
      return Err((
        axum::http::StatusCode::NOT_FOUND,
        ResponseJson(BaseResponse::<()>::error(vec![error], transaction_id)),
      ));
    }
  };

  // Then update it
      match insight::update(&mut insight_data, request.overview.as_deref(), request.details.as_deref())
  {
    Ok(()) => {
      // Update embedding in LanceDB
      match generate_and_store_embedding(&context, &insight_data).await {
        Ok(_) => {
          context
            .log_success(
              &format!("Successfully updated insight {}/{} with new embedding", insight_data.topic, insight_data.name),
              "insights-api",
            )
            .await;
        }
        Err(e) => {
          // Log warning but don't fail the request - insight was updated successfully
          context
            .log_warn(
              &format!("Insight updated but embedding update failed: {}", e),
              "insights-api",
            )
            .await;
        }
      }

      Ok(ResponseJson(BaseResponse::success((), transaction_id)))
    }
    Err(e) => {
      let error = ApiError::new("insight_update_failed", &format!("Failed to update insight: {e}"));
      Err((
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        ResponseJson(BaseResponse::<()>::error(vec![error], transaction_id)),
      ))
    }
  }
}

/// DELETE /insights/remove - Remove an insight
pub async fn remove_insight(
  Extension(context): Extension<RequestContext>,
  Json(request): Json<RemoveInsightRequest>,
) -> Result<ResponseJson<BaseResponse<()>>, (axum::http::StatusCode, ResponseJson<BaseResponse<()>>)> {
  let transaction_id = Uuid::new_v4();

  // Call library function directly (no HTTP client recursion!)
  let insight_to_delete = match insight::load(&request.topic, &request.name) {
    Ok(insight_data) => insight_data,
    Err(e) => {
      let error = ApiError::new("insight_not_found", &format!("Insight not found: {e}"));
      return Err((
        axum::http::StatusCode::NOT_FOUND,
        ResponseJson(BaseResponse::<()>::error(vec![error], transaction_id)),
      ));
    }
  };

  match insight::delete(&insight_to_delete) {
    Ok(()) => {
      // Delete embedding from LanceDB
      match context.lancedb.delete_embedding(&request.topic, &request.name).await {
        Ok(_) => {
          context
            .log_success(
              &format!("Successfully deleted insight {}/{} and its embedding", request.topic, request.name),
              "insights-api",
            )
            .await;
        }
        Err(e) => {
          // Log warning but don't fail the request - insight was deleted successfully
          context
            .log_warn(
              &format!("Insight deleted but embedding deletion failed: {}", e),
              "insights-api",
            )
            .await;
        }
      }

      Ok(ResponseJson(BaseResponse::success((), transaction_id)))
    }
    Err(e) => {
      let error = ApiError::new("insight_remove_failed", &format!("Failed to remove insight: {e}"));
      Err((
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        ResponseJson(BaseResponse::<()>::error(vec![error], transaction_id)),
      ))
    }
  }
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
  context.log_info("Loading all insights for re-indexing", "insights-reindex").await;

  // Load all insights from filesystem
  let all_insights = match insight::get_insights(None) {
    Ok(insights) => insights,
    Err(e) => {
      context.log_error(&format!("Failed to load insights: {e}"), "insights-reindex").await;
      return Err(e);
    }
  };

  let total_insights = all_insights.len();
  context
    .log_info(&format!("Found {total_insights} insights to re-index"), "insights-reindex")
    .await;

  // Clear existing LanceDB table to start fresh
  context.log_info("Clearing existing LanceDB table", "insights-reindex").await;

  // Delete table if it exists (LanceDB will recreate on first insert)
  let table_names = context.lancedb.connection.table_names().execute().await?;
  if table_names.contains(&context.lancedb.table_name) {
    let table = context.lancedb.get_table().await?;
    // LanceDB doesn't have a direct drop_table method, so we delete all records
    table.delete("id IS NOT NULL").await?;
    context.log_info("Cleared existing embeddings from LanceDB", "insights-reindex").await;
  }

  let mut embedded = 0;
  let mut errors = 0;

  for (index, insight) in all_insights.iter().enumerate() {
    // Log progress every 10 insights
    if (index + 1) % 10 == 0 || index == total_insights - 1 {
      context
        .log_info(
          &format!("Re-indexing progress: {}/{} (embedded: {}, errors: {})",
                   index + 1, total_insights, embedded, errors),
          "insights-reindex",
        )
        .await;
    }

    // Generate embedding for this insight
    match generate_and_store_embedding(&context, insight).await {
      Ok(_) => {
        embedded += 1;
      }
      Err(e) => {
        errors += 1;
        context.log_warn(
          &format!("Failed to generate embedding for {}/{}: {}", insight.topic, insight.name, e),
          "insights-reindex"
        ).await;
        // Continue with next insight - don't fail the entire reindexing process
      }
    }

    // Small delay to prevent overwhelming the embedding service
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
  }

  context
    .log_success(
      &format!("Re-indexing completed: {embedded}/{total_insights} insights embedded successfully, {errors} errors"),
      "insights-reindex",
    )
    .await;

  Ok(())
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

  // Remove limit entirely to see all results in database
  let limit = usize::MAX; // No limit - show everything

  // Set similarity threshold for normalized embeddings (temporarily lowered for debugging)
  // 0.3 = somewhat similar, 0.5 = moderately similar, 0.7 = very similar
  let threshold = Some(0.65);

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

  context
    .log_info(&format!("Adding insight {}/{}", request.topic, request.name), "insights-api")
    .await;

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
    Ok(()) => {
      // Generate and store embedding in LanceDB
      match generate_and_store_embedding(&context, &new_insight).await {
        Ok(_) => {
          context
            .log_success(
              &format!("Successfully added insight {}/{} with embedding", new_insight.topic, new_insight.name),
              "insights-api",
            )
            .await;
        }
        Err(e) => {
          // Log warning but don't fail the request - insight was saved successfully
          context
            .log_warn(
              &format!("Insight saved but embedding storage failed: {}", e),
              "insights-api",
            )
            .await;
        }
      }

      Ok(ResponseJson(BaseResponse::success((), transaction_id)))
    }
    Err(e) => {
      context
        .log_error(
          &format!("Failed to add insight {}/{}: {}", new_insight.topic, new_insight.name, e),
          "insights-api",
        )
        .await;
      let error = ApiError::new("insight_add_failed", &format!("Failed to add insight: {e}"));
      Err((
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        ResponseJson(BaseResponse::<()>::error(vec![error], transaction_id)),
      ))
    }
  }
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

  context
    .log_info(
      &format!("Searching insights: terms={:?}, topic={:?}", request.terms, request.topic),
      "insights-api",
    )
    .await;

  // Convert request to search options
  let search_options = crate::server::services::search::SearchOptions {
    topic: request.topic.clone(),
    case_sensitive: request.case_sensitive,
    overview_only: request.overview_only,
    exact: request.exact,
    semantic: request.semantic,
  };

  // Perform search based on the requested mode
  let mut all_results = Vec::new();

  // Always perform term-based search (exact or semantic)
  match crate::server::services::search::search(&request.terms, &search_options) {
    Ok(search_results) => {
      context
        .log_info(&format!("Term search found {} results for {:?}", search_results.len(), request.terms), "insights-api")
        .await;

      // Convert SearchResult to SearchResultData format
      let term_results: Vec<SearchResultData> = search_results
        .into_iter()
        .map(|result| SearchResultData {
          topic: result.topic,
          name: result.name,
          overview: result.overview,
          details: result.details,
          score: result.score,
        })
        .collect();
      all_results.extend(term_results);
    }
    Err(e) => {
      context
        .log_error(&format!("Term search failed for {:?}: {}", request.terms, e), "insights-api")
        .await;
      let error = ApiError::new("search_failed", &format!("Term search failed: {e}"));
      return Err((
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        ResponseJson(BaseResponse::<()>::error(vec![error], transaction_id)),
      ));
    }
  }

  // Add embedding search if using full mode (not exact and not semantic)
  if !request.exact && !request.semantic {
    // Check if embeddings exist first
    match context.lancedb.has_embeddings().await {
      Ok(has_embeddings) => {
        if has_embeddings {
          context
            .log_info(&format!("Starting embedding search for {:?} (embeddings exist)", request.terms), "insights-api")
            .await;
        } else {
          context
            .log_info(&format!("Skipping embedding search for {:?} (no embeddings in database)", request.terms), "insights-api")
            .await;
          // Skip the embedding search entirely
          return Ok(ResponseJson(BaseResponse::success(
            SearchResponse { count: all_results.len(), results: all_results },
            transaction_id
          )));
        }
      }
      Err(e) => {
        context
          .log_error(&format!("Failed to check for embeddings: {}", e), "insights-api")
          .await;
      }
    }

    match perform_vector_search(&context, &request).await {
      Ok(embedding_results) => {
        context
          .log_info(&format!("Embedding search found {} results for {:?}", embedding_results.len(), request.terms), "insights-api")
          .await;
        all_results.extend(embedding_results);
      }
      Err(e) => {
        // Log the error but don't fail the entire search
        context
          .log_error(&format!("Embedding search failed for {:?}: {}", request.terms, e), "insights-api")
          .await;
      }
    }
  }

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

  Ok(ResponseJson(BaseResponse::success(response_data, transaction_id)))
}
