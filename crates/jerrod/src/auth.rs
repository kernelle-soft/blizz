use anyhow::Result;
use sentinel::Sentinel;
use std::sync::{Mutex, OnceLock};

// Type alias for the sentinel factory function
type SentinelFactory = Box<dyn Fn() -> Box<dyn SentinelTrait> + Send + Sync>;

// Trait that defines the interface we need from Sentinel
pub trait SentinelTrait {
  fn get_credential(&self, service: &str, key: &str) -> Result<String>;
}

// Implement the trait for the real Sentinel
impl SentinelTrait for Sentinel {
  fn get_credential(&self, service: &str, key: &str) -> Result<String> {
    self.get_credential(service, key)
  }
}

// Global factory function holder
static SENTINEL_FACTORY: OnceLock<Mutex<SentinelFactory>> = OnceLock::new();

// Default factory that creates real Sentinel instances
fn default_sentinel_factory() -> Box<dyn SentinelTrait> {
  Box::new(Sentinel::new())
}

// Get the current factory function
fn get_sentinel_factory() -> Box<dyn SentinelTrait> {
  let factory = SENTINEL_FACTORY.get_or_init(|| Mutex::new(Box::new(default_sentinel_factory)));

  let guard = factory.lock().unwrap();
  guard()
}

// Register a custom sentinel factory (for testing)
pub fn register_sentinel_factory<F>(factory: F)
where
  F: Fn() -> Box<dyn SentinelTrait> + Send + Sync + 'static,
{
  let sentinel_factory =
    SENTINEL_FACTORY.get_or_init(|| Mutex::new(Box::new(default_sentinel_factory)));

  let mut guard = sentinel_factory.lock().unwrap();
  *guard = Box::new(factory);
}

// Reset to default factory (useful for test cleanup)
pub fn reset_sentinel_factory() {
  if let Some(factory) = SENTINEL_FACTORY.get() {
    let mut guard = factory.lock().unwrap();
    *guard = Box::new(default_sentinel_factory);
  }
}

pub async fn get_github_token() -> Result<String> {
  let sentinel = get_sentinel_factory();
  sentinel.get_credential("github", "token")
}

pub async fn get_gitlab_token() -> Result<String> {
  let sentinel = get_sentinel_factory();
  sentinel.get_credential("gitlab", "token")
}
