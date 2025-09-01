use anyhow::{anyhow, Result};
use colored::*;

use std::{env, path::PathBuf};

use crate::client::{get_client};
use crate::insight::{self, Insight, InsightMetaData};
use crate::server_manager::ensure_server_running;
use bentley::daemon_logs::{LogsRequest, LogsResponse};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

/// Add a new insight to the knowledge base (testable version with dependency injection)
pub async fn add_insight_with_client(
  topic: &str,
  name: &str,
  overview: &str,
  details: &str,
) -> Result<()> {
  let client = get_client();
  client.add_insight(topic, name, overview, details).await?;

  println!("{} Added insight {}/{}", "✓".green(), topic.cyan(), name.yellow());
  Ok(())
}

/// Add a new insight to the knowledge base (production version)
pub async fn add_insight(topic: &str, name: &str, overview: &str, details: &str) -> Result<()> {
  ensure_server_running().await?;
  add_insight_with_client(topic, name, overview, details).await
}

/// Get content of a specific insight
pub async fn get_insight(topic: &str, name: &str, overview_only: bool) -> Result<()> {
  ensure_server_running().await?;
  
  let client = get_client();
  let response = client.get_insight(topic, name, overview_only).await?;

  if overview_only {
    println!("{}", response.insight.overview);
  } else {
    println!("---\n{}\n---\n\n{}", response.insight.overview, response.insight.details);
  }

  Ok(())
}

pub async fn list_insights(filter: Option<&str>, verbose: bool) -> Result<()> {
  ensure_server_running().await?;
  
  let client = get_client();
  let response = client.list_insights(Vec::new()).await?; // TODO: Add topic filtering
  
  let insights = if let Some(topic_filter) = filter {
    response.insights.into_iter()
      .filter(|insight| insight.topic == topic_filter)
      .collect::<Vec<_>>()
  } else {
    response.insights
  };

  if insights.is_empty() {
    if let Some(topic) = filter {
      println!("No insights found in topic: {}", topic.yellow());
    } else {
      println!("No insights found.");
    }
    return Ok(());
  }

  for insight in insights {
    let formatted_name = if verbose {
      format!("{}/{} - {}", insight.topic.cyan(), insight.name.yellow(), insight.overview)
    } else {
      format!("{}/{}", insight.topic.cyan(), insight.name.yellow())
    };
    println!("{formatted_name}");
  }

  Ok(())
}

pub async fn list_topics() -> Result<()> {
  ensure_server_running().await?;
  
  let client = get_client();
  let topics = client.list_topics().await?;

  if topics.is_empty() {
    println!("No topics found.");
    return Ok(());
  }

  for topic in topics {
    println!("{}", topic.cyan());
  }

  Ok(())
}

/// Update an existing insight's overview and/or details
pub async fn update_insight_with_client(
  topic: &str,
  name: &str,
  new_overview: Option<&str>,
  new_details: Option<&str>,
) -> Result<()> {
  let client = get_client();
  client.update_insight(topic, name, new_overview, new_details).await?;

  println!("{} Updated insight {}/{}", "✓".green(), topic.cyan(), name.yellow());
  Ok(())
}

/// Update an existing insight's overview and/or details
pub async fn update_insight(
  topic: &str,
  name: &str,
  new_overview: Option<&str>,
  new_details: Option<&str>,
) -> Result<()> {
  ensure_server_running().await?;
  update_insight_with_client(topic, name, new_overview, new_details).await
}

/// Delete an insight
pub async fn delete_insight(topic: &str, name: &str, force: bool) -> Result<()> {
  ensure_server_running().await?;
  
  // Check if insight exists first
  let client = get_client();
  if let Err(e) = client.get_insight(topic, name, true).await {
    let error_msg = e.to_string();
    if error_msg.contains("insight_not_found") || error_msg.contains("not found") {
      return Err(anyhow!("Insight {}/{} not found", topic, name));
    } else {
      return Err(e);
    }
  }

  // Ask for confirmation unless --force is used
  if !force {
    use std::io::{self, Write};
    
    print!("Are you sure you want to delete insight {}/{}? [y/N]: ", topic.cyan(), name.yellow());
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    let response = input.trim().to_lowercase();
    if response != "y" && response != "yes" {
      println!("Delete cancelled.");
      return Ok(());
    }
  }

  // Delete the insight
  client.remove_insight(topic, name).await?;
  println!("{} Deleted insight {}/{}", "✓".green(), topic.cyan(), name.yellow());
  Ok(())
}

