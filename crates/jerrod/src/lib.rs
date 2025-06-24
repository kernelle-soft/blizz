pub mod auth;
pub mod commands;
pub mod display;
pub mod platform;
pub mod session;

// Re-export commonly used types for easier testing
pub use platform::{Discussion, GitPlatform, MergeRequest, Repository};
pub use session::{ReviewSession, SessionManager};
