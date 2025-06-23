use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::RwLock;

use crate::encryption::{CredentialCache, EncryptedBlob, EncryptionManager};

/// Request message sent to the daemon
#[derive(Debug, Serialize, Deserialize)]
pub enum DaemonRequest {
    GetCredential { service: String, key: String },
    StoreCredential { service: String, key: String, value: String },
    DeleteCredential { service: String, key: String },
    ListCredentials { service: Option<String> },
    Shutdown,
}

/// Response message from the daemon
#[derive(Debug, Serialize, Deserialize)]
pub enum DaemonResponse {
    Success { data: Option<String> },
    Error { message: String },
    CredentialList { credentials: Vec<String> },
}

/// Daemon client for communicating with the background service
pub struct DaemonClient {
    socket_path: PathBuf,
}

impl DaemonClient {
    pub fn new() -> Self {
        Self {
            socket_path: Self::get_socket_path(),
        }
    }

    /// Check if daemon is running
    pub async fn is_running(&self) -> bool {
        tokio::net::UnixStream::connect(&self.socket_path).await.is_ok()
    }

    /// Get a credential from the daemon
    pub async fn get_credential(&self, service: &str, key: &str) -> Result<String> {
        let request = DaemonRequest::GetCredential {
            service: service.to_string(),
            key: key.to_string(),
        };
        
        let response = self.send_request(request).await?;
        
        match response {
            DaemonResponse::Success { data: Some(value) } => Ok(value),
            DaemonResponse::Success { data: None } => Err(anyhow!("Credential not found")),
            DaemonResponse::Error { message } => Err(anyhow!("Daemon error: {}", message)),
            _ => Err(anyhow!("Unexpected response from daemon")),
        }
    }

    /// Store a credential in the daemon
    pub async fn store_credential(&self, service: &str, key: &str, value: &str) -> Result<()> {
        let request = DaemonRequest::StoreCredential {
            service: service.to_string(),
            key: key.to_string(),
            value: value.to_string(),
        };
        
        let response = self.send_request(request).await?;
        
        match response {
            DaemonResponse::Success { .. } => Ok(()),
            DaemonResponse::Error { message } => Err(anyhow!("Daemon error: {}", message)),
            _ => Err(anyhow!("Unexpected response from daemon")),
        }
    }

    /// Delete a credential from the daemon
    pub async fn delete_credential(&self, service: &str, key: &str) -> Result<()> {
        let request = DaemonRequest::DeleteCredential {
            service: service.to_string(),
            key: key.to_string(),
        };
        
        let response = self.send_request(request).await?;
        
        match response {
            DaemonResponse::Success { .. } => Ok(()),
            DaemonResponse::Error { message } => Err(anyhow!("Daemon error: {}", message)),
            _ => Err(anyhow!("Unexpected response from daemon")),
        }
    }

    /// Send a request to the daemon
    async fn send_request(&self, request: DaemonRequest) -> Result<DaemonResponse> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .await
            .map_err(|_| anyhow!("Failed to connect to daemon. Is it running?"))?;
        
        // Send request
        let request_json = serde_json::to_string(&request)?;
        stream.write_all(request_json.as_bytes()).await?;
        stream.write_all(b"\n").await?;
        
        // Read response
        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await?;
        
        let response: DaemonResponse = serde_json::from_str(response_line.trim())?;
        Ok(response)
    }

    fn get_socket_path() -> PathBuf {
        let mut path = dirs::runtime_dir()
            .or_else(|| dirs::cache_dir())
            .unwrap_or_else(|| std::env::temp_dir());
        path.push("sentinel-daemon.sock");
        path
    }
}

/// Background daemon for credential management
pub struct SentinelDaemon {
    cache: Arc<RwLock<CredentialCache>>,
    socket_path: PathBuf,
    storage_path: PathBuf,
}

