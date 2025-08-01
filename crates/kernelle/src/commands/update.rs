use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UpdateError {
  #[error("Failed to fetch GitHub release: {message}")]
  GitHubApiFailed { message: String },

  #[error("Failed to parse GitHub release response: {message}")]
  GitHubParseError { message: String },

  #[error("Version '{version}' not found")]
  VersionNotFound { version: String },

  #[error("Update failed and was rolled back: {stderr}")]
  UpdateFailedRolledBack { stderr: String },

  #[error("Update failed: {update_error}. Rollback also failed: {rollback_error}. Manual recovery may be needed.")]
  UpdateAndRollbackFailed { update_error: String, rollback_error: String },

  #[error("Rollback failed: {message}")]
  RollbackFailed { message: String },

  #[error("Download failed: {message}")]
  DownloadFailed { message: String },

  #[error("Extraction failed: {message}")]
  ExtractionFailed { message: String },
}

impl UpdateError {
  pub fn github_api_failed(message: impl Into<String>) -> Self {
    Self::GitHubApiFailed { message: message.into() }
  }

  pub fn github_parse_error(message: impl Into<String>) -> Self {
    Self::GitHubParseError { message: message.into() }
  }

  pub fn version_not_found(version: impl Into<String>) -> Self {
    Self::VersionNotFound { version: version.into() }
  }

  pub fn update_failed_rolled_back(stderr: impl Into<String>) -> Self {
    Self::UpdateFailedRolledBack { stderr: stderr.into() }
  }

  pub fn update_and_rollback_failed(
    update_error: impl Into<String>,
    rollback_error: impl Into<String>,
  ) -> Self {
    Self::UpdateAndRollbackFailed {
      update_error: update_error.into(),
      rollback_error: rollback_error.into(),
    }
  }

  pub fn rollback_failed(message: impl Into<String>) -> Self {
    Self::RollbackFailed { message: message.into() }
  }

  pub fn download_failed(message: impl Into<String>) -> Self {
    Self::DownloadFailed { message: message.into() }
  }

  pub fn extraction_failed(message: impl Into<String>) -> Self {
    Self::ExtractionFailed { message: message.into() }
  }
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
  tag_name: String,
  tarball_url: String,
}

pub async fn execute(version: Option<&str>) -> Result<()> {
  println!("starting update...");

  let target_version = match version {
    Some(v) => {
      println!("updating to {v}");
      v.to_string()
    }
    None => {
      println!("fetching latest version...");
      get_latest_version().await?
    }
  };

  let staging_dir = TempDir::new().context("Failed to create staging directory")?;
  let kernelle_staging = staging_dir.path().join("kernelle_home");
  fs::create_dir_all(&kernelle_staging)?;
  println!("staging in: {}", staging_dir.path().display());

  println!("downloading {target_version}...");
  let extracted_dir = download_and_extract(&target_version, staging_dir.path()).await?;

  println!("creating snapshot of current installation...");
  let snapshot_dir = create_snapshot().await?;

  println!("installing...");
  let install_script = extracted_dir.join("scripts").join("install.sh");
  let output = Command::new("bash")
    .arg(&install_script)
    .arg("--non-interactive")
    .env("KERNELLE_HOME", &kernelle_staging)
    .env("RUST_MIN_STACK", "33554432")
    .env("CARGO_NET_RETRY", "3")
    .env("RUSTFLAGS", "-C opt-level=1 -C codegen-units=16")
    .output()
    .context("Failed to run install.sh for new version")?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("❌ installation failed: {stderr}");
    println!("automatically rolling back to previous version...");
    match perform_rollback(&snapshot_dir).await {
      Ok(()) => {
        println!("rollback completed successfully");
        return Err(UpdateError::update_failed_rolled_back(stderr.to_string()).into());
      }
      Err(rollback_err) => {
        println!("❌ CRITICAL: rollback also FAILED: {rollback_err}");
        return Err(
          UpdateError::update_and_rollback_failed(stderr.to_string(), rollback_err.to_string())
            .into(),
        );
      }
    }
  }

  // Verify installation using the temp KERNELLE_HOME
  println!("verifying installation with temp KERNELLE_HOME...");
  let verify_output = Command::new("kernelle")
    .arg("--version")
    .env("KERNELLE_HOME", &kernelle_staging)
    .output()
    .context("failed to test kernelle after installation")?;

  if !verify_output.status.success() {
    let stderr = String::from_utf8_lossy(&verify_output.stderr);
    println!("❌ verification failed: {stderr}");
    println!("automatically rolling back to previous version...");
    match perform_rollback(&snapshot_dir).await {
      Ok(()) => {
        println!("rollback completed successfully");
        return Err(UpdateError::update_failed_rolled_back(stderr.to_string()).into());
      }
      Err(rollback_err) => {
        println!("❌ CRITICAL: rollback also failed: {rollback_err}");
        return Err(
          UpdateError::update_and_rollback_failed(stderr.to_string(), rollback_err.to_string())
            .into(),
        );
      }
    }
  }

  let real_kernelle_home = env::var("KERNELLE_HOME").unwrap_or_else(|_| {
    format!("{}/.kernelle", dirs::home_dir().unwrap_or_default().to_string_lossy())
  });
  let real_kernelle_home = Path::new(&real_kernelle_home);
  let new_kernelle_home = &kernelle_staging;

  let volatile_path = real_kernelle_home.join("volatile");
  if volatile_path.exists() {
    fs::remove_dir_all(&volatile_path)?;
  }
  let source_path = real_kernelle_home.join("kernelle.internal.source");
  if source_path.exists() {
    fs::remove_file(&source_path)?;
  }

  let new_volatile = new_kernelle_home.join("volatile");
  if new_volatile.exists() {
    copy_dir_recursive(&new_volatile, &volatile_path)?;
  }

  let new_source = new_kernelle_home.join("kernelle.internal.source");
  if new_source.exists() {
    fs::copy(&new_source, &source_path)?;
  }

  println!("update complete!");
  println!("snapshot saved at: {}", snapshot_dir.display());
  println!("snapshot will be automatically cleaned up in 24 hours");
  Ok(())
}