fn index_insight(insight: &mut Insight, _force: bool) -> Result<bool> {
  let file_path = insight::file_path(insight)?;
  let metadata = InsightMetaData {
    topic: insight.topic.clone(),
    name: insight.name.clone(),
    overview: insight.overview.clone(),
    embedding_version: insight.embedding_version.clone(),
    embedding: insight.embedding.clone(),
    embedding_text: insight.embedding_text.clone(),
    embedding_computed: insight.embedding_computed,
  };

  let yaml = serde_yaml::to_string(&metadata)?;
  let content = format!("---\n{}---\n\n# Details\n{}", yaml, insight.details);
  std::fs::write(&file_path, content)?;

  println!(
    "  {} Updated embeddings for {}/{}",
    "✓".green(),
    insight.topic.cyan(),
    insight.name.yellow()
  );

  Ok(true)
}

fn index_topics(topic: &str, force: bool) -> Result<(usize, usize)> {
  let insights = insight::get_insights(Some(topic))?;
  let total = insights.len();
  let mut updated = 0;

  for mut insight in insights {
    if index_insight(&mut insight, force)? {
      updated += 1;
    }
  }

  Ok((updated, total))
}

/// Recompute embeddings for insights (testable version with dependency injection)
pub fn index_insights(force: bool) -> Result<()> {
  let topics = insight::get_topics()?;

  if topics.is_empty() {
    println!("No topics found to index.");
    return Ok(());
  }

  let mut total_updated = 0;
  let mut total_processed = 0;

  for topic in &topics {
    let (updated, processed) = index_topics(topic, force)?;
    total_updated += updated;
    total_processed += processed;
  }

  println!(
    "{} Indexed {} of {} insights",
    "✓".green(),
    total_updated.to_string().yellow(),
    total_processed.to_string().cyan()
  );

  Ok(())
}

/// Query daemon logs for debugging and monitoring
pub fn query_daemon_logs(limit: usize, level: &str) -> Result<()> {
  // Use tokio runtime for async operations
  let rt = tokio::runtime::Runtime::new()?;
  rt.block_on(async { query_daemon_logs_async(limit, level).await })
}

async fn query_daemon_logs_async(limit: usize, level: &str) -> Result<()> {
  // Get daemon socket path
  let socket_path = get_daemon_socket_path()?;

  // Check if socket exists
  if !socket_path.exists() {
    return Err(anyhow!("Daemon is not running. Socket not found at: {}", socket_path.display()));
  }

  // Connect to daemon
  let mut stream = UnixStream::connect(&socket_path)
    .await
    .map_err(|e| anyhow!("Failed to connect to daemon: {}", e))?;

  // Create request
  let request = LogsRequest {
    request: "logs".to_string(),
    limit: Some(limit),
    level: if level == "all" { None } else { Some(level.to_string()) },
  };

  // Send request
  let request_json = serde_json::to_string(&request)?;
  stream
    .write_all(request_json.as_bytes())
    .await
    .map_err(|e| anyhow!("Failed to send request: {}", e))?;

  // Read response
  let mut response_data = Vec::new();
  stream
    .read_to_end(&mut response_data)
    .await
    .map_err(|e| anyhow!("Failed to read response: {}", e))?;

  // Parse response
  let response_str =
    String::from_utf8(response_data).map_err(|e| anyhow!("Invalid UTF-8 in response: {}", e))?;

  let response: LogsResponse =
    serde_json::from_str(&response_str).map_err(|e| anyhow!("Failed to parse response: {}", e))?;

  // Display results
  if !response.success {
    return Err(anyhow!("Daemon returned error"));
  }

  if response.logs.is_empty() {
    println!("No logs found matching criteria.");
    return Ok(());
  }

  println!("{} daemon logs (showing {} entries):", "📋".cyan(), response.logs.len());
  println!();

  for log in response.logs {
    let level_colored = match log.level.as_str() {
      "error" => log.level.red().bold(),
      "warn" => log.level.yellow().bold(),
      "info" => log.level.blue().bold(),
      _ => log.level.normal(),
    };

    let component_colored = log.component.green();
    let timestamp_colored = log.timestamp.format("%H:%M:%S").to_string().dimmed();

    println!("[{}] {} [{}] {}", timestamp_colored, level_colored, component_colored, log.message);
  }

  Ok(())
}

fn get_daemon_socket_path() -> Result<PathBuf> {
  let base = if let Ok(dir) = env::var("BLIZZ_HOME") {
    PathBuf::from(dir)
  } else {
    dirs::home_dir().ok_or_else(|| anyhow!("failed to determine home directory"))?.join(".blizz")
  };

  let socket_path = base.join("persistent").join("insights").join("daemon.sock");
  Ok(socket_path)
}