impl SentinelDaemon {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(CredentialCache::new())),
            socket_path: DaemonClient::get_socket_path(),
            storage_path: Self::get_storage_path(),
        }
    }

    /// Start the daemon with the given master password
    pub async fn start(&self, master_password: &str) -> Result<()> {
        // Load existing credentials if they exist
        if let Err(e) = self.load_credentials(master_password).await {
            bentley::warn(&format!("Failed to load existing credentials: {}", e));
            bentley::info("Starting with empty credential cache");
        }

        // Remove existing socket file
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)?;
        }

        // Create Unix socket listener
        let listener = UnixListener::bind(&self.socket_path)?;
        bentley::success("Sentinel daemon started successfully");

        // Handle incoming connections
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let cache = Arc::clone(&self.cache);
                    let storage_path = self.storage_path.clone();
                    
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client(stream, cache, storage_path).await {
                            bentley::warn(&format!("Client handler error: {}", e));
                        }
                    });
                }
                Err(e) => {
                    bentley::error(&format!("Failed to accept connection: {}", e));
                }
            }
        }
    }

    /// Handle a client connection
    async fn handle_client(
        stream: UnixStream,
        cache: Arc<RwLock<CredentialCache>>,
        _storage_path: PathBuf,
    ) -> Result<()> {
        let mut reader = BufReader::new(stream);
        let mut request_line = String::new();
        
        reader.read_line(&mut request_line).await?;
        let request: DaemonRequest = serde_json::from_str(request_line.trim())?;
        
        let response = match request {
            DaemonRequest::GetCredential { service, key } => {
                let cache_read = cache.read().await;
                match cache_read.get(&service, &key) {
                    Some(value) => DaemonResponse::Success {
                        data: Some(value.clone()),
                    },
                    None => DaemonResponse::Error {
                        message: format!("Credential not found: {}/{}", service, key),
                    },
                }
            }
            DaemonRequest::StoreCredential { service, key, value } => {
                let mut cache_write = cache.write().await;
                cache_write.insert(&service, &key, value);
                
                // Save to disk (we'll need the master password for this)
                // For now, just acknowledge the store
                DaemonResponse::Success { data: None }
            }
            DaemonRequest::DeleteCredential { service, key } => {
                let mut cache_write = cache.write().await;
                cache_write.remove(&service, &key);
                DaemonResponse::Success { data: None }
            }
            DaemonRequest::ListCredentials { service: _ } => {
                // TODO: Implement credential listing
                DaemonResponse::CredentialList {
                    credentials: vec![],
                }
            }
            DaemonRequest::Shutdown => {
                // TODO: Implement graceful shutdown
                DaemonResponse::Success { data: None }
            }
        };
        
        let response_json = serde_json::to_string(&response)?;
        let mut stream = reader.into_inner();
        stream.write_all(response_json.as_bytes()).await?;
        stream.write_all(b"\n").await?;
        
        Ok(())
    }

    /// Load credentials from encrypted storage
    async fn load_credentials(&self, master_password: &str) -> Result<()> {
        // Try to load from encrypted storage
        if self.storage_path.exists() {
            match std::fs::read(&self.storage_path) {
                Ok(encrypted_data) => {
                    match serde_json::from_slice::<EncryptedBlob>(&encrypted_data) {
                        Ok(blob) => {
                            match EncryptionManager::decrypt_credentials(&blob, master_password) {
                                Ok(credentials) => {
                                    let mut cache = self.cache.write().await;
                                    for (full_key, value) in credentials {
                                        // Parse the full_key back to service and key
                                        if let Some(underscore_pos) = full_key.find('_') {
                                            let service = &full_key[..underscore_pos];
                                            let key = &full_key[underscore_pos + 1..];
                                            cache.insert(service, key, value);
                                        }
                                    }
                                    bentley::success("Loaded credentials from encrypted storage");
                                    return Ok(());
                                }
                                Err(e) => {
                                    bentley::warn(&format!("Failed to decrypt credentials: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            bentley::warn(&format!("Failed to parse encrypted credentials: {}", e));
                        }
                    }
                }
                Err(e) => {
                    bentley::warn(&format!("Failed to read encrypted credentials: {}", e));
                }
            }
        } else {
            bentley::info("No encrypted credentials found, starting with empty cache");
        }
        
        Ok(())
    }

    /// Save credentials to encrypted storage
    pub async fn save_credentials(&self, master_password: &str) -> Result<()> {
        let cache = self.cache.read().await;
        let credentials = cache.to_map().clone();
        
        let blob = EncryptionManager::encrypt_credentials(&credentials, master_password)?;
        let encrypted_data = serde_json::to_vec(&blob)?;
        
        // Ensure directory exists
        if let Some(parent) = self.storage_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        std::fs::write(&self.storage_path, encrypted_data)?;
        bentley::success("Saved credentials to encrypted storage");
        Ok(())
    }

    fn get_storage_path() -> PathBuf {
        let mut path = dirs::config_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| std::env::temp_dir()));
        path.push("sentinel");
        path.push("credentials.enc");
        path
    }
}

/// Authentication manager for getting master passwords
pub struct AuthManager;

impl AuthManager {
    /// Get master password with fallback priority
    pub async fn get_master_password() -> Result<String> {
        // Use a simple fixed password for now to avoid keychain complexity
        // In production, this could prompt the user or use environment variables
        Ok("kernelle-master-key".to_string())
    }
} 