async fn get_latest_version() -> Result<String> {
  get_latest_version_from_url(
    "https://api.github.com/repos/TravelSizedLions/kernelle/releases/latest",
  )
  .await
}

async fn get_latest_version_from_url(url: &str) -> Result<String> {
  let client = reqwest::Client::new();

  let response = client
    .get(url)
    .header("User-Agent", "kernelle-updater")
    .send()
    .await
    .context("failed to fetch latest release from GitHub")?;

  if !response.status().is_success() {
    return Err(
      UpdateError::github_api_failed(format!("request failed with status: {}", response.status()))
        .into(),
    );
  }

  let release: GitHubRelease = response.json().await.map_err(|e| {
    UpdateError::github_parse_error(format!("failed to parse release response: {e}"))
  })?;

  println!("latest version: {}", release.tag_name);
  Ok(release.tag_name)
}

async fn download_and_extract(version: &str, staging_path: &Path) -> Result<std::path::PathBuf> {
  download_and_extract_from_api(
    version,
    staging_path,
    "https://api.github.com/repos/TravelSizedLions/kernelle/releases",
  )
  .await
}

async fn download_and_extract_from_api(
  version: &str,
  staging_path: &Path,
  api_base: &str,
) -> Result<std::path::PathBuf> {
  let client = reqwest::Client::new();

  // Get release info
  let release_url = if version == "latest" {
    format!("{api_base}/latest")
  } else {
    format!("{api_base}/tags/{version}")
  };

  let response = client
    .get(&release_url)
    .header("User-Agent", "kernelle-updater")
    .send()
    .await
    .context("failed to fetch release info")?;

  if !response.status().is_success() {
    return Err(UpdateError::version_not_found(version.to_string()).into());
  }

  let release: GitHubRelease = response.json().await.map_err(|e| {
    UpdateError::github_parse_error(format!("Failed to parse GitHub release response: {e}"))
  })?;

  println!("downloading from: {}", release.tarball_url);
  let tarball_response = client
    .get(&release.tarball_url)
    .header("User-Agent", "kernelle-updater")
    .send()
    .await
    .context("failed to download release tarball")?;

  if !tarball_response.status().is_success() {
    return Err(
      UpdateError::download_failed(format!(
        "failed to download tarball: HTTP {}",
        tarball_response.status()
      ))
      .into(),
    );
  }

  let tarball_path = staging_path.join("kernelle.tar.gz");
  let tarball_bytes = tarball_response.bytes().await.context("Failed to read tarball content")?;

  fs::write(&tarball_path, &tarball_bytes).context("Failed to write tarball to disk")?;

  println!("extracting...");
  let output = Command::new("tar")
    .args(["-xzf", &tarball_path.to_string_lossy()])
    .current_dir(staging_path)
    .output()
    .context("failed to execute tar command")?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    return Err(
      UpdateError::extraction_failed(format!("failed to extract tarball: {stderr}")).into(),
    );
  }

  let entries = fs::read_dir(staging_path)?;
  for entry in entries {
    let entry = entry?;
    let path = entry.path();
    if path.is_dir()
      && path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name_str| name_str.contains("kernelle"))
        .unwrap_or(false)
      && path != tarball_path.parent().unwrap()
    {
      return Ok(path);
    }
  }

  Err(UpdateError::extraction_failed("could not find extracted directory").into())
}

