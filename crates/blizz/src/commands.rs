use anyhow::{anyhow, Result};
use colored::*;
use std::fs;

use crate::insight::*;

/// Add a new insight to the knowledge base
pub fn add_insight(topic: &str, name: &str, overview: &str, details: &str) -> Result<()> {
    let insight = Insight::new(
        topic.to_string(),
        name.to_string(),
        overview.to_string(),
        details.to_string(),
    );

    insight.save()?;
    
    println!("{} Added insight {}/{}", "✓".green(), topic.cyan(), name.yellow());
    Ok(())
}

/// Search through all insights for matching content
pub fn search_insights(
    query: &str,
    topic_filter: Option<&str>,
    case_sensitive: bool,
    overview_only: bool,
) -> Result<()> {
    let insights_root = get_insights_root()?;
    
    if !insights_root.exists() {
        println!("No insights directory found");
        return Ok(());
    }

    let search_paths = if let Some(topic) = topic_filter {
        vec![insights_root.join(topic)]
    } else {
        get_topics()?
            .into_iter()
            .map(|topic| insights_root.join(topic))
            .collect()
    };

    let mut found_any = false;

    for topic_path in search_paths {
        if !topic_path.exists() {
            continue;
        }

        let topic_name = topic_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        for entry in fs::read_dir(&topic_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if file_stem.ends_with(".insight") {
                        let insight_name = file_stem.trim_end_matches(".insight");
                        
                        // Read and search the file content
                        if let Ok(content) = fs::read_to_string(&path) {
                            let search_content = if overview_only {
                                // Extract just the overview section
                                if let Ok((overview, _)) = parse_insight_content(&content) {
                                    overview
                                } else {
                                    continue;
                                }
                            } else {
                                content
                            };

                            // Perform the search
                            let matches = if case_sensitive {
                                search_content.contains(query)
                            } else {
                                search_content.to_lowercase().contains(&query.to_lowercase())
                            };

                            if matches {
                                found_any = true;
                                println!("=== {}/{} ===", topic_name.cyan(), insight_name.yellow());
                                
                                // Show matching lines
                                for line in search_content.lines() {
                                    let line_matches = if case_sensitive {
                                        line.contains(query)
                                    } else {
                                        line.to_lowercase().contains(&query.to_lowercase())
                                    };
                                    
                                    if line_matches {
                                        println!("{}", line);
                                    }
                                }
                                println!();
                            }
                        }
                    }
                }
            }
        }
    }

    if !found_any {
        println!("No matches found for: {}", query.yellow());
    }

    Ok(())
}

/// Get content of a specific insight
pub fn get_insight(topic: &str, name: &str, overview_only: bool) -> Result<()> {
    let insight = Insight::load(topic, name)?;
    
    if overview_only {
        println!("{}", insight.overview);
    } else {
        println!("---\n{}\n---\n\n{}", insight.overview, insight.details);
    }
    
    Ok(())
}

/// List insights in a topic or all topics
pub fn list_insights(topic_filter: Option<&str>, verbose: bool) -> Result<()> {
    let insights = get_insights(topic_filter)?;
    
    if insights.is_empty() {
        if let Some(topic) = topic_filter {
            println!("No insights found in topic: {}", topic.yellow());
        } else {
            println!("No insights found");
        }
        return Ok(());
    }

    for (topic, name) in insights {
        if verbose {
            if let Ok(insight) = Insight::load(&topic, &name) {
                println!("{}/{}: {}", 
                    topic.cyan(), 
                    name.yellow(), 
                    insight.overview.trim().replace('\n', " ")
                );
            }
        } else {
            println!("{}/{}", topic.cyan(), name.yellow());
        }
    }
    
    Ok(())
}

/// Update an existing insight
pub fn update_insight(
    topic: &str,
    name: &str,
    new_overview: Option<&str>,
    new_details: Option<&str>,
) -> Result<()> {
    let mut insight = Insight::load(topic, name)?;
    insight.update(new_overview, new_details)?;
    
    println!("{} Updated insight {}/{}", "✓".green(), topic.cyan(), name.yellow());
    Ok(())
}

/// Create a link from one insight to another topic
pub fn link_insight(
    src_topic: &str,
    src_name: &str,
    target_topic: &str,
    target_name: Option<&str>,
) -> Result<()> {
    let insights_root = get_insights_root()?;
    let target_name = target_name.unwrap_or(src_name);
    
    let src_path = insights_root.join(src_topic).join(format!("{}.insight.md", src_name));
    let target_dir = insights_root.join(target_topic);
    let target_path = target_dir.join(format!("{}.insight.md", target_name));

    // Check if source insight exists
    if !src_path.exists() {
        return Err(anyhow!("Source insight {}/{} not found", src_topic, src_name));
    }

    // Create target directory if it doesn't exist
    fs::create_dir_all(&target_dir)?;

    // Create the symbolic link
    std::os::unix::fs::symlink(&src_path, &target_path)?;
    
    println!("{} Created link: {}/{} -> {}/{}", 
        "✓".green(), 
        target_topic.cyan(), 
        target_name.yellow(),
        src_topic.cyan(), 
        src_name.yellow()
    );
    
    Ok(())
}

/// Delete an insight
pub fn delete_insight(topic: &str, name: &str, force: bool) -> Result<()> {
    let insight = Insight::load(topic, name)?;
    
    if !force {
        println!("Are you sure you want to delete {}/{}? [y/N]", 
            topic.cyan(), 
            name.yellow()
        );
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        if !input.trim().to_lowercase().starts_with('y') {
            println!("Deletion cancelled");
            return Ok(());
        }
    }
    
    insight.delete()?;
    
    println!("{} Deleted insight {}/{}", "✓".green(), topic.cyan(), name.yellow());
    Ok(())
}

/// List all available topics
pub fn list_topics() -> Result<()> {
    let topics = get_topics()?;
    
    if topics.is_empty() {
        println!("No topics found");
        return Ok(());
    }
    
    for topic in topics {
        println!("{}", topic.cyan());
    }
    
    Ok(())
} 