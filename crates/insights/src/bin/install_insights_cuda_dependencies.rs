use anyhow::{anyhow, Result};
use std::process::{Command, Stdio};
use std::env;

fn main() -> Result<()> {
  // Skip entirely in CI environments
  if is_ci_environment() {
    bentley::info("CI environment detected - skipping CUDA dependency setup");
    return Ok(());
  }

  bentley::info("Checking NVIDIA GPU and CUDA dependencies...");

  // 1. Check if NVIDIA GPU hardware is present
  if !has_nvidia_gpu()? {
    bentley::info("No NVIDIA GPU detected in system - I'm no expert, but that's probably a more important dependency for using an NVIDIA GPU than having their drivers.");
    bentley::info("Skipping script.");
    return Ok(());
  }

  // 2. Check LD_LIBRARY_PATH configuration
  check_library_path()?;

  // 3. Check and install NVIDIA dependencies (Ubuntu/apt only)
  if !is_ubuntu_apt() {
    bentley::info("Non-Ubuntu system detected - please install CUDA dependencies manually if needed");
    print_manual_instructions()?;
    return Ok(());
  }

  // 4. Ensure NVIDIA drivers are installed and accessible
  if !check_nvidia_drivers()? {
    return Ok(()); // Error messages already printed
  }

  check_and_install_cuda_dependencies()?;
  
  bentley::success("CUDA dependency check complete!");
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
  // Method 1: Check /proc/driver/nvidia/gpus/ (if NVIDIA drivers are installed)
  if let Ok(entries) = std::fs::read_dir("/proc/driver/nvidia/gpus") {
    if entries.count() > 0 {
      bentley::info("NVIDIA GPU detected via /proc/driver/nvidia");
      return Ok(true);
    }
  }

  // Method 2: Use lspci to check for NVIDIA VGA devices  
  if let Ok(output) = Command::new("lspci")
    .args(["-nn"])
    .output() 
  {
    let output_str = String::from_utf8_lossy(&output.stdout);
    for line in output_str.lines() {
      if line.contains("VGA") || line.contains("3D") || line.contains("Display") {
        if line.to_lowercase().contains("nvidia") || line.contains("[10de:") { // 10de is NVIDIA vendor ID
          bentley::info("NVIDIA GPU detected via lspci");
          return Ok(true);
        }
      }
    }
  }

  // Method 3: Check /sys/class/drm for NVIDIA cards
  if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
    for entry in entries.flatten() {
      if let Some(name) = entry.file_name().to_str() {
        if name.starts_with("card") && name.len() > 4 {
          let device_path = entry.path().join("device/vendor");
          if let Ok(vendor) = std::fs::read_to_string(device_path) {
            if vendor.trim() == "0x10de" { // NVIDIA vendor ID
              bentley::info("NVIDIA GPU detected via sysfs");
              return Ok(true);
            }
          }
        }
      }
    }
  }

  Ok(false)
}

/// Check if NVIDIA drivers are installed and suggest installation if needed
fn check_nvidia_drivers() -> Result<bool> {
  // Check if nvidia-smi exists and works
  match Command::new("nvidia-smi")
      .stdout(Stdio::null())
      .stderr(Stdio::null())
      .status()
  {
    Ok(status) if status.success() => {
      bentley::info("NVIDIA drivers are installed and accessible");
      Ok(true)
    }
    _ => {
      bentley::warn("NVIDIA GPU found but drivers not accessible (nvidia-smi failed)");
      bentley::info("Please install NVIDIA drivers:");
      bentley::info("   sudo apt update");
      bentley::info("   sudo apt install -y nvidia-driver-580  # or latest available");
      bentley::info("   sudo reboot  # reboot required after driver installation");
      bentley::info("");
      bentley::info("Alternative: Install from NVIDIA's official repository:");
      bentley::info("   https://developer.nvidia.com/cuda-downloads");
      Ok(false)
    }
  }
}

