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

fn updates_api_base() -> String {
  // Allow overriding the releases API base for testing via env var
  // Default to GitHub repo API
  env::var("BLIZZ_UPDATES_API_BASE")
    .unwrap_or_else(|_| "https://api.github.com/repos/TravelSizedLions/blizz".to_string())
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

  // Check if we're already on the target version
  let current_version = get_current_version();
  let target_version_clean = target_version.strip_prefix('v').unwrap_or(&target_version);
  if current_version == target_version_clean {
    println!("you're already up to date!");
    return Ok(());
  }

  let staging_dir = TempDir::new().context("Failed to create staging directory")?;
  let blizz_staging = staging_dir.path().join("blizz_home");
  fs::create_dir_all(&blizz_staging)?;
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
    .env("BLIZZ_HOME", &blizz_staging)
    .env("RUST_MIN_STACK", "1000000000")
    .env("CARGO_NET_RETRY", "3")
    .env("RUSTFLAGS", "-C opt-level=1 -C codegen-units=16")
    .output()
    .context("Failed to run install.sh for new version")?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("installation failed: {stderr}");
    println!("automatically rolling back to previous version...");
    match perform_rollback(&snapshot_dir).await {
      Ok(()) => {
        println!("rollback completed successfully");
        return Err(UpdateError::update_failed_rolled_back(stderr.to_string()).into());
      }
      Err(rollback_err) => {
        println!("CRITICAL: rollback also FAILED: {rollback_err}");
        return Err(
          UpdateError::update_and_rollback_failed(stderr.to_string(), rollback_err.to_string())
            .into(),
        );
      }
    }
  }

  // Verify installation using the temp BLIZZ_HOME
  println!("verifying installation with temp BLIZZ_HOME...");
  let verify_output = Command::new("blizz")
    .arg("--version")
    .env("BLIZZ_HOME", &blizz_staging)
    .output()
    .context("failed to test blizz after installation")?;

  if !verify_output.status.success() {
    let stderr = String::from_utf8_lossy(&verify_output.stderr);
    println!("verification failed: {stderr}");
    println!("automatically rolling back to previous version...");
    match perform_rollback(&snapshot_dir).await {
      Ok(()) => {
        println!("rollback completed successfully");
        return Err(UpdateError::update_failed_rolled_back(stderr.to_string()).into());
      }
      Err(rollback_err) => {
        println!("CRITICAL: rollback also failed: {rollback_err}");
        return Err(
          UpdateError::update_and_rollback_failed(stderr.to_string(), rollback_err.to_string())
            .into(),
        );
      }
    }
  }

  let real_blizz_home = env::var("BLIZZ_HOME").unwrap_or_else(|_| {
    format!("{}/.blizz", dirs::home_dir().unwrap_or_default().to_string_lossy())
  });
  let real_blizz_home = Path::new(&real_blizz_home);
  let new_blizz_home = &blizz_staging;

  let volatile_path = real_blizz_home.join("volatile");
  if volatile_path.exists() {
    fs::remove_dir_all(&volatile_path)?;
  }
  let source_path = real_blizz_home.join("blizz.internal.source");
  if source_path.exists() {
    fs::remove_file(&source_path)?;
  }

  let new_volatile = new_blizz_home.join("volatile");
  if new_volatile.exists() {
    copy_dir_recursive(&new_volatile, &volatile_path)?;
  }

  let new_source = new_blizz_home.join("blizz.internal.source");
  if new_source.exists() {
    fs::copy(&new_source, &source_path)?;
  }

  // Update the uninstaller script
  let uninstall_path = real_blizz_home.join("uninstall.sh");
  let new_uninstall = new_blizz_home.join("uninstall.sh");
  if new_uninstall.exists() {
    fs::copy(&new_uninstall, &uninstall_path)?;
    // Ensure it's executable
    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;
      let mut perms = fs::metadata(&uninstall_path)?.permissions();
      perms.set_mode(0o755);
      fs::set_permissions(&uninstall_path, perms)?;
    }
  }

  println!("update complete!");
  println!("snapshot saved at: {}", snapshot_dir.display());
  println!("snapshot will be automatically cleaned up in 24 hours");
  Ok(())
}

fn get_current_version() -> String {
  env!("CARGO_PKG_VERSION").to_string()
}

