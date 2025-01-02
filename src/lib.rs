//! A library for getting system information, including CPU, GPU, RAM, and OS information.
//!
//! This library provides a set of APIs to retrieve system information, including CPU, GPU, RAM, and OS information.
//! It supports both macOS and Linux systems.
//!
//! # Usage
//!
//! ```rust
//! use system_info_lite::{get_system_info, get_cpu_info, get_ram_info, get_os_info};
//! use serde_json::json;
//!
//! let info = get_system_info();
//! println!("{}", json!(info));
//! ```

use regex::Regex;
use serde::Serialize;
use std::{error::Error, process::Command};
use sysctl::Sysctl;

/// CPU information.
#[derive(Debug, Clone, Serialize)]
pub struct CPUInfo {
    /// CPU manufacturer.
    pub manufacturer: String,
    /// CPU model.
    pub model: String,
    /// Number of CPU cores.
    pub cores: usize,
}

/// GPU information.
#[derive(Debug, Clone, Serialize)]
pub struct GPUInfo {
    /// GPU manufacturer.
    pub manufacturer: String,
    /// GPU model.
    pub model: String,
    /// Memory in MB.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<u32>,
    /// GPU cores (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cores: Option<u32>, // GPU cores (if available)
}

/// RAM information.
#[derive(Debug, Clone, Serialize)]
pub struct RAMInfo {
    /// Total RAM in GB.
    pub total: u64,
}

/// OS information.
#[derive(Debug, Clone, Serialize)]
pub struct OSInfo {
    /// OS name.
    pub name: String,
    /// OS version.
    pub version: String,
    /// OS architecture.
    pub architecture: String,
}

/// System information.
#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    /// CPU information.
    pub cpu: CPUInfo,
    /// GPU information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu: Option<Vec<GPUInfo>>,
    /// RAM information.
    pub ram: RAMInfo,
    /// OS information.
    pub os: OSInfo,
}

/// Get system information, including CPU, GPU, RAM, and OS information.
pub fn get_system_info() -> Result<SystemInfo, Box<dyn Error>> {
    // CPU Information
    let cpu_info = get_cpu_info();

    // GPU Information
    let gpu_info = if cfg!(target_os = "macos") {
        get_macos_gpu_info()
    } else if cfg!(target_os = "linux") {
        get_linux_gpu_info()?
    } else {
        vec![]
    };
    let gpu_info = if gpu_info.is_empty() {
        None
    } else {
        Some(gpu_info)
    };

    // RAM Information
    let ram_info = get_ram_info();

    // OS Information
    let os_info = get_os_info();

    // Combine all information
    Ok(SystemInfo {
        cpu: cpu_info,
        gpu: gpu_info,
        ram: ram_info,
        os: os_info,
    })
}

/// Get CPU information.
pub fn get_cpu_info() -> CPUInfo {
    // CPU Information
    let manufacturer = if cfg!(target_os = "macos") {
        "Apple".to_string()
    } else {
        "Intel/AMD".to_string() // Simplified for Linux
    };

    let model = if cfg!(target_os = "macos") {
        sysctl::Ctl::new("machdep.cpu.brand_string")
            .and_then(|ctl| ctl.value_string())
            .unwrap_or_else(|_| "Unknown".to_string())
    } else {
        "Generic Model".to_string() // Placeholder for Linux
    };

    // Get the number of CPU cores using std::thread
    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    CPUInfo {
        manufacturer,
        model,
        cores,
    }
}

#[test]
fn test_get_cpu_info() {
    let info = get_cpu_info();
    println!("{}", serde_json::json!(info));
}

/// Get RAM information.
pub fn get_ram_info() -> RAMInfo {
    if cfg!(target_os = "macos") {
        // macOS: Use sysctl to get memory info
        let total_memory_kb = sysctl::Ctl::new("hw.memsize")
            .and_then(|ctl| ctl.value_string())
            .and_then(|value| Ok(value.parse::<u64>().unwrap_or(0)))
            .unwrap_or(0);

        RAMInfo {
            total: total_memory_kb / 1024 / 1024 / 1024, // Convert bytes to GB
        }
    } else if cfg!(target_os = "linux") {
        // Linux: Parse /proc/meminfo
        let meminfo = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
        let total_line = meminfo
            .lines()
            .find(|line| line.starts_with("MemTotal"))
            .unwrap_or("MemTotal: 0 kB");

        let total_kb: u64 = total_line
            .split_whitespace()
            .nth(1)
            .and_then(|val| val.parse::<u64>().ok())
            .unwrap_or(0);

        RAMInfo {
            total: total_kb / 1024 / 1024, // Convert KB to GB
        }
    } else {
        // Default to 0 for unsupported systems
        RAMInfo { total: 0 }
    }
}

#[test]
pub fn test_get_ram_info() {
    let info = get_ram_info();
    println!("{}", serde_json::json!(info));
}

/// Get OS information.
pub fn get_os_info() -> OSInfo {
    if cfg!(target_os = "macos") {
        let os_name = "macOS".to_string();
        let version = sysctl::Ctl::new("kern.osrelease")
            .and_then(|ctl| ctl.value_string())
            .unwrap_or_else(|_| "Unknown".to_string());
        let architecture = sysctl::Ctl::new("hw.machine")
            .and_then(|ctl| ctl.value_string())
            .unwrap_or_else(|_| "Unknown".to_string());

        OSInfo {
            name: os_name,
            version,
            architecture,
        }
    } else if cfg!(target_os = "linux") {
        let os_name = std::fs::read_to_string("/etc/os-release")
            .unwrap_or_default()
            .lines()
            .find(|line| line.starts_with("PRETTY_NAME"))
            .and_then(|line| line.split('=').nth(1))
            .map(|name| name.replace("\"", ""))
            .unwrap_or_else(|| "Linux".to_string());

        let version =
            std::fs::read_to_string("/proc/version").unwrap_or_else(|_| "Unknown".to_string());
        let architecture = std::env::consts::ARCH.to_string();

        OSInfo {
            name: os_name,
            version,
            architecture,
        }
    } else {
        // Default values for unsupported systems
        OSInfo {
            name: "Unknown".to_string(),
            version: "Unknown".to_string(),
            architecture: "Unknown".to_string(),
        }
    }
}

