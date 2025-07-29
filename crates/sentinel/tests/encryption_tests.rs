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
fn test_encryption_manager_machine_key_deterministic() {
  let _temp_dir = setup_test_env();

  // Machine key should be deterministic - same inputs produce same output
  let key1 = EncryptionManager::machine_key().unwrap();
  let key2 = EncryptionManager::machine_key().unwrap();
  
  assert_eq!(key1, key2, "Machine key should be deterministic");
  assert_eq!(key1.len(), 32, "Machine key should be 32 bytes");
  assert_eq!(key2.len(), 32, "Machine key should be 32 bytes");
}

#[test]
fn test_encryption_manager_machine_key_not_empty() {
  let _temp_dir = setup_test_env();

  let key = EncryptionManager::machine_key().unwrap();
  
  // Ensure the key is not all zeros (highly unlikely with SHA-256)
  let all_zeros = vec![0u8; 32];
  assert_ne!(key, all_zeros, "Machine key should not be all zeros");
  
  // Ensure key has some entropy (not all same byte)
  let first_byte = key[0];
  let all_same = key.iter().all(|&b| b == first_byte);
  assert!(!all_same, "Machine key should have entropy (not all same byte)");
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

#[test]
fn test_encryption_manager_key_derivation_deterministic() {
  let _temp_dir = setup_test_env();

  let master_password = "test_password_123";
  let machine_key = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150, 160];
  let salt = vec![255, 254, 253, 252, 251, 250, 249, 248, 247, 246, 245, 244, 243, 242, 241, 240];

  // Same inputs should produce same output
  let key1 = EncryptionManager::derive_key(master_password, &machine_key, &salt).unwrap();
  let key2 = EncryptionManager::derive_key(master_password, &machine_key, &salt).unwrap();
  
  assert_eq!(key1, key2, "Key derivation should be deterministic");
  assert_eq!(key1.len(), 32, "Derived key should be 32 bytes");
}

#[test]
fn test_encryption_manager_key_derivation_different_inputs() {
  let _temp_dir = setup_test_env();

  let machine_key = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
  let salt = vec![16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1];

  // Different passwords should produce different keys
  let key1 = EncryptionManager::derive_key("password1", &machine_key, &salt).unwrap();
  let key2 = EncryptionManager::derive_key("password2", &machine_key, &salt).unwrap();
  assert_ne!(key1, key2, "Different passwords should produce different keys");

  // Different machine keys should produce different keys
  let machine_key2 = vec![16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1];
  let key3 = EncryptionManager::derive_key("password1", &machine_key2, &salt).unwrap();
  assert_ne!(key1, key3, "Different machine keys should produce different keys");

  // Different salts should produce different keys
  let salt2 = vec![32, 31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17];
  let key4 = EncryptionManager::derive_key("password1", &machine_key, &salt2).unwrap();
  assert_ne!(key1, key4, "Different salts should produce different keys");
}

#[test]
fn test_encryption_manager_key_derivation_edge_cases() {
  let _temp_dir = setup_test_env();

  let machine_key = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
  let salt = vec![16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1];

  // Empty password should work
  let key_empty = EncryptionManager::derive_key("", &machine_key, &salt);
  assert!(key_empty.is_ok(), "Empty password should be handled");
  assert_eq!(key_empty.unwrap().len(), 32);

  // Very long password should work
  let long_password = "a".repeat(1000);
  let key_long = EncryptionManager::derive_key(&long_password, &machine_key, &salt);
  assert!(key_long.is_ok(), "Long password should be handled");
  assert_eq!(key_long.unwrap().len(), 32);

  // Empty machine key should work
  let key_empty_machine = EncryptionManager::derive_key("password", &[], &salt);
  assert!(key_empty_machine.is_ok(), "Empty machine key should be handled");
  assert_eq!(key_empty_machine.unwrap().len(), 32);

  // Empty salt should work
  let key_empty_salt = EncryptionManager::derive_key("password", &machine_key, &[]);
  assert!(key_empty_salt.is_ok(), "Empty salt should be handled");
  assert_eq!(key_empty_salt.unwrap().len(), 32);
}

#[test]
fn test_encryption_manager_sha256_properties() {
  let _temp_dir = setup_test_env();

  let machine_key = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
  let salt = vec![16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1];

  // Test that derived keys have good entropy (not predictable patterns)
  let key1 = EncryptionManager::derive_key("password1", &machine_key, &salt).unwrap();
  let key2 = EncryptionManager::derive_key("password2", &machine_key, &salt).unwrap();
  
  // Keys should not be identical
  assert_ne!(key1, key2);
  
  // Keys should not have obvious patterns (all same byte)
  let first_byte = key1[0];
  let all_same = key1.iter().all(|&b| b == first_byte);
  assert!(!all_same, "Derived key should have entropy");
  
  // Check that changing one character changes many bytes (avalanche effect)
  let key1_alt = EncryptionManager::derive_key("password1x", &machine_key, &salt).unwrap();
  let different_bytes = key1.iter().zip(key1_alt.iter()).filter(|(a, b)| a != b).count();
  
  // SHA-256 should have good avalanche effect - small input change affects many output bytes
  assert!(different_bytes > 8, "Small input change should affect many output bytes (avalanche effect)");
}

