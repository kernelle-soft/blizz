use anyhow::Result;
use sentinel::Sentinel;
use std::env;

pub async fn get_github_token() -> Result<String> {
    // In test mode, return a fake token to avoid credential prompts
    if env::var("JERROD_TEST_MODE").is_ok() {
        return Ok("fake_github_token_for_testing".to_string());
    }
    
    let sentinel = Sentinel::new();
    sentinel.get_credential("github", "token")
}

pub async fn get_gitlab_token() -> Result<String> {
    // In test mode, return a fake token to avoid credential prompts  
    if env::var("JERROD_TEST_MODE").is_ok() {
        return Ok("fake_gitlab_token_for_testing".to_string());
    }
    
    let sentinel = Sentinel::new();
    sentinel.get_credential("gitlab", "token")
} 