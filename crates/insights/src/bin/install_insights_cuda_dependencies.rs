use anyhow::{anyhow, Result};
use std::env;
use std::io::{self, Write};
use std::process::{Command, Stdio};

// NOTE: This file is intentionally not unit tested for the following reasons:
//
// 1. SYSTEM INTEGRATION SCRIPT: 95% of this code deals with system commands,
//    hardware detection, package installation, and file I/O operations that
//    require real system state and cannot be meaningfully mocked.
//
// 2. LOW VALUE-TO-EFFORT RATIO: The ~5% of pure logic (string parsing, env
//    checking) is simple and would require extensive mocking infrastructure
//    that adds complexity without proportional benefit.
//
// 3. FAILURE MODES: Most failures are environmental (missing drivers, network
//    issues, permission problems) rather than logical bugs in our code.
//
// For setup/installation scripts like this, clear error messages and good
// documentation provide more value than unit test coverage.
// violet ignore chunk

fn main() -> Result<()> {
  // Skip entirely in CI environments
  if is_ci_environment() {
    bentley::info!("CI environment detected.");
    bentley::info!("Skipping CUDA dependency setup.");
    return Ok(());
  }

  bentley::info!("Checking NVIDIA GPU and CUDA dependencies...");

  if !has_nvidia_gpu()? {
    bentley::info!("No NVIDIA GPU detected in system.");
    bentley::info!("Skipping CUDA dependency setup.");
    return Ok(());
  }

  check_library_path()?;

  if !is_ubuntu_apt() {
    bentley::info!(
      "Non-Ubuntu system detected - please install CUDA dependencies manually if needed"
    );
    print_manual_instructions()?;
    return Ok(());
  }

  if !check_nvidia_drivers()? {
    return Ok(()); // Error messages already printed
  }

  check_and_install_cuda_dependencies()?;

  bentley::success!("CUDA dependency check complete!");
  Ok(())
}

/// Check if we're running in a CI environment
fn is_ci_environment() -> bool {
  env::var("CI").unwrap_or_default() == "true"
    || env::var("GITHUB_ACTIONS").unwrap_or_default() == "true"
    || env::var("GITLAB_CI").unwrap_or_default() == "true"
    || env::var("JENKINS_URL").is_ok()
    || env::var("TRAVIS").unwrap_or_default() == "true"
}

/// Check if NVIDIA GPU hardware is present in the system
fn has_nvidia_gpu() -> Result<bool> {
  if check_nvidia_proc_directory()? {
    return Ok(true);
  }

  if check_nvidia_via_lspci()? {
    return Ok(true);
  }

  if check_nvidia_via_sysfs()? {
    return Ok(true);
  }

  Ok(false)
}

/// Check for NVIDIA GPU via /proc/driver/nvidia/gpus/ (if drivers are installed)
fn check_nvidia_proc_directory() -> Result<bool> {
  let Ok(entries) = std::fs::read_dir("/proc/driver/nvidia/gpus") else {
    return Ok(false);
  };

  if entries.count() == 0 {
    return Ok(false);
  }

  bentley::info!("NVIDIA GPU detected via /proc/driver/nvidia");
  Ok(true)
}

/// Check for NVIDIA GPU using lspci command
fn check_nvidia_via_lspci() -> Result<bool> {
  let Ok(output) = Command::new("lspci").args(["-nn"]).output() else {
    return Ok(false);
  };

  let output_str = String::from_utf8_lossy(&output.stdout);

  for line in output_str.lines() {
    if is_nvidia_display_device(line) {
      bentley::info!("NVIDIA GPU detected via lspci");
      return Ok(true);
    }
  }

  Ok(false)
}

/// Check if a lspci line represents an NVIDIA display device
fn is_nvidia_display_device(line: &str) -> bool {
  let is_display_device = line.contains("VGA") || line.contains("3D") || line.contains("Display");
  let is_nvidia = line.to_lowercase().contains("nvidia") || line.contains("[10de:");
  is_display_device && is_nvidia
}