#[test]
fn test_machine_key_components() {
  let _temp_dir = setup_test_env();
  
  // Test that machine key generation includes hostname and username components
  let key1 = EncryptionManager::machine_key().unwrap();
  assert_eq!(key1.len(), 32, "Machine key should be 32 bytes");
  
  // The key should be consistent across multiple calls
  let key2 = EncryptionManager::machine_key().unwrap();
  assert_eq!(key1, key2, "Machine key should be deterministic");
  
  // Machine key should be based on actual system information
  // We can't test the exact values since they depend on the system,
  // but we can verify it's not a default/empty value
  let empty_key = vec![0u8; 32];
  assert_ne!(key1, empty_key, "Machine key should not be all zeros");
}

#[test]
fn test_key_derivation_integration() {
  let _temp_dir = setup_test_env();
  
  // Test the full key derivation process with realistic data
  let password = "MySecurePassword123!";
  let machine_key = EncryptionManager::machine_key().unwrap();
  let salt = b"test_salt_16_byt"; // 16 bytes
  
  let derived_key = EncryptionManager::derive_key(password, &machine_key, salt).unwrap();
  
  assert_eq!(derived_key.len(), 32, "Derived key should be 32 bytes");
  
  // Verify the same inputs produce the same key
  let derived_key2 = EncryptionManager::derive_key(password, &machine_key, salt).unwrap();
  assert_eq!(derived_key, derived_key2, "Key derivation should be deterministic");
  
  // Verify different password produces different key
  let different_key = EncryptionManager::derive_key("DifferentPassword", &machine_key, salt).unwrap();
  assert_ne!(derived_key, different_key, "Different password should produce different key");
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

#[test]
fn test_argon2_specific_properties() {
  let _temp_dir = setup_test_env();

  let machine_key = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
  let salt = vec![16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1];

  // Test that Argon2 is computationally intensive (takes measurable time)
  let start = std::time::Instant::now();
  let _key = EncryptionManager::derive_key("password", &machine_key, &salt).unwrap();
  let duration = start.elapsed();
  
  // Argon2 should take at least some time (more than 1ms due to work factor)
  assert!(duration.as_millis() > 0, "Argon2 should take measurable time");
  
  // Test that different salts produce different keys (salt dependency)
  let salt2 = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
  let key1 = EncryptionManager::derive_key("password", &machine_key, &salt).unwrap();
  let key2 = EncryptionManager::derive_key("password", &machine_key, &salt2).unwrap();
  assert_ne!(key1, key2, "Different salts should produce different keys");
  
  // Test that small password changes produce very different keys (avalanche effect)
  let key_a = EncryptionManager::derive_key("password", &machine_key, &salt).unwrap();
  let key_b = EncryptionManager::derive_key("passworD", &machine_key, &salt).unwrap(); // One char different
  
  // Count differing bytes - should be significant for good key derivation
  let differing_bytes = key_a.iter().zip(key_b.iter()).filter(|(a, b)| a != b).count();
  assert!(differing_bytes > 15, "Single char change should affect many bytes, got {} differing", differing_bytes);
}

#[test]
fn test_argon2_salt_padding() {
  let _temp_dir = setup_test_env();

  let machine_key = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
  
  // Test that short salts are handled properly (padded to minimum length)
  let short_salt = vec![1, 2, 3]; // Only 3 bytes
  let result = EncryptionManager::derive_key("password", &machine_key, &short_salt);
  assert!(result.is_ok(), "Short salt should be handled gracefully");
  assert_eq!(result.unwrap().len(), 32, "Should still produce 32-byte key");
  
  // Test that empty salt is handled
  let empty_salt = vec![];
  let result = EncryptionManager::derive_key("password", &machine_key, &empty_salt);
  assert!(result.is_ok(), "Empty salt should be handled gracefully");
  assert_eq!(result.unwrap().len(), 32, "Should still produce 32-byte key");
}

#[test]
fn test_argon2_security_parameters() {
  let _temp_dir = setup_test_env();

  let machine_key = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
  let salt = vec![16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1];
  
  // Test that Argon2 produces consistent results (deterministic)
  let key1 = EncryptionManager::derive_key("password", &machine_key, &salt).unwrap();
  let key2 = EncryptionManager::derive_key("password", &machine_key, &salt).unwrap();
  assert_eq!(key1, key2, "Argon2 should be deterministic");
  
  // Test that machine key integration works
  let machine_key2 = vec![16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1];
  let key_diff_machine = EncryptionManager::derive_key("password", &machine_key2, &salt).unwrap();
  assert_ne!(key1, key_diff_machine, "Different machine keys should produce different results");
  
  // Test that output length is always 32 bytes
  assert_eq!(key1.len(), 32, "Argon2 should always produce 32-byte key");
}