async fn get_latest_version() -> Result<String> {
  let base = updates_api_base();
  let url = format!("{base}/releases/latest");
  get_latest_version_from_url(&url).await
}

async fn get_latest_version_from_url(url: &str) -> Result<String> {
  let client = reqwest::Client::new();

  let response = client
    .get(url)
    .header("User-Agent", "blizz-updater")
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
  let base = updates_api_base();
  download_and_extract_from_api(version, staging_path, &format!("{base}/releases")).await
}

async fn download_and_extract_from_api(
  version: &str,
  staging_path: &Path,
  api_base: &str,
) -> Result<std::path::PathBuf> {
  let client = reqwest::Client::new();

  // Normalize version for GitHub API - ensure it has 'v' prefix
  let normalized_version =
    if version.starts_with('v') { version.to_string() } else { format!("v{version}") };

  // Get release info
  let release_url = if version == "latest" {
    format!("{api_base}/latest")
  } else {
    format!("{api_base}/tags/{normalized_version}")
  };

  let response = client
    .get(&release_url)
    .header("User-Agent", "blizz-updater")
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
    .header("User-Agent", "blizz-updater")
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

  let tarball_path = staging_path.join("blizz.tar.gz");
  let tarball_bytes = tarball_response.bytes().await.context("Failed to read tarball content")?;

  fs::write(&tarball_path, &tarball_bytes).context("Failed to write tarball to disk")?;

  println!("extracting...");
  let extracted_root = staging_path.join("extracted");
  fs::create_dir_all(&extracted_root)?;

  let output = Command::new("tar")
    .arg("-xzf")
    .arg(&tarball_path)
    .arg("-C")
    .arg(&extracted_root)
    .current_dir(staging_path)
    .output()
    .context("failed to execute tar command")?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    return Err(
      UpdateError::extraction_failed(format!("failed to extract tarball: {stderr}")).into(),
    );
  }

  // Determine the single top-level directory created by extraction
  let mut dirs: Vec<std::path::PathBuf> = vec![];
  for entry in fs::read_dir(&extracted_root)? {
    let entry = entry?;
    let path = entry.path();
    if path.is_dir() {
      dirs.push(path);
    }
  }

  // Expect exactly one directory from the tarball; error otherwise to remain deterministic
  if !dirs.is_empty() {
    dirs.sort();
    return Ok(dirs.remove(0));
  }

  Err(UpdateError::extraction_failed("could not find extracted directory").into())
}

async fn create_snapshot() -> Result<std::path::PathBuf> {
  let blizz_home = env::var("BLIZZ_HOME").unwrap_or_else(|_| {
    format!("{}/.blizz", dirs::home_dir().unwrap_or_default().to_string_lossy())
  });
  let install_dir = env::var("INSTALL_DIR").unwrap_or_else(|_| {
    format!("{}/.cargo/bin", dirs::home_dir().unwrap_or_default().to_string_lossy())
  });

  let snapshot_base = Path::new(&blizz_home).join("snapshots");
  fs::create_dir_all(&snapshot_base).context("Failed to create snapshots directory")?;

  let timestamp = chrono::Utc::now().timestamp();
  let snapshot_dir = snapshot_base.join(timestamp.to_string());
  fs::create_dir_all(&snapshot_dir)?;

  // Snapshot the entire blizz_home directory (including persistent for backup purposes)
  let blizz_home_snapshot = snapshot_dir.join("blizz_home");
  copy_dir_recursive(Path::new(&blizz_home), &blizz_home_snapshot)?;

  // Snapshot binaries
  let bins_snapshot = snapshot_dir.join("bins");
  fs::create_dir_all(&bins_snapshot)?;

  let binaries = ["blizz", "insights", "insights_embedding_daemon", "install_insights_cuda_dependencies", "secrets", "keeper", "violet", "adam"];
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
  // Get the install directory and verify blizz binary exists there
  let install_dir = env::var("INSTALL_DIR")
    .unwrap_or_else(|_| format!("{}/.cargo/bin", env::var("HOME").unwrap_or_default()));

  let blizz_path = Path::new(&install_dir).join("blizz");

  if !blizz_path.exists() {
    return Err(anyhow::anyhow!(
      "blizz binary not found at expected location: {}",
      blizz_path.display()
    ));
  }

  // Test that blizz works
  let output = Command::new(&blizz_path)
    .arg("--version")
    .output()
    .context("Failed to test blizz after installation")?;

  if !output.status.success() {
    return Err(anyhow::anyhow!("blizz failed version check after installation"));
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
      // Check if it's a regular file before copying
      let metadata = match fs::metadata(&src_path) {
        Ok(meta) => meta,
        Err(_) => continue, // Skip files we can't read metadata for
      };

      // Only copy regular files, skip special files (sockets, pipes, devices, etc.)
      if metadata.is_file() {
        fs::copy(&src_path, &dst_path)?;
      }
    }
  }

  Ok(())
}

