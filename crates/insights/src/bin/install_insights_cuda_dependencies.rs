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

  // 1. Check if we can attempt to use CUDA
  if !can_attempt_cuda()? {
    bentley::info("No NVIDIA GPU detected or CUDA not feasible - using CPU inference");
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

  check_and_install_cuda_dependencies()?;
  
  bentley::info("CUDA dependency check complete!");
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

/// Check if CUDA is worth attempting
fn can_attempt_cuda() -> Result<bool> {
  // Check if nvidia-smi exists and works
  match Command::new("nvidia-smi")
      .stdout(Stdio::null())
      .stderr(Stdio::null())
      .status()
  {
    Ok(status) if status.success() => {
      bentley::info("NVIDIA GPU detected");
      Ok(true)
    }
    _ => {
      bentley::info("No NVIDIA GPU detected (nvidia-smi not found or failed)");
      Ok(false)
    }
  }
}

/// Check if LD_LIBRARY_PATH is configured for ONNX Runtime
fn check_library_path() -> Result<()> {
  let ld_library_path = env::var("LD_LIBRARY_PATH").unwrap_or_default();
  let ort_cache_pattern = ".cache/ort.pyke.io/dfbin";
  
  if ld_library_path.contains(ort_cache_pattern) {
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
  // Check CUDA driver
  let cuda_version = check_cuda_driver()?;

  // Check cuDNN
  check_cudnn(&cuda_version)?;

  Ok(())
}

/// Check CUDA driver and attempt installation if missing
fn check_cuda_driver() -> Result<String> {
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

  bentley::warn("Could not determine CUDA version from nvidia-smi --version");
  
  // Check if CUDA toolkit is installed
  if Command::new("nvcc")
    .arg("--version")
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .status()
    .map(|s| s.success())
    .unwrap_or(false)
  {
    bentley::info("CUDA toolkit (nvcc) found");
    // Try to extract version from nvcc
    if let Ok(output) = Command::new("nvcc").arg("--version").output() {
      let output_str = String::from_utf8_lossy(&output.stdout);
      if let Some(version_line) = output_str.lines().find(|line| line.contains("release")) {
        if let Some(version) = extract_version_from_nvcc_output(version_line) {
          bentley::info(format!("CUDA toolkit version {} detected", version).as_str());
          return Ok(version);
        }
      }
    }
    return Ok("unknown".to_string());
  }

  // No CUDA found - offer to install
  bentley::info("No CUDA toolkit detected");
  bentley::info("Attempting to install CUDA drivers...");
  
  match install_cuda_driver() {
    Ok(version) => {
      bentley::info(format!("CUDA {} installed successfully", version).as_str());
      Ok(version)
    }
    Err(e) => {
      bentley::info(format!("Failed to install CUDA driver: {}", e).as_str());
      bentley::info("Please install CUDA manually:");
      bentley::info("   sudo apt update");
      bentley::info("   sudo apt install -y cuda-toolkit nvidia-cuda-toolkit");
      bentley::info("   # Or install from NVIDIA's official repository");
      Err(e)
    }
  }
}

/// Extract CUDA version from nvcc output
fn extract_version_from_nvcc_output(line: &str) -> Option<String> {
  // Look for pattern like "release 12.0, V12.0.140"
  if let Some(start) = line.find("release ") {
    let after_release = &line[start + 8..];
    if let Some(end) = after_release.find(',') {
      return Some(after_release[..end].to_string());
    }
  }
  None
}

/// Install CUDA driver using Ubuntu's package manager
fn install_cuda_driver() -> Result<String> {
  bentley::info("Installing CUDA toolkit from Ubuntu repositories...");
  
  // Update package lists
  let status = Command::new("sudo")
    .args(["apt", "update"])
    .status()?;
  
  if !status.success() {
    return Err(anyhow!("Failed to update package lists"));
  }

  // Install CUDA toolkit
  let status = Command::new("sudo")
    .args(["apt", "install", "-y", "nvidia-cuda-toolkit"])
    .status()?;
  
  if !status.success() {
    return Err(anyhow!("Failed to install CUDA toolkit"));
  }

  // Try to determine installed version
  if let Ok(output) = Command::new("nvcc").arg("--version").output() {
    let output_str = String::from_utf8_lossy(&output.stdout);
    if let Some(version_line) = output_str.lines().find(|line| line.contains("release")) {
      if let Some(version) = extract_version_from_nvcc_output(version_line) {
        return Ok(version);
      }
    }
  }
  
  Ok("unknown".to_string())
}

/// Check cuDNN and install if missing
fn check_cudnn(cuda_version: &str) -> Result<()> {
    // Check if cuDNN is already installed
    if is_cudnn_installed() {
      bentley::info("cuDNN is already installed");
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

/// Check if cuDNN is installed by looking for the library
fn is_cudnn_installed() -> bool {
  // Check common locations for cuDNN
  let cudnn_paths = [
    "/lib/x86_64-linux-gnu/libcudnn.so.9",
    "/usr/lib/x86_64-linux-gnu/libcudnn.so.9",
    "/usr/local/cuda/lib64/libcudnn.so.9",
  ];

  for path in &cudnn_paths {
    if std::path::Path::new(path).exists() {
      return true;
    }
  }

  // Also try to use ldconfig to check
  Command::new("ldconfig")
    .args(["-p"])
    .output()
    .map(|output| String::from_utf8_lossy(&output.stdout).contains("libcudnn"))
    .unwrap_or(false)
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
  println!("📋 Manual CUDA setup instructions:");
  println!("1. Install NVIDIA drivers for your GPU");
  println!("2. Install CUDA toolkit compatible with your drivers");
  println!("3. Install cuDNN library matching your CUDA version");
  println!("4. Ensure libraries are in your LD_LIBRARY_PATH");
  println!();
  println!("For detailed instructions, visit:");
  println!("  CUDA: https://docs.nvidia.com/cuda/cuda-installation-guide-linux/");
  println!("  cuDNN: https://docs.nvidia.com/deeplearning/cudnn/install-guide/");
  Ok(())
}