/// Check for NVIDIA GPU via sysfs (/sys/class/drm)
fn check_nvidia_via_sysfs() -> Result<bool> {
  let Ok(entries) = std::fs::read_dir("/sys/class/drm") else {
    return Ok(false);
  };

  for entry in entries.flatten() {
    if is_nvidia_drm_device(&entry)? {
      bentley::info!("NVIDIA GPU detected via sysfs");
      return Ok(true);
    }
  }

  Ok(false)
}

/// Check if a DRM device entry represents an NVIDIA card
fn is_nvidia_drm_device(entry: &std::fs::DirEntry) -> Result<bool> {
  let file_name = entry.file_name();
  let Some(name) = file_name.to_str() else {
    return Ok(false);
  };

  if !name.starts_with("card") || name.len() <= 4 {
    return Ok(false);
  }

  let device_path = entry.path().join("device/vendor");
  let Ok(vendor) = std::fs::read_to_string(device_path) else {
    return Ok(false);
  };

  Ok(vendor.trim() == "0x10de") // NVIDIA vendor ID
}

/// Check if NVIDIA drivers are installed and attempt installation if needed
fn check_nvidia_drivers() -> Result<bool> {
  if is_nvidia_smi_working() {
    bentley::info!("NVIDIA drivers are installed and accessible");
    return Ok(true);
  }

  bentley::warn!("NVIDIA GPU found but drivers not accessible !(nvidia-smi failed)");
  handle_missing_nvidia_drivers()
}

/// Check if nvidia-smi command is working
fn is_nvidia_smi_working() -> bool {
  Command::new("nvidia-smi")
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .status()
    .map(|status| status.success())
    .unwrap_or(false)
}

/// Handle the case when NVIDIA drivers are missing or not working
fn handle_missing_nvidia_drivers() -> Result<bool> {
  match install_nvidia_drivers()? {
    true => handle_successful_driver_installation(),
    false => handle_skipped_driver_installation(),
  }
}

/// Handle successful driver installation (requires reboot)
fn handle_successful_driver_installation() -> Result<bool> {
  bentley::success!("NVIDIA drivers installed successfully!");
  bentley::warn!("*** REBOOT REQUIRED ***");
  bentley::info!("Please reboot your system and run this script again.");
  bentley::info!("After reboot, GPU acceleration will be available.");
  Ok(false) // Return false so we don't continue (need reboot)
}

/// Handle when driver installation is skipped or fails
fn handle_skipped_driver_installation() -> Result<bool> {
  bentley::info!("Driver installation skipped by user");
  print_manual_driver_instructions();
  Ok(false)
}

/// Attempt to install NVIDIA drivers
fn install_nvidia_drivers() -> Result<bool> {
  if is_ci_environment() {
    bentley::info!("CI environment detected - skipping interactive driver installation");
    return Ok(false);
  }

  if !prompt_user_for_driver_installation()? {
    bentley::info!("Driver installation skipped");
    return Ok(false);
  }

  bentley::info!("Proceeding with automatic installation...");

  update_package_lists()?;

  if try_ubuntu_drivers_autoinstall()? {
    return Ok(true);
  }

  try_fallback_driver_installation()
}

/// Prompt user to confirm driver installation
fn prompt_user_for_driver_installation() -> Result<bool> {
  bentley::info!("Would you like to install NVIDIA drivers automatically? !(y/N)");
  bentley::info!("This will:");
  bentley::info!("  1. Update package lists");
  bentley::info!("  2. Detect and install the recommended NVIDIA driver");
  bentley::info!("  3. Require a manual system reboot to take effect");

  print!("Install drivers? [y/N]: ");
  io::stdout().flush()?;

  let mut input = String::new();
  io::stdin().read_line(&mut input)?;
  let input = input.trim().to_lowercase();

  Ok(input == "y" || input == "yes")
}

/// Update system package lists
fn update_package_lists() -> Result<()> {
  bentley::info!("Updating package lists...");
  let status = Command::new("sudo").args(["apt", "update"]).status()?;

  if !status.success() {
    return Err(anyhow!("Failed to update package lists"));
  }

  Ok(())
}

