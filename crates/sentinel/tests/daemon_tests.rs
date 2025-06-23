use sentinel::daemon::{SentinelDaemon, DaemonClient, DaemonRequest, DaemonResponse};
use std::env;
use tempfile::TempDir;

fn setup_test_env() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    env::set_var("KERNELLE_DIR", temp_dir.path());
    temp_dir
}

#[test]
fn test_daemon_request_serialization() {
    let _temp_dir = setup_test_env();
    
    let request = DaemonRequest::GetCredential {
        service: "github".to_string(),
        key: "token".to_string(),
    };
    
    let serialized = serde_json::to_string(&request).unwrap();
    assert!(serialized.contains("GetCredential"));
    assert!(serialized.contains("github"));
    assert!(serialized.contains("token"));
}

#[test]
fn test_daemon_response_serialization() {
    let _temp_dir = setup_test_env();
    
    let response = DaemonResponse::Success {
        data: Some("test_token".to_string()),
    };
    
    let serialized = serde_json::to_string(&response).unwrap();
    assert!(serialized.contains("Success"));
    assert!(serialized.contains("test_token"));
}

#[test]
fn test_daemon_client_creation() {
    let _temp_dir = setup_test_env();
    
    let client = DaemonClient::new();
    // Should be able to create client without errors
    // (actual connection tests would require running daemon)
}

#[test]
fn test_daemon_creation() {
    let _temp_dir = setup_test_env();
    
    let daemon = SentinelDaemon::new();
    // Should be able to create daemon without errors
    // (actual daemon start would require background process)
}

#[tokio::test]
async fn test_daemon_client_is_running_false() {
    let _temp_dir = setup_test_env();
    
    let client = DaemonClient::new();
    // Should return false when daemon is not running
    let is_running = client.is_running().await;
    assert!(!is_running);
}

#[test]
fn test_daemon_request_types() {
    let _temp_dir = setup_test_env();
    
    // Test different request types can be created
    let get_request = DaemonRequest::GetCredential {
        service: "test".to_string(),
        key: "key".to_string(),
    };
    
    let store_request = DaemonRequest::StoreCredential {
        service: "test".to_string(),
        key: "key".to_string(),
        value: "value".to_string(),
    };
    
    let delete_request = DaemonRequest::DeleteCredential {
        service: "test".to_string(),
        key: "key".to_string(),
    };
    
    let list_request = DaemonRequest::ListCredentials {
        service: Some("test".to_string()),
    };
    
    let shutdown_request = DaemonRequest::Shutdown;
    
    // Should be able to serialize all types
    assert!(serde_json::to_string(&get_request).is_ok());
    assert!(serde_json::to_string(&store_request).is_ok());
    assert!(serde_json::to_string(&delete_request).is_ok());
    assert!(serde_json::to_string(&list_request).is_ok());
    assert!(serde_json::to_string(&shutdown_request).is_ok());
}

#[test]
fn test_daemon_response_types() {
    let _temp_dir = setup_test_env();
    
    // Test different response types
    let success_response = DaemonResponse::Success {
        data: Some("value".to_string()),
    };
    
    let error_response = DaemonResponse::Error {
        message: "Test error".to_string(),
    };
    
    let list_response = DaemonResponse::CredentialList {
        credentials: vec!["cred1".to_string(), "cred2".to_string()],
    };
    
    // Should be able to serialize all types
    assert!(serde_json::to_string(&success_response).is_ok());
    assert!(serde_json::to_string(&error_response).is_ok());
    assert!(serde_json::to_string(&list_response).is_ok());
}

// Skip tests that would require actual daemon process
#[tokio::test]
#[ignore]
async fn test_daemon_start_stop() {
    // This would start an actual daemon process
    // Skip for coverage testing to focus on achievable improvements
}

#[tokio::test] 
#[ignore]
async fn test_daemon_client_operations() {
    // This would test actual client-daemon communication
    // Skip for coverage testing to focus on achievable improvements
} 