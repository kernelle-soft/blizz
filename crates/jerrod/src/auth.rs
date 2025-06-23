use anyhow::{anyhow, Result};
use sentinel::Sentinel;
use std::io::{self, Write};

pub async fn get_github_token() -> Result<String> {
    let sentinel = Sentinel::new();
    
    // Try to get existing token
    match sentinel.get_credential("github", "token") {
        Ok(token) => {
            bentley::info("🔑 Using existing GitHub token");
            Ok(token)
        }
        Err(_) => {
            // No token found, prompt for one
            bentley::info("🔑 No GitHub token found. Please enter your GitHub personal access token:");
            print!("> ");
            io::stdout().flush()?;
            
            let token = rpassword::read_password()?;
            
            if token.trim().is_empty() {
                return Err(anyhow!("GitHub token cannot be empty"));
            }
            
            // Store the token
            sentinel.store_credential("github", "token", token.trim())?;
            bentley::success("🔑 GitHub token stored securely");
            
            Ok(token.trim().to_string())
        }
    }
}

pub async fn get_gitlab_token() -> Result<String> {
    let sentinel = Sentinel::new();
    
    // Try to get existing token
    match sentinel.get_credential("gitlab", "token") {
        Ok(token) => {
            bentley::info("🔑 Using existing GitLab token");
            Ok(token)
        }
        Err(_) => {
            // No token found, prompt for one
            bentley::info("🔑 No GitLab token found. Please enter your GitLab personal access token:");
            print!("> ");
            io::stdout().flush()?;
            
            let token = rpassword::read_password()?;
            
            if token.trim().is_empty() {
                return Err(anyhow!("GitLab token cannot be empty"));
            }
            
            // Store the token
            sentinel.store_credential("gitlab", "token", token.trim())?;
            bentley::success("🔑 GitLab token stored securely");
            
            Ok(token.trim().to_string())
        }
    }
} 