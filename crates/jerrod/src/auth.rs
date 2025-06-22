use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use aes_gcm::{Aes256Gcm, Key, Nonce, aead::{Aead, KeyInit, OsRng}};
use base64;
use rand::RngCore;

#[derive(Debug, Serialize, Deserialize)]
struct EncryptedCredentialStore {
    credentials: HashMap<String, HashMap<String, String>>, // service -> key -> encrypted_value
}

impl EncryptedCredentialStore {
    fn new() -> Self {
        Self {
            credentials: HashMap::new(),
        }
    }

    fn get_encrypted(&self, service: &str, key: &str) -> Option<&String> {
        self.credentials.get(service)?.get(key)
    }

    fn set_encrypted(&mut self, service: &str, key: &str, encrypted_value: String) {
        self.credentials
            .entry(service.to_string())
            .or_insert_with(HashMap::new)
            .insert(key.to_string(), encrypted_value);
    }

    fn load_from_file(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }
        
        let content = fs::read_to_string(path)?;
        let store: EncryptedCredentialStore = serde_json::from_str(&content)?;
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

struct CryptoManager {
    key_path: PathBuf,
}

impl CryptoManager {
    fn new() -> Self {
        let mut base_path = dirs::home_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap());
        base_path.push(".kernelle");
        base_path.push("sentinel");
        
        let mut key_path = base_path;
        key_path.push("master.key");
        
        Self { key_path }
    }

    fn key_exists(&self) -> bool {
        self.key_path.exists()
    }

    fn generate_key(&self) -> Result<()> {
        bentley::info("🔐 Generating AES encryption key for secure credential storage...");
        
        let mut key = [0u8; 32]; // 256-bit key for AES-256
        OsRng.fill_bytes(&mut key);

        // Create directory if it doesn't exist
        if let Some(parent) = self.key_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Save key as base64
        let key_b64 = base64::encode(key);
        fs::write(&self.key_path, key_b64)?;

        // Set restrictive permissions on key file
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&self.key_path)?.permissions();
            perms.set_mode(0o600); // Owner read/write only
            fs::set_permissions(&self.key_path, perms)?;
        }

        bentley::success("🔑 AES encryption key generated and stored securely");
        Ok(())
    }

    fn load_key(&self) -> Result<[u8; 32]> {
        let key_b64 = fs::read_to_string(&self.key_path)?;
        let key_bytes = base64::decode(key_b64.trim())?;
        
        if key_bytes.len() != 32 {
            return Err(anyhow!("Invalid key length"));
        }
        
        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes);
        Ok(key)
    }

    fn encrypt_value(&self, value: &str) -> Result<String> {
        let key_bytes = self.load_key()?;
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12]; // 96-bit nonce for GCM
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Encrypt the value
        let ciphertext = cipher.encrypt(nonce, value.as_bytes())
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;
        
        // Combine nonce + ciphertext and encode as base64
        let mut combined = nonce_bytes.to_vec();
        combined.extend_from_slice(&ciphertext);
        
        Ok(base64::encode(combined))
    }

    fn decrypt_value(&self, encrypted_value: &str) -> Result<String> {
        let key_bytes = self.load_key()?;
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        
        // Decode from base64
        let combined = base64::decode(encrypted_value)?;
        
        if combined.len() < 12 {
            return Err(anyhow!("Invalid encrypted data"));
        }
        
        // Split nonce and ciphertext
        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        // Decrypt
        let plaintext = cipher.decrypt(nonce, ciphertext)
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;
        
        Ok(String::from_utf8(plaintext)?)
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

async fn ensure_crypto_setup() -> Result<CryptoManager> {
    let crypto = CryptoManager::new();
    
    if !crypto.key_exists() {
        println!("\n🔐 No encryption key found for secure credential storage.");
        println!("Would you like to generate a new AES encryption key? (y/N)");
        print!("> ");
        
        use std::io::{self, Write};
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let response = input.trim().to_lowercase();
        
        if response == "y" || response == "yes" {
            crypto.generate_key()?;
        } else {
            return Err(anyhow!("Encryption key required for secure credential storage"));
        }
    }
    
    Ok(crypto)
}

async fn prompt_for_token(service: &str, token_type: &str, crypto: &CryptoManager) -> Result<String> {
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
    
    // Encrypt and store the token
    let encrypted_token = crypto.encrypt_value(&token)?;
    
    let mut store = EncryptedCredentialStore::load_from_file(&get_credentials_path())?;
    store.set_encrypted(service, "token", encrypted_token);
    store.save_to_file(&get_credentials_path())?;
    
    bentley::success("🔐 Token encrypted and stored securely!");
    Ok(token)
}

/// Get GitHub token with JIT decryption - secret only exists in memory during this call
pub async fn get_github_token() -> Result<String> {
    // 0. Ensure encryption is set up first
    let crypto = ensure_crypto_setup().await?;
    
    // 1. Check encrypted credential store first
    let credentials_path = get_credentials_path();
    let store = EncryptedCredentialStore::load_from_file(&credentials_path)?;
    
    if let Some(encrypted_token) = store.get_encrypted("github", "token") {
        // JIT decrypt - token only exists in memory for this operation
        let decrypted_token = crypto.decrypt_value(encrypted_token)?;
        return Ok(decrypted_token);
        // decrypted_token is dropped here - no longer in memory
    }
    
    // 2. Check environment variables as fallback
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            // Encrypt and store it for future use
            let encrypted_token = crypto.encrypt_value(&token)?;
            
            let mut store = EncryptedCredentialStore::load_from_file(&credentials_path)?;
            store.set_encrypted("github", "token", encrypted_token);
            store.save_to_file(&credentials_path)?;
            
            return Ok(token);
        }
    }
    
    // 3. Prompt user for token (crypto is already set up)
    prompt_for_token("github", "GitHub Personal Access Token", &crypto).await
} 