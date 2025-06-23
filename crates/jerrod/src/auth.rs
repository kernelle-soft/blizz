use anyhow::Result;
use sentinel::Sentinel;

pub async fn get_github_token() -> Result<String> {
    let sentinel = Sentinel::new();
    sentinel.get_credential("github", "token")
}

pub async fn get_gitlab_token() -> Result<String> {
    let sentinel = Sentinel::new();
    sentinel.get_credential("gitlab", "token")
} 