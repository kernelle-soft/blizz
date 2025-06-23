use anyhow::{anyhow, Result};
use sentinel::{Sentinel, services};

pub async fn get_github_token() -> Result<String> {
    let sentinel = Sentinel::new();
    let github_config = services::github();
    
    // Check if credentials exist, set them up if not
    let missing = sentinel.verify_service_credentials(&github_config)?;
    if !missing.is_empty() {
        sentinel.setup_service(&github_config)?;
    }
    
    // Get the token
    sentinel.get_credential("github", "token")
}

pub async fn get_gitlab_token() -> Result<String> {
    let sentinel = Sentinel::new();
    let gitlab_config = services::gitlab();
    
    // Check if credentials exist, set them up if not
    let missing = sentinel.verify_service_credentials(&gitlab_config)?;
    if !missing.is_empty() {
        sentinel.setup_service(&gitlab_config)?;
    }
    
    // Get the token
    sentinel.get_credential("gitlab", "token")
} 