#[test]
fn test_get_os_info() {
    let info = get_os_info();
    println!("{}", serde_json::json!(info));
}

/// Get GPU information for macOS.
pub fn get_macos_gpu_info() -> Vec<GPUInfo> {
    // Run system_profiler to get GPU details
    let output = std::process::Command::new("system_profiler")
        .arg("SPDisplaysDataType")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    let model_regex = Regex::new(r"Chipset Model: (.+)").unwrap();
    let manufacturer_regex = Regex::new(r"Vendor: (.+)").unwrap();
    let gpu_cores_regex = Regex::new(r"Total Number of Cores: (\d+)").unwrap();

    let mut gpu = GPUInfo {
        manufacturer: "Unknown".to_string(),
        model: "Unknown".to_string(),
        memory: None,
        cores: None,
    };
    for line in stdout.lines() {
        if let Some(caps) = model_regex.captures(line) {
            gpu.model = caps[1].trim().to_string();
        } else if let Some(caps) = gpu_cores_regex.captures(line) {
            gpu.cores = Some(caps[1].trim().parse::<u32>().unwrap_or(0));
        } else if let Some(caps) = manufacturer_regex.captures(line) {
            gpu.manufacturer = caps[1].trim().to_string();
        }
    }

    vec![gpu]
}

#[cfg(target_os = "macos")]
#[test]
fn test_get_macos_gpu_info() {
    let info = get_macos_gpu_info();
    println!("{}", serde_json::json!(info));
}

/// Check if nvidia-smi is installed
fn is_nvidia_smi_installed() -> bool {
    Command::new("nvidia-smi")
        .arg("--version")
        .output()
        .map_or(false, |output| output.status.success())
}

/// Get GPU information via nvidia-smi
fn get_gpu_info_from_nvidia_smi() -> Result<Vec<GPUInfo>, Box<dyn Error>> {
    let mut gpus = Vec::new();

    if wasmedge_on_gpu() {
        let output = Command::new("nvidia-smi")
            .arg("--query-gpu=name,memory.total") // Query GPU name, memory and core count
            .arg("--format=csv,noheader,nounits") // Format output as CSV
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "Failed to execute nvidia-smi: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        let stdout = String::from_utf8(output.stdout)?;

        for line in stdout.lines() {
            let fields: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if fields.len() == 2 {
                // vram in GB
                let memory = match fields[1].parse::<f32>() {
                    Ok(memory) => Some(memory / 1024.0),
                    Err(_) => None,
                };

                let gpu = GPUInfo {
                    manufacturer: "NVIDIA".to_string(),
                    model: fields[0].to_string(),
                    memory: memory.map(|m| m as u32),
                    cores: None,
                };
                gpus.push(gpu);
            }
        }
    }

    Ok(gpus)
}

fn wasmedge_on_gpu() -> bool {
    // Execute nvidia-smi command
    let output = Command::new("nvidia-smi")
        .output()
        .expect("Failed to execute nvidia-smi");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Filter lines related to wasmedge
    let wasmedge_lines: Vec<&str> = stdout
        .lines()
        .filter(|line| line.contains("wasmedge"))
        .collect();

    if wasmedge_lines.is_empty() {
        false
    } else {
        true
    }
}

/// Get GPU information via lshw
fn get_gpu_info_from_lshw() -> Result<Vec<GPUInfo>, Box<dyn Error>> {
    let output = Command::new("lshw").arg("-C").arg("display").output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to execute lshw: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    let mut gpus = Vec::new();
    let mut current_vendor = None;
    let mut current_product = None;

    for line in stdout.lines() {
        let line = line.trim();

        if line.starts_with("vendor:") {
            current_vendor = Some(line.trim_start_matches("vendor:").trim().to_string());
        }

        if line.starts_with("product:") {
            current_product = Some(line.trim_start_matches("product:").trim().to_string());
        }

        if let (Some(vendor), Some(product)) = (&current_vendor, &current_product) {
            let gpu = GPUInfo {
                manufacturer: vendor.clone(),
                model: product.clone(),
                memory: None, // lshw cannot provide memory size
                cores: None,  // lshw cannot provide core count
            };
            gpus.push(gpu);
            current_vendor = None;
            current_product = None;
        }
    }

    Ok(gpus)
}

/// Get GPU information for Linux.
pub fn get_linux_gpu_info() -> Result<Vec<GPUInfo>, Box<dyn Error>> {
    if is_nvidia_smi_installed() {
        get_gpu_info_from_nvidia_smi()
    } else {
        get_gpu_info_from_lshw()
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_get_linux_gpu_info() {
    let result = get_linux_gpu_info();
    match result {
        Ok(info) => println!("{}", serde_json::json!(info)),
        Err(e) => println!("Error: {}", e),
    }
}

#[test]
fn test_get_system_info() {
    let result = get_system_info();
    match result {
        Ok(info) => println!("{}", serde_json::json!(info)),
        Err(e) => println!("Error: {}", e),
    }
}