/// Try ubuntu-drivers autoinstall method
fn try_ubuntu_drivers_autoinstall() -> Result<bool> {
  bentley::info!("Detecting recommended NVIDIA driver...");

  let Ok(output) = Command::new("ubuntu-drivers").arg("devices").output() else {
    bentley::warn!("ubuntu-drivers not available, trying alternative method...");
    return Ok(false);
  };

  if !output.status.success() {
    bentley::warn!("ubuntu-drivers not available, trying alternative method...");
    return Ok(false);
  }

  show_available_drivers(&output.stdout);

  if !run_ubuntu_drivers_autoinstall()? {
    bentley::warn!("ubuntu-drivers autoinstall failed, trying fallback installation...");
    return Ok(false);
  }

  if is_nvidia_smi_working() {
    return Ok(true);
  }

  bentley::warn!("ubuntu-drivers autoinstall completed but nvidia-smi still not working");
  bentley::warn!("This usually requires a reboot. Continuing with fallback installation...");
  Ok(false)
}

/// Show available NVIDIA drivers detected by ubuntu-drivers
fn show_available_drivers(stdout: &[u8]) {
  let devices_output = String::from_utf8_lossy(stdout);
  bentley::info!("Available drivers detected:");

  for line in devices_output.lines() {
    if line.contains("nvidia") || line.contains("recommended") {
      bentley::info!(&format!("  {line}"));
    }
  }
}

/// Run the ubuntu-drivers autoinstall command
fn run_ubuntu_drivers_autoinstall() -> Result<bool> {
  bentley::info!("Installing recommended drivers automatically...");
  let status = Command::new("sudo").args(["ubuntu-drivers", "autoinstall"]).status()?;
  Ok(status.success())
}

/// Try fallback driver installation method
fn try_fallback_driver_installation() -> Result<bool> {
  bentley::info!("Falling back to recent stable driver !(nvidia-driver-580)...");
  let status = Command::new("sudo").args(["apt", "install", "-y", "nvidia-driver-580"]).status()?;

  if !status.success() {
    return Err(anyhow!("Driver installation failed"));
  }

  if is_nvidia_smi_working() {
    bentley::info!("Driver installation successful and nvidia-smi is working!");
    Ok(true)
  } else {
    bentley::warn!("Driver packages installed but nvidia-smi not yet working");
    bentley::warn!("This is normal - drivers require a reboot to become active");
    Ok(true) // Installation succeeded, just needs reboot
  }
}

/// Print manual driver installation instructions
fn print_manual_driver_instructions() {
  bentley::info!("Manual NVIDIA driver installation:");
  bentley::info!("   sudo apt update");
  bentley::info!("   ubuntu-drivers devices  # see available drivers");
  bentley::info!("   sudo ubuntu-drivers autoinstall  # install recommended");
  bentley::info!("   # OR manually install specific version:");
  bentley::info!("   sudo apt install -y nvidia-driver-535  # or latest available");
  bentley::info!("   sudo reboot  # reboot required after driver installation");
  bentley::info!("");
  bentley::info!("Alternative: Install from NVIDIA's official repository:");
  bentley::info!("   https://developer.nvidia.com/cuda-downloads");
}

/// Check if LD_LIBRARY_PATH is configured for ONNX Runtime
fn check_library_path() -> Result<()> {
  let ld_library_path = env::var("LD_LIBRARY_PATH").unwrap_or_default();

  if is_ld_library_path_configured(&ld_library_path) {
    report_existing_library_path(&ld_library_path);
    return Ok(());
  }

  check_for_unconfigured_libraries()
}

/// Check if LD_LIBRARY_PATH already contains ONNX Runtime paths
fn is_ld_library_path_configured(ld_library_path: &str) -> bool {
  ld_library_path.contains(".cache/ort.pyke.io/dfbin")
}

/// Report the existing ONNX Runtime library path configuration
fn report_existing_library_path(ld_library_path: &str) {
  for path_part in ld_library_path.split(':') {
    if is_onnxruntime_lib_path(path_part) {
      bentley::info!(&format!("LD_LIBRARY_PATH configured: {path_part}"));
      return;
    }
  }
  bentley::info!("LD_LIBRARY_PATH already configured for ONNX Runtime");
}

/// Check if a path part is an ONNX Runtime library path
fn is_onnxruntime_lib_path(path_part: &str) -> bool {
  path_part.contains(".cache/ort.pyke.io/dfbin") && path_part.contains("onnxruntime/lib")
}