async fn create_snapshot() -> Result<std::path::PathBuf> {
  let kernelle_home = env::var("KERNELLE_HOME").unwrap_or_else(|_| {
    format!("{}/.kernelle", dirs::home_dir().unwrap_or_default().to_string_lossy())
  });
  let install_dir = env::var("INSTALL_DIR").unwrap_or_else(|_| {
    format!("{}/.cargo/bin", dirs::home_dir().unwrap_or_default().to_string_lossy())
  });

  let snapshot_base = Path::new(&kernelle_home).join("snapshots");
  fs::create_dir_all(&snapshot_base).context("Failed to create snapshots directory")?;

  let timestamp = chrono::Utc::now().timestamp();
  let snapshot_dir = snapshot_base.join(timestamp.to_string());
  fs::create_dir_all(&snapshot_dir)?;

  // Snapshot kernelle home directory
  let kernelle_snapshot = snapshot_dir.join("kernelle_home");
  copy_dir_recursive(&kernelle_home, &kernelle_snapshot)?;

  // Snapshot binaries
  let bins_snapshot = snapshot_dir.join("bins");
  fs::create_dir_all(&bins_snapshot)?;

  let binaries = ["kernelle", "jerrod", "blizz", "violet", "adam", "sentinel"];
  for binary in &binaries {
    let src = Path::new(&install_dir).join(binary);
    if src.exists() {
      let dst = bins_snapshot.join(binary);
      fs::copy(&src, &dst).context(format!("Failed to backup {binary}"))?;
    }
  }

  Ok(snapshot_dir)
}

async fn verify_installation() -> Result<()> {
  // Test that kernelle works
  let output = Command::new("kernelle")
    .arg("--version")
    .output()
    .context("Failed to test kernelle after installation")?;

  if !output.status.success() {
    return Err(anyhow::anyhow!("kernelle failed version check after installation"));
  }

  println!("installation verified");
  Ok(())
}

fn copy_dir_recursive<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> Result<()> {
  let src = src.as_ref();
  let dst = dst.as_ref();

  if !src.exists() {
    return Ok(()); // Nothing to copy
  }

  fs::create_dir_all(dst)?;

  for entry in fs::read_dir(src)? {
    let entry = entry?;
    let src_path = entry.path();
    let dst_path = dst.join(entry.file_name());

    // Skip the snapshots directory to avoid infinite recursion
    if entry.file_name() == "snapshots" {
      continue;
    }

    if src_path.is_dir() {
      copy_dir_recursive(&src_path, &dst_path)?;
    } else {
      fs::copy(&src_path, &dst_path)?;
    }
  }

  Ok(())
}

