use anyhow::{anyhow, Result};
use crate::session::SessionManager;

pub async fn handle() -> Result<()> {
    let session_manager = SessionManager::new()?;
    
    let session = session_manager.load_session()?
        .ok_or_else(|| anyhow!("No active review session. Use 'jerrod start' to begin."))?;
    
    if let Some(thread) = session.peek_next_thread() {
        bentley::announce("Next Thread");
        println!("🧵 Discussion #{}", thread.id);
        
        println!("Resolved: {}", thread.resolved);
        println!("Resolvable: {}", thread.resolvable);
        
        if let Some(file_path) = &thread.file_path {
            println!("File: {}", file_path);
            if let Some(line) = thread.line_number {
                println!("Line: {}", line);
            }
        }
        
        for (i, note) in thread.notes.iter().enumerate() {
            println!("\n--- Comment {} by {} ---", i + 1, note.author.display_name);
            println!("{}", note.body);
            println!("Posted: {}", note.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
        }
    } else {
        bentley::info("No threads remaining in queue!");
    }
    
    Ok(())
} 