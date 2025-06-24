use anyhow::Result;
use sentinel::{CredentialProvider, Sentinel};
use std::sync::{Mutex, OnceLock};

// Type alias for the credential provider factory function
type ProviderFactory = Box<dyn Fn() -> Box<dyn CredentialProvider> + Send + Sync>;

// Global factory function holder
static PROVIDER_FACTORY: OnceLock<Mutex<ProviderFactory>> = OnceLock::new();

// Default factory that creates real Sentinel instances
fn default_provider_factory() -> Box<dyn CredentialProvider> {
  Box::new(Sentinel::new())
}

// Get the current factory function
fn get_provider() -> Box<dyn CredentialProvider> {
  let factory = PROVIDER_FACTORY.get_or_init(|| Mutex::new(Box::new(default_provider_factory)));

  let guard = factory.lock().unwrap();
  guard()
}

// Register a custom provider factory (for testing)
pub fn register_provider_factory<F>(factory: F)
where
  F: Fn() -> Box<dyn CredentialProvider> + Send + Sync + 'static,
{
  let provider_factory =
    PROVIDER_FACTORY.get_or_init(|| Mutex::new(Box::new(default_provider_factory)));

  let mut guard = provider_factory.lock().unwrap();
  *guard = Box::new(factory);
}

// Reset to default factory (useful for test cleanup)
pub fn reset_provider_factory() {
  if let Some(factory) = PROVIDER_FACTORY.get() {
    let mut guard = factory.lock().unwrap();
    *guard = Box::new(default_provider_factory);
  }
}

pub async fn get_github_token() -> Result<String> {
  let provider = get_provider();
  provider.get_credential("github", "token")
}

pub async fn get_gitlab_token() -> Result<String> {
  let provider = get_provider();
  provider.get_credential("gitlab", "token")
}