/// Look for ONNX Runtime libraries that aren't configured in LD_LIBRARY_PATH
fn check_for_unconfigured_libraries() -> Result<()> {
  let home = env::var("HOME")?;
  let ort_cache_base = format!("{home}/.cache/ort.pyke.io/dfbin/x86_64-unknown-linux-gnu");

  let Ok(entries) = std::fs::read_dir(&ort_cache_base) else {
    bentley::info!("ONNX Runtime GPU libraries not found - they'll be downloaded when needed");
    return Ok(());
  };

  for entry in entries.flatten() {
    let lib_path = entry.path().join("onnxruntime/lib");
    if lib_path.exists() {
      suggest_library_path_configuration(&lib_path);
      return Ok(());
    }
  }

  bentley::info!("ONNX Runtime GPU libraries not found - they'll be downloaded when needed");
  Ok(())
}

/// Suggest how to configure LD_LIBRARY_PATH for the found library
fn suggest_library_path_configuration(lib_path: &std::path::Path) {
  bentley::info!("ONNX Runtime GPU libraries found but LD_LIBRARY_PATH not configured");
  bentley::info!("To enable GPU acceleration, add this to your ~/.zshrc !(or ~/.bashrc):");
  bentley::info!(&format!("   export LD_LIBRARY_PATH=\"{}:$LD_LIBRARY_PATH\"", lib_path.display()));
  bentley::info!("   Then restart your shell or run: source ~/.zshrc");
  bentley::info!("Proceeding with CPU inference for now...");
}

/// Check if we're on Ubuntu with apt package manager
fn is_ubuntu_apt() -> bool {
  Command::new("apt-get")
    .arg("--version")
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .status()
    .map(|s| s.success())
    .unwrap_or(false)
}

/// Check and install CUDA dependencies  
fn check_and_install_cuda_dependencies() -> Result<()> {
  // Get CUDA version from driver
  let cuda_version = get_cuda_version_from_driver()?;

  // Check cuDNN
  check_cudnn(&cuda_version)?;

  Ok(())
}

/// Get CUDA version from NVIDIA driver
fn get_cuda_version_from_driver() -> Result<String> {
  // Try to get CUDA version from nvidia-smi --version
  let output = Command::new("nvidia-smi").arg("--version").output()?;

  if output.status.success() {
    let output_str = String::from_utf8_lossy(&output.stdout);

    // Look for the line containing "CUDA Version" and extract version
    for line in output_str.lines() {
      if line.contains("CUDA Version") {
        if let Some(colon_pos) = line.find(':') {
          let cuda_version = line[colon_pos + 1..].trim();
          bentley::info!(&format!("CUDA {cuda_version} detected"));
          return Ok(cuda_version.to_string());
        }
      }
    }
  }

  Err(anyhow!("Could not determine CUDA version from NVIDIA driver"))
}

/// Check cuDNN and install if missing
fn check_cudnn(cuda_version: &str) -> Result<()> {
  if let Some(cudnn_info) = get_cudnn_info() {
    bentley::info!(&format!("cuDNN {cudnn_info} is already installed"));
    return Ok(());
  }

  bentley::info!("cuDNN not found");
  install_appropriate_cudnn_package(cuda_version)
}

/// Install the appropriate cuDNN package for the given CUDA version
fn install_appropriate_cudnn_package(cuda_version: &str) -> Result<()> {
  let cudnn_package = determine_cudnn_package(cuda_version);

  bentley::info!(&format!("Attempting to install {cudnn_package} for CUDA {cuda_version}..."));

  match install_cudnn(cudnn_package) {
    Ok(_) => {
      bentley::info!(&format!("{cudnn_package} installed successfully"));
      Ok(())
    }
    Err(e) => {
      show_manual_cudnn_installation_instructions(cudnn_package);
      Err(e)
    }
  }
}

/// Determine the appropriate cuDNN package based on CUDA version
fn determine_cudnn_package(cuda_version: &str) -> &str {
  match cuda_version {
    v if v.starts_with("13.") => "libcudnn9-cuda-13",
    v if v.starts_with("12.") => "libcudnn9-cuda-12",
    v if v.starts_with("11.") => "libcudnn9-cuda-11",
    _ => {
      bentley::info!(&format!(
        "Unknown CUDA version {cuda_version}, defaulting to cuDNN for CUDA 12"
      ));
      "libcudnn9-cuda-12"
    }
  }
}