/// Check if LD_LIBRARY_PATH is configured for ONNX Runtime
fn check_library_path() -> Result<()> {
  let ld_library_path = env::var("LD_LIBRARY_PATH").unwrap_or_default();
  let ort_cache_pattern = ".cache/ort.pyke.io/dfbin";
  
  if ld_library_path.contains(ort_cache_pattern) {
    // Find and report the specific ONNX Runtime path
    for path_part in ld_library_path.split(':') {
      if path_part.contains(ort_cache_pattern) && path_part.contains("onnxruntime/lib") {
        bentley::info(&format!("LD_LIBRARY_PATH configured: {}", path_part));
        return Ok(());
      }
    }
    bentley::info("LD_LIBRARY_PATH already configured for ONNX Runtime");
    return Ok(());
  }

  // Try to find the ONNX Runtime cache directory
  let home = env::var("HOME")?;
  let ort_cache_base = format!("{}/.cache/ort.pyke.io/dfbin/x86_64-unknown-linux-gnu", home);
  
  if let Ok(entries) = std::fs::read_dir(&ort_cache_base) {
    for entry in entries {
      if let Ok(entry) = entry {
        let lib_path = entry.path().join("onnxruntime/lib");
        if lib_path.exists() {
          bentley::info("ONNX Runtime GPU libraries found but LD_LIBRARY_PATH not configured");
          bentley::info("To enable GPU acceleration, add this to your ~/.zshrc (or ~/.bashrc):");
          bentley::info(format!("   export LD_LIBRARY_PATH=\"{}:$LD_LIBRARY_PATH\"", lib_path.display()).as_str());
          bentley::info("   Then restart your shell or run: source ~/.zshrc");
          bentley::info("Proceeding with CPU inference for now...");
          return Ok(());
        }
      }
    }
  }

  bentley::info("ONNX Runtime GPU libraries not found - they'll be downloaded when needed");
  Ok(())
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
  let output = Command::new("nvidia-smi")
    .arg("--version")
    .output()?;

  if output.status.success() {
    let output_str = String::from_utf8_lossy(&output.stdout);
    
    // Look for the line containing "CUDA Version" and extract version
    for line in output_str.lines() {
      if line.contains("CUDA Version") {
        if let Some(colon_pos) = line.find(':') {
          let cuda_version = line[colon_pos + 1..].trim();
          bentley::info(&format!("CUDA {} detected", cuda_version));
          return Ok(cuda_version.to_string());
        }
      }
    }
  }

  Err(anyhow!("Could not determine CUDA version from NVIDIA driver"))
}

/// Check cuDNN and install if missing
fn check_cudnn(cuda_version: &str) -> Result<()> {
  // Check if cuDNN is already installed
  if let Some(cudnn_info) = get_cudnn_info() {
    bentley::info(&format!("cuDNN {} is already installed", cudnn_info));
    return Ok(());
  }

  bentley::info("cuDNN not found");
  
  // Determine appropriate cuDNN package based on CUDA version
  let cudnn_package = match cuda_version {
    v if v.starts_with("13.") => "libcudnn9-cuda-13",
    v if v.starts_with("12.") => "libcudnn9-cuda-12", 
    v if v.starts_with("11.") => "libcudnn9-cuda-11",
    _ => {
      bentley::info(format!("Unknown CUDA version {}, defaulting to cuDNN for CUDA 12", cuda_version).as_str());
      "libcudnn9-cuda-12"
    }
  };

  bentley::info(format!("Attempting to install {} for CUDA {}...", cudnn_package, cuda_version).as_str());
  
  match install_cudnn(cudnn_package) {
    Ok(_) => {
      bentley::info(format!("{} installed successfully", cudnn_package).as_str());
      Ok(())
    }
    Err(e) => {
      bentley::info(format!("Failed to install {}: {}", cudnn_package, e).as_str());
      bentley::info("Please install cuDNN manually:");
      bentley::info("   sudo apt update");
      bentley::info(format!("   sudo apt install -y {}", cudnn_package).as_str());
      bentley::info("   # Or download from NVIDIA's cuDNN page");
      Err(e)
    }
  }
}

/// Get cuDNN information if installed, including version
fn get_cudnn_info() -> Option<String> {
  // First try to get version from package manager (most reliable)
  if let Ok(output) = Command::new("dpkg")
    .args(["-l"])
    .output() 
  {
    let output_str = String::from_utf8_lossy(&output.stdout);
    for line in output_str.lines() {
      if line.contains("libcudnn9") {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
          let package_name = parts[1];
          let version = parts[2];
          // Extract CUDA version from package name (e.g., libcudnn9-cuda-13)
          if let Some(cuda_part) = package_name.strip_prefix("libcudnn9-cuda-") {
            return Some(format!("v{} (CUDA {})", version, cuda_part));
          } else {
            return Some(format!("v{}", version));
          }
        }
      }
    }
  }

  // Fallback: check file system for library existence
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

  // Also try ldconfig as final check
  if let Ok(output) = Command::new("ldconfig").args(["-p"]).output() {
    let output_str = String::from_utf8_lossy(&output.stdout);
    if output_str.contains("libcudnn") {
      return Some("detected via ldconfig".to_string());
    }
  }

  None
}

/// Install cuDNN package
fn install_cudnn(package_name: &str) -> Result<()> {
  let status = Command::new("sudo")
    .args(["apt", "install", "-y", package_name])
    .status()?;
  
  if status.success() {
    Ok(())
  } else {
    Err(anyhow!("Failed to install {}", package_name))
  }
}

/// Print manual installation instructions for non-Ubuntu systems
fn print_manual_instructions() -> Result<()> {
  println!("📋 Manual GPU setup instructions:");
  println!("1. Ensure NVIDIA drivers are installed for your GPU");
  println!("2. Install cuDNN library matching your CUDA driver version");
  println!("3. Ensure cuDNN libraries are in your LD_LIBRARY_PATH");
  println!();
  println!("For detailed instructions, visit:");
  println!("  NVIDIA Drivers: https://www.nvidia.com/drivers/");
  println!("  cuDNN: https://docs.nvidia.com/deeplearning/cudnn/install-guide/");
  println!();
  println!("Note: CUDA toolkit is not required for GPU inference, only cuDNN.");
  Ok(())
}
