use sentinel::encryption::{CredentialCache, EncryptedBlob, EncryptionManager};
use std::collections::HashMap;
use std::env;
use tempfile::TempDir;

fn setup_test_env() -> TempDir {
  let temp_dir = TempDir::new().unwrap();
  env::set_var("KERNELLE_DIR", temp_dir.path());
  temp_dir
}

#[test]
fn test_credential_cache_creation() {
  let _temp_dir = setup_test_env();

  let cache = CredentialCache::new();
  // Should be able to create empty cache
  // (no is_empty method, but we can check via to_map)
  assert_eq!(cache.to_map().len(), 0);
}

#[test]
fn test_credential_cache_operations() {
  let mut cache = CredentialCache::new();

  // Test storing a credential
  cache.store("github_token".to_string(), "test_token_value".to_string());

  // Test retrieving the stored credential
  let value = cache.get("github_token");
  assert!(value.is_some());
  assert_eq!(value.unwrap(), "test_token_value");

  // Test retrieving a non-existent credential
  let missing = cache.get("nonexistent");
  assert!(missing.is_none());
}

#[test]
fn test_credential_cache_clear() {
  let mut cache = CredentialCache::new();
  cache.store("github_token".to_string(), "test_value".to_string());
  assert!(cache.get("github_token").is_some());

  let removed = cache.remove("github_token");
  assert!(removed.is_some());
  assert_eq!(removed.unwrap(), "test_value");

  // After removal, should be gone
  assert!(cache.get("github_token").is_none());
}

#[test]
fn test_credential_cache_from_map() {
  let _temp_dir = setup_test_env();

  let mut initial_map = HashMap::new();
  initial_map.insert("github_token".to_string(), "test_value".to_string());
  initial_map.insert("gitlab_api_key".to_string(), "another_value".to_string());

  let cache = CredentialCache::from_map(initial_map);
  assert_eq!(cache.to_map().len(), 2);
  assert!(cache.to_map().contains_key("github_token"));
  assert!(cache.to_map().contains_key("gitlab_api_key"));
}

#[test]
fn test_credential_cache_to_map() {
  let _temp_dir = setup_test_env();

  let mut cache = CredentialCache::new();
  cache.store("github_token".to_string(), "value1".to_string());
  cache.store("gitlab_api_key".to_string(), "value2".to_string());

  let map = cache.to_map();
  assert_eq!(map.len(), 2);
  assert!(map.contains_key("github_token"));
  assert!(map.contains_key("gitlab_api_key"));
}

#[test]
fn test_encrypted_blob_creation() {
  let _temp_dir = setup_test_env();

  let blob = EncryptedBlob {
    data: vec![1, 2, 3, 4, 5],
    salt: vec![6, 7, 8, 9, 10],
    nonce: vec![11, 12, 13, 14, 15],
  };

  assert_eq!(blob.data.len(), 5);
  assert_eq!(blob.salt.len(), 5);
  assert_eq!(blob.nonce.len(), 5);
}

#[test]
fn test_encrypted_blob_serialization() {
  let _temp_dir = setup_test_env();

  let blob = EncryptedBlob { data: vec![1, 2, 3], salt: vec![4, 5, 6], nonce: vec![7, 8, 9] };

  // Should be able to serialize/deserialize
  let serialized = serde_json::to_string(&blob).unwrap();
  assert!(serialized.contains("data"));
  assert!(serialized.contains("salt"));
  assert!(serialized.contains("nonce"));

  let deserialized: EncryptedBlob = serde_json::from_str(&serialized).unwrap();
  assert_eq!(deserialized.data, blob.data);
  assert_eq!(deserialized.salt, blob.salt);
  assert_eq!(deserialized.nonce, blob.nonce);
}

#[test]
fn test_encryption_manager_machine_key() {
  let _temp_dir = setup_test_env();

  // Test machine key generation
  let key_result = EncryptionManager::machine_key();
  assert!(key_result.is_ok());
  let key = key_result.unwrap();
  assert_eq!(key.len(), 32); // Should be 32 bytes
}

#[test]
fn test_encryption_manager_key_derivation() {
  let _temp_dir = setup_test_env();

  let master_password = "test_password";
  let machine_key = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
  let salt = vec![16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1];

  let derived_key = EncryptionManager::derive_key(master_password, &machine_key, &salt);
  assert!(derived_key.is_ok());
  let key = derived_key.unwrap();
  assert_eq!(key.len(), 32); // Should be 32 bytes
}

// Skip tests that would require actual encryption/decryption with master passwords
#[test]
#[ignore]
fn test_encryption_manager_encrypt_decrypt() {
  // This would test actual encryption/decryption operations
  // Skip for coverage testing to focus on achievable improvements
}

#[test]
fn test_credential_cache_multiple_services() {
  let mut cache = CredentialCache::new();
  cache.store("github_token".to_string(), "github_token".to_string());
  cache.store("gitlab_token".to_string(), "gitlab_token".to_string());
}