/// Show manual cuDNN installation instructions
fn show_manual_cudnn_installation_instructions(cudnn_package: &str) {
  bentley::info!(&format!("Failed to install {cudnn_package}"));
  bentley::info!("Please install cuDNN manually:");
  bentley::info!("   sudo apt update");
  bentley::info!(&format!("   sudo apt install -y {cudnn_package}"));
  bentley::info!("   # Or download from NVIDIA's cuDNN page");
}

/// Get cuDNN information if installed, including version
fn get_cudnn_info() -> Option<String> {
  if let Some(info) = get_cudnn_info_from_package_manager() {
    return Some(info);
  }

  if let Some(info) = check_cudnn_filesystem_paths() {
    return Some(info);
  }

  check_cudnn_via_ldconfig()
}

/// Try to get cuDNN information from the package manager (most reliable)
fn get_cudnn_info_from_package_manager() -> Option<String> {
  let Ok(output) = Command::new("dpkg").args(["-l"]).output() else {
    return None;
  };

  let output_str = String::from_utf8_lossy(&output.stdout);

  for line in output_str.lines() {
    if let Some(info) = parse_cudnn_package_line(line) {
      return Some(info);
    }
  }

  None
}

/// Parse a dpkg line to extract cuDNN package information
fn parse_cudnn_package_line(line: &str) -> Option<String> {
  if !line.contains("libcudnn9") {
    return None;
  }

  let parts: Vec<&str> = line.split_whitespace().collect();
  if parts.len() < 3 {
    return None;
  }

  let package_name = parts[1];
  let version = parts[2];

  // Extract CUDA version from package name (e.g., libcudnn9-cuda-13)
  if let Some(cuda_part) = package_name.strip_prefix("libcudnn9-cuda-") {
    Some(format!("v{version} (CUDA {cuda_part})"))
  } else {
    Some(format!("v{version}"))
  }
}

/// Check common filesystem paths for cuDNN libraries
fn check_cudnn_filesystem_paths() -> Option<String> {
  let cudnn_paths = [
    "/lib/x86_64-linux-gnu/libcudnn.so.9",
    "/usr/lib/x86_64-linux-gnu/libcudnn.so.9",
    "/usr/local/cuda/lib64/libcudnn.so.9",
  ];

  for path in &cudnn_paths {
    if std::path::Path::new(path).exists() {
      return Some("detected (version unknown)".to_string());
    }
  }

  None
}

/// Check for cuDNN via ldconfig as a final detection method
fn check_cudnn_via_ldconfig() -> Option<String> {
  let Ok(output) = Command::new("ldconfig").args(["-p"]).output() else {
    return None;
  };

  let output_str = String::from_utf8_lossy(&output.stdout);
  if output_str.contains("libcudnn") {
    Some("detected via ldconfig".to_string())
  } else {
    None
  }
}

/// Install cuDNN package
fn install_cudnn(package_name: &str) -> Result<()> {
  let status = Command::new("sudo").args(["apt", "install", "-y", package_name]).status()?;

  if status.success() {
    Ok(())
  } else {
    Err(anyhow!("Failed to install {}", package_name))
  }
}

/// Print manual installation instructions for non-Ubuntu systems
fn print_manual_instructions() -> Result<()> {
  bentley::info!("📋 Manual GPU setup instructions:");
  bentley::info!("1. Ensure NVIDIA drivers are installed for your GPU");
  bentley::info!("2. Install cuDNN library matching your CUDA driver version");
  bentley::info!("3. Ensure cuDNN libraries are in your LD_LIBRARY_PATH");
  bentley::info!("");
  bentley::info!("For detailed instructions, visit:");
  bentley::info!("  NVIDIA Drivers: https://www.nvidia.com/drivers/");
  bentley::info!("  cuDNN: https://docs.nvidia.com/deeplearning/cudnn/install-guide/");
  bentley::info!("");
  bentley::info!("Note: CUDA toolkit is not required for GPU inference, only cuDNN.");
  Ok(())
}