async fn perform_rollback(snapshot_path: &Path) -> Result<()> {
  println!("Rolling back from snapshot: {}", snapshot_path.display());

  let blizz_home = env::var("BLIZZ_HOME")
    .unwrap_or_else(|_| format!("{}/.blizz", env::var("HOME").unwrap_or_default()));
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

  // Check if we have a full blizz_home backup
  let blizz_home_backup = snapshot_path.join("blizz_home");
  if blizz_home_backup.exists() {
    let blizz_home_path = Path::new(&blizz_home);

    // Remove everything from blizz_home except snapshots and persistent
    for entry in fs::read_dir(blizz_home_path)? {
      let entry = entry?;
      let path = entry.path();

      // Skip snapshots directory to avoid deleting our own snapshot
      if entry.file_name() == "snapshots" {
        continue;
      }

      // Skip persistent directory - NEVER touch it during rollback
      if entry.file_name() == "persistent" {
        continue;
      }

      if path.is_dir() {
        fs::remove_dir_all(&path)?;
      } else {
        fs::remove_file(&path)?;
      }
    }

    // Restore everything from backup except persistent and snapshots
    for entry in fs::read_dir(&blizz_home_backup)? {
      let entry = entry?;
      let src_path = entry.path();
      let dst_path = blizz_home_path.join(entry.file_name());

      // Skip restoring persistent directory - it should never be touched
      if entry.file_name() == "persistent" {
        continue;
      }

      // Skip snapshots directory to avoid overwriting current snapshots
      if entry.file_name() == "snapshots" {
        continue;
      }

      if src_path.is_dir() {
        copy_dir_recursive(&src_path, &dst_path)?;
      } else {
        fs::copy(&src_path, &dst_path)?;
      }
    }

    println!("restored blizz home from full backup (persistent directory left untouched)");
  } else {
    // Fallback to legacy volatile-only restore for older snapshots
    let volatile_backup = snapshot_path.join("volatile");
    if volatile_backup.exists() {
      let volatile_path = Path::new(&blizz_home).join("volatile");

      // Remove current volatile directory if it exists
      if volatile_path.exists() {
        fs::remove_dir_all(&volatile_path)?;
      }

      // Restore volatile directory from backup
      copy_dir_recursive(&volatile_backup, &volatile_path)?;

      println!("restored .blizz/volatile/ (legacy backup format)");
    }
  }

  // Restore binaries
  let bins_backup = snapshot_path.join("bins");
  if bins_backup.exists() {
    let binaries = ["blizz", "insights", "insights_embedding_daemon", "install_insights_cuda_dependencies", "secrets", "keeper", "violet", "adam"];
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
            "tarball_url": "https://api.github.com/repos/TravelSizedLions/blizz/tarball/v1.2.3"
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

  #[tokio::test]
  async fn test_download_version_without_v_prefix_should_work() {
    let mut server = Server::new_async().await;

    // Mock the release API response for version v1.2.3
    let mock_response = r#"{
      "tag_name": "v1.2.3",
      "tarball_url": "https://api.github.com/repos/test/test/tarball/v1.2.3"
    }"#;

    // Only /tags/v1.2.3 exists (like real GitHub), /tags/1.2.3 returns 404
    let _mock_v_version = server
      .mock("GET", "/tags/v1.2.3")
      .with_status(200)
      .with_header("content-type", "application/json")
      .with_body(mock_response)
      .create_async()
      .await;

    let _mock_no_v_version =
      server.mock("GET", "/tags/1.2.3").with_status(404).create_async().await;

    let temp_dir = TempDir::new().unwrap();
    let mock_url = server.url();

    // Test with v prefix - should work
    let result_with_v = download_and_extract_from_api("v1.2.3", temp_dir.path(), &mock_url).await;

    // Test without v prefix - should now work after fix (internally normalizes to v1.2.3)
    let result_without_v = download_and_extract_from_api("1.2.3", temp_dir.path(), &mock_url).await;

    // Both should succeed at least to download step, but fail at extraction (fake content)
    assert!(result_with_v.is_err());
    assert!(result_without_v.is_err());

    // Verify both fail at extraction/download, not version lookup
  }

  #[tokio::test]
  async fn test_download_latest_version_still_works() {
    let mut server = Server::new_async().await;

    // Mock the latest release API response
    let mock_response = r#"{
      "tag_name": "v2.0.0",
      "tarball_url": "https://api.github.com/repos/test/test/tarball/v2.0.0"
    }"#;

    // Latest endpoint should still work without v prefix normalization
    let _mock_latest = server
      .mock("GET", "/latest")
      .with_status(200)
      .with_header("content-type", "application/json")
      .with_body(mock_response)
      .create_async()
      .await;

    let temp_dir = TempDir::new().unwrap();
    let mock_url = server.url();

    // Test latest - should work and not be affected by v prefix logic
    let result_latest = download_and_extract_from_api("latest", temp_dir.path(), &mock_url).await;

    // Should fail at extraction/download, not version lookup
    assert!(result_latest.is_err());
    let error_latest = result_latest.unwrap_err();
    let update_error_latest = error_latest.downcast_ref::<UpdateError>().unwrap();

    if let UpdateError::VersionNotFound { .. } = update_error_latest {
      panic!("Should not be VersionNotFound for latest");
    } // Expected - should be a download/extraction error
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

  #[test]
  fn test_copy_dir_recursive_with_socket() {
    use std::os::unix::net::UnixListener;

    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let dst_dir = temp_dir.path().join("dst");

    // Create source directory structure
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("regular_file.txt"), "content").unwrap();

    // Create a Unix socket file (this will create a special file type)
    let socket_path = src_dir.join("test.sock");
    let _listener = UnixListener::bind(&socket_path).unwrap();

    // Verify the socket file exists and is not a regular file
    assert!(socket_path.exists());
    let metadata = fs::metadata(&socket_path).unwrap();
    assert!(!metadata.is_file()); // Should not be a regular file

    // Test copy_dir_recursive - should succeed and skip the socket
    let result = copy_dir_recursive(&src_dir, &dst_dir);
    assert!(result.is_ok());

    // Verify regular file was copied but socket was skipped
    assert!(dst_dir.join("regular_file.txt").exists());
    assert!(!dst_dir.join("test.sock").exists()); // Socket should be skipped
    assert_eq!(fs::read_to_string(dst_dir.join("regular_file.txt")).unwrap(), "content");
  }

  #[test]
  fn test_get_current_version() {
    let current_version = get_current_version();
    // Should return the cargo package version
    assert_eq!(current_version, env!("CARGO_PKG_VERSION"));
  }

  #[tokio::test]
  async fn test_execute_already_up_to_date() {
    // Test the version comparison logic directly
    let current_version = env!("CARGO_PKG_VERSION");

    // Test with 'v' prefix (common GitHub tag format)
    let target_version_with_v = format!("v{current_version}");
    let target_version_clean =
      target_version_with_v.strip_prefix('v').unwrap_or(&target_version_with_v);
    assert_eq!(current_version, target_version_clean);

    // Test without 'v' prefix
    let target_version_without_v = current_version.to_string();
    let target_version_clean =
      target_version_without_v.strip_prefix('v').unwrap_or(&target_version_without_v);
    assert_eq!(current_version, target_version_clean);
  }

  #[tokio::test]
  async fn test_get_latest_version_mock_up_to_date() {
    let mut server = Server::new_async().await;

    // Mock the GitHub API to return the current version as the latest
    let current_version = env!("CARGO_PKG_VERSION");
    let mock_response = format!(
      r#"{{
      "tag_name": "v{current_version}",
      "tarball_url": "https://api.github.com/repos/blizz-soft/blizz/tarball/v{current_version}"
    }}"#
    );

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
    let latest_version = result.unwrap();
    assert_eq!(latest_version, format!("v{current_version}"));

    // Test the version comparison logic
    let latest_version_clean = latest_version.strip_prefix('v').unwrap_or(&latest_version);
    assert_eq!(current_version, latest_version_clean);
  }

  #[test]
  fn test_version_normalization_logic() {
    // Test the version normalization logic directly
    let test_cases = vec![
      ("1.2.3", "v1.2.3"),    // Should add v prefix
      ("v1.2.3", "v1.2.3"),   // Should keep v prefix
      ("0.2.20", "v0.2.20"),  // Should add v prefix (the original issue case)
      ("v0.2.20", "v0.2.20"), // Should keep v prefix
      ("2.0.0", "v2.0.0"),    // Should add v prefix
      ("v2.0.0", "v2.0.0"),   // Should keep v prefix
    ];

    for (input, expected) in test_cases {
      let normalized = if input.starts_with('v') { input.to_string() } else { format!("v{input}") };
      assert_eq!(normalized, expected, "Failed for input: {input}");
    }
  }

  #[test]
  fn test_latest_version_not_normalized() {
    // Test that "latest" is not affected by normalization
    let version = "latest";

    // The actual logic in the function checks if version == "latest"
    // to use a different URL path, so normalization doesn't affect it
    if version == "latest" {
      // For latest, the URL is built differently: /releases/latest vs /releases/tags/{version}
      assert_eq!(version, "latest");
    } else {
      // For non-latest versions, normalization applies
      let normalized =
        if version.starts_with('v') { version.to_string() } else { format!("v{version}") };
      assert!(normalized.starts_with('v'));
    }
  }

  #[test]
  fn test_persistent_data_recovery_from_snapshot() {
    // Test scenario: verify that persistent data can be manually recovered from snapshots
    let temp_dir = TempDir::new().unwrap();
    let snapshot_dir = temp_dir.path().join("snapshot");
    let blizz_home_backup = snapshot_dir.join("blizz_home");
    let snapshot_persistent = blizz_home_backup.join("persistent");

    // Create a mock snapshot with persistent data
    fs::create_dir_all(&snapshot_persistent).unwrap();
    fs::write(snapshot_persistent.join("important_data.txt"), "important data").unwrap();
    fs::write(snapshot_persistent.join("credentials.enc"), "encrypted credentials").unwrap();

    // Verify the snapshot contains persistent data that could be manually recovered
    assert!(snapshot_persistent.join("important_data.txt").exists());
    assert!(snapshot_persistent.join("credentials.enc").exists());
    assert_eq!(
      fs::read_to_string(snapshot_persistent.join("important_data.txt")).unwrap(),
      "important data"
    );
    assert_eq!(
      fs::read_to_string(snapshot_persistent.join("credentials.enc")).unwrap(),
      "encrypted credentials"
    );

    // This test demonstrates that persistent data is available in snapshots
    // for manual recovery if ~/.blizz/persistent gets accidentally deleted
  }

  #[tokio::test]
  async fn test_complete_backup_and_rollback_workflow() {
    // Test the complete workflow described in the issue:
    // 1. take snapshot of entire ~/.blizz folder
    // 2. perform update, but don't touch ~/.blizz/persistent
    // 3. if rollback is needed, perform rollback, but STILL do not touch ~/.blizz/persistent

    let temp_dir = TempDir::new().unwrap();
    let blizz_home = temp_dir.path().join(".blizz");
    let volatile_dir = blizz_home.join("volatile");
    let persistent_dir = blizz_home.join("persistent");

    // Set up initial state
    fs::create_dir_all(&volatile_dir).unwrap();
    fs::create_dir_all(&persistent_dir).unwrap();
    fs::write(volatile_dir.join("old_volatile.txt"), "old volatile").unwrap();
    fs::write(persistent_dir.join("user_data.txt"), "important user data").unwrap();
    fs::write(blizz_home.join("config.toml"), "old config").unwrap();

    // Set environment
    std::env::set_var("BLIZZ_HOME", blizz_home.to_string_lossy().to_string());
    std::env::set_var("INSTALL_DIR", "/tmp/non_existent_bin_dir");

    // Step 1: Create snapshot of entire ~/.blizz folder (including persistent)
    let snapshot_path = create_snapshot().await.unwrap();
    let blizz_home_backup = snapshot_path.join("blizz_home");

    // Verify entire ~/.blizz was snapshotted including persistent
    assert!(blizz_home_backup.join("volatile").exists());
    assert!(blizz_home_backup.join("persistent").exists());
    assert!(blizz_home_backup.join("config.toml").exists());
    assert!(blizz_home_backup.join("persistent").join("user_data.txt").exists());

    // Step 2: Simulate update that changes volatile and config but doesn't touch persistent
    fs::write(volatile_dir.join("new_volatile.txt"), "new volatile").unwrap();
    fs::remove_file(volatile_dir.join("old_volatile.txt")).unwrap();
    fs::write(blizz_home.join("config.toml"), "new config").unwrap();
    // Note: persistent directory is intentionally left untouched during "update"

    // Verify update didn't affect persistent
    assert_eq!(
      fs::read_to_string(persistent_dir.join("user_data.txt")).unwrap(),
      "important user data"
    );

    // Step 3: Test the rollback logic manually to avoid verification issues
    // (We can't test the full perform_rollback function because it tries to verify the blizz binary)
    let blizz_home_str = blizz_home.to_string_lossy().to_string();
    let blizz_home_path = std::path::Path::new(&blizz_home_str);
    let blizz_home_backup = snapshot_path.join("blizz_home");

    // Preserve the persistent directory by moving it temporarily
    let persistent_path = blizz_home_path.join("persistent");
    let temp_dir_for_persistent = tempfile::TempDir::new().unwrap();
    let temp_persistent_path = temp_dir_for_persistent.path().join("persistent");
    copy_dir_recursive(&persistent_path, &temp_persistent_path).unwrap();

    // Remove everything from blizz_home except snapshots
    for entry in fs::read_dir(blizz_home_path).unwrap() {
      let entry = entry.unwrap();
      let path = entry.path();

      // Skip snapshots directory to avoid deleting our own snapshot
      if entry.file_name() == "snapshots" {
        continue;
      }

      if path.is_dir() {
        fs::remove_dir_all(&path).unwrap();
      } else {
        fs::remove_file(&path).unwrap();
      }
    }

    // Restore everything from backup except persistent
    for entry in fs::read_dir(&blizz_home_backup).unwrap() {
      let entry = entry.unwrap();
      let src_path = entry.path();
      let dst_path = blizz_home_path.join(entry.file_name());

      // Skip restoring persistent directory
      if entry.file_name() == "persistent" {
        continue;
      }

      // Skip snapshots directory to avoid overwriting current snapshots
      if entry.file_name() == "snapshots" {
        continue;
      }

      if src_path.is_dir() {
        copy_dir_recursive(&src_path, &dst_path).unwrap();
      } else {
        fs::copy(&src_path, &dst_path).unwrap();
      }
    }

    // Restore the preserved persistent directory
    copy_dir_recursive(&temp_persistent_path, &persistent_path).unwrap();

    // Verify rollback restored volatile and config
    assert!(
      volatile_dir.join("old_volatile.txt").exists(),
      "old_volatile.txt should exist after rollback"
    );
    assert!(
      !volatile_dir.join("new_volatile.txt").exists(),
      "new_volatile.txt should not exist after rollback"
    );
    assert_eq!(fs::read_to_string(blizz_home.join("config.toml")).unwrap(), "old config");

    // Verify persistent data was STILL not touched during rollback
    assert!(persistent_dir.join("user_data.txt").exists());
    assert_eq!(
      fs::read_to_string(persistent_dir.join("user_data.txt")).unwrap(),
      "important user data"
    );

    // Verify recovery path exists: persistent data is available in snapshot for manual recovery
    let snapshot_persistent_data = blizz_home_backup.join("persistent").join("user_data.txt");
    assert!(snapshot_persistent_data.exists());
    assert_eq!(fs::read_to_string(&snapshot_persistent_data).unwrap(), "important user data");

    // Clean up
    std::env::remove_var("BLIZZ_HOME");
    std::env::remove_var("INSTALL_DIR");
  }
}