async fn perform_rollback(snapshot_path: &Path) -> Result<()> {
  println!("🔄 Rolling back from snapshot: {}", snapshot_path.display());

  let kernelle_home = env::var("KERNELLE_HOME")
    .unwrap_or_else(|_| format!("{}/.kernelle", env::var("HOME").unwrap_or_default()));
  let install_dir = env::var("INSTALL_DIR")
    .unwrap_or_else(|_| format!("{}/.cargo/bin", env::var("HOME").unwrap_or_default()));

  if !snapshot_path.exists() {
    return Err(
      UpdateError::rollback_failed(format!(
        "Snapshot directory not found: {}",
        snapshot_path.display()
      ))
      .into(),
    );
  }

  // Restore kernelle home (excluding the snapshots directory itself)
  let kernelle_backup = snapshot_path.join("kernelle_home");
  if kernelle_backup.exists() {
    // Create a temporary backup of current snapshots
    let temp_snapshots = tempfile::tempdir()?;
    let snapshots_dir = Path::new(&kernelle_home).join("snapshots");
    if snapshots_dir.exists() {
      copy_dir_recursive(&snapshots_dir, temp_snapshots.path().join("snapshots"))?;
    }

    // Clear current kernelle home
    if Path::new(&kernelle_home).exists() {
      fs::remove_dir_all(&kernelle_home)?;
    }

    // Restore from backup
    copy_dir_recursive(&kernelle_backup, &kernelle_home)?;

    if Path::new(&kernelle_home).join("snapshots").exists() {
      fs::remove_dir_all(Path::new(&kernelle_home).join("snapshots"))?;
    }
    copy_dir_recursive(temp_snapshots.path().join("snapshots"), &snapshots_dir)?;

    println!("restored .kernelle/");
  }

  // Restore binaries
  let bins_backup = snapshot_path.join("bins");
  if bins_backup.exists() {
    let binaries = ["kernelle", "jerrod", "blizz", "violet", "adam", "sentinel"];
    for binary in &binaries {
      let backup_bin = bins_backup.join(binary);
      let install_bin = Path::new(&install_dir).join(binary);

      if backup_bin.exists() {
        if install_bin.exists() {
          fs::remove_file(&install_bin)?;
        }
        fs::copy(&backup_bin, &install_bin).context(format!("Failed to restore {binary}"))?;
        println!("restored {binary}");
      }
    }
  }

  println!("verifying rollback...");
  verify_installation().await?;
  println!("rollback completed successfully!");

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use mockito::Server;
  use std::fs;
  use tempfile::TempDir;

  #[tokio::test]
  async fn test_get_latest_version_success() {
    let mut server = Server::new_async().await;
    let mock_response = r#"{
            "tag_name": "v1.2.3",
            "tarball_url": "https://api.github.com/repos/TravelSizedLions/kernelle/tarball/v1.2.3"
        }"#;

    let _mock = server
      .mock("GET", "/releases/latest")
      .with_status(200)
      .with_header("content-type", "application/json")
      .with_body(mock_response)
      .create_async()
      .await;

    let mock_url = format!("{}/releases/latest", server.url());
    let result = get_latest_version_from_url(&mock_url).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "v1.2.3");
  }

  #[tokio::test]
  async fn test_get_latest_version_failure() {
    let mut server = Server::new_async().await;
    let _mock = server.mock("GET", "/releases/latest").with_status(404).create_async().await;

    let mock_url = format!("{}/releases/latest", server.url());
    let result = get_latest_version_from_url(&mock_url).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    let update_error = error.downcast_ref::<UpdateError>().unwrap();
    match update_error {
      UpdateError::GitHubApiFailed { .. } => {
        // Expected error type
      }
      _ => panic!("Expected GitHubApiFailed error, got: {update_error:?}"),
    }
  }

  #[tokio::test]
  async fn test_get_latest_version_invalid_json() {
    let mut server = Server::new_async().await;
    let _mock = server
      .mock("GET", "/releases/latest")
      .with_status(200)
      .with_header("content-type", "application/json")
      .with_body("invalid json")
      .create_async()
      .await;

    let mock_url = format!("{}/releases/latest", server.url());
    let result = get_latest_version_from_url(&mock_url).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    let update_error = error.downcast_ref::<UpdateError>().unwrap();
    match update_error {
      UpdateError::GitHubParseError { .. } => {
        // Expected error type
      }
      _ => panic!("Expected GitHubParseError error, got: {update_error:?}"),
    }
  }

  #[tokio::test]
  async fn test_download_version_not_found() {
    let mut server = Server::new_async().await;
    let _mock = server.mock("GET", "/tags/v99.99.99").with_status(404).create_async().await;

    let temp_dir = TempDir::new().unwrap();
    let mock_url = server.url();
    let result = download_and_extract_from_api("v99.99.99", temp_dir.path(), &mock_url).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    let update_error = error.downcast_ref::<UpdateError>().unwrap();
    match update_error {
      UpdateError::VersionNotFound { version } => {
        assert_eq!(version, "v99.99.99");
      }
      _ => panic!("Expected VersionNotFound error, got: {update_error:?}"),
    }
  }

  #[test]
  fn test_copy_dir_recursive() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let dst_dir = temp_dir.path().join("dst");

    // Create source directory structure
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(src_dir.join("subdir")).unwrap();
    fs::write(src_dir.join("file1.txt"), "content1").unwrap();
    fs::write(src_dir.join("subdir").join("file2.txt"), "content2").unwrap();

    // Test copy_dir_recursive
    let result = copy_dir_recursive(&src_dir, &dst_dir);
    assert!(result.is_ok());

    // Verify copied files
    assert!(dst_dir.join("file1.txt").exists());
    assert!(dst_dir.join("subdir").join("file2.txt").exists());
    assert_eq!(fs::read_to_string(dst_dir.join("file1.txt")).unwrap(), "content1");
    assert_eq!(fs::read_to_string(dst_dir.join("subdir").join("file2.txt")).unwrap(), "content2");
  }

  #[test]
  fn test_copy_dir_recursive_nonexistent_source() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("nonexistent");
    let dst_dir = temp_dir.path().join("dst");

    // Should succeed with no-op when source doesn't exist
    let result = copy_dir_recursive(&src_dir, &dst_dir);
    assert!(result.is_ok());
    assert!(!dst_dir.exists());
  }
}
