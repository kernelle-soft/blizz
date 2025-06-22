use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;


#[derive(Debug, Serialize, Deserialize)]
struct CredentialStore {
    credentials: HashMap<String, HashMap<String, String>>,
}

impl CredentialStore {
    fn new() -> Self {
        Self {
            credentials: HashMap::new(),
        }
    }

    fn get(&self, service: &str, key: &str) -> Option<&String> {
        self.credentials.get(service)?.get(key)
    }

    fn set(&mut self, service: &str, key: &str, value: String) {
        self.credentials
            .entry(service.to_string())
            .or_insert_with(HashMap::new)
            .insert(key.to_string(), value);
    }

    fn load_from_file(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }
        
        let content = fs::read_to_string(path)?;
        let store: CredentialStore = serde_json::from_str(&content)?;
        Ok(store)
    }

    fn save_to_file(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

fn get_credentials_path() -> PathBuf {
    let mut path = dirs::home_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    path.push(".kernelle");
    path.push("sentinel");
    path.push("credentials.json");
    path
}

async fn ensure_daemon_running() -> Result<()> {
    // For now, we'll just use the file-based approach
    // Later we can add a proper daemon
    Ok(())
}

async fn prompt_for_token(service: &str, token_type: &str) -> Result<String> {
    println!("\n🔑 GitHub token needed for {}", service);
    println!("Please enter your {} token:", token_type);
    print!("> ");
    
    use std::io::{self, Write};
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let token = input.trim().to_string();
    
    if token.is_empty() {
        return Err(anyhow!("Token cannot be empty"));
    }
    
    // Store the token for future use
    let mut store = CredentialStore::load_from_file(&get_credentials_path())?;
    store.set(service, "token", token.clone());
    store.save_to_file(&get_credentials_path())?;
    
    bentley::success("Token stored successfully!");
    Ok(token)
}

/// Get GitHub token with lazy loading and prompting
pub async fn get_github_token() -> Result<String> {
    ensure_daemon_running().await?;
    
    // 1. Check local credential store first
    let credentials_path = get_credentials_path();
    let store = CredentialStore::load_from_file(&credentials_path)?;
    
    if let Some(token) = store.get("github", "token") {
        return Ok(token.clone());
    }
    
    // 2. Check environment variables as fallback
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            // Store it for future use
            let mut store = CredentialStore::load_from_file(&credentials_path)?;
            store.set("github", "token", token.clone());
            store.save_to_file(&credentials_path)?;
            return Ok(token);
        }
    }
    
    // 3. Prompt user for token
    prompt_for_token("github", "GitHub Personal Access Token").await
} 