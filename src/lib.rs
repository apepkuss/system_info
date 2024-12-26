use regex::Regex;
use serde::Serialize;
use std::{error::Error, process::Command};
use sysctl::Sysctl;

#[derive(Debug, Clone, Serialize)]
pub struct CPUInfo {
    manufacturer: String,
    model: String,
    cores: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct GPUInfo {
    manufacturer: String,
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory: Option<u32>, // Memory in MB
    #[serde(skip_serializing_if = "Option::is_none")]
    cores: Option<u32>, // GPU cores (if available)
}

#[derive(Debug, Clone, Serialize)]
pub struct RAMInfo {
    total: u64, // Total RAM in GB
}

#[derive(Debug, Clone, Serialize)]
pub struct OSInfo {
    name: String,
    version: String,
    architecture: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    cpu: CPUInfo,
    gpu: Vec<GPUInfo>,
    ram: RAMInfo,
    os: OSInfo,
}

/// Get system information, including CPU, GPU, RAM, and OS information.
pub fn get_system_info() -> SystemInfo {
    // CPU Information
    let cpu_info = get_cpu_info();

    // GPU Information
    let gpu_info = if cfg!(target_os = "macos") {
        get_macos_gpu_info()
    } else if cfg!(target_os = "linux") {
        get_linux_gpu_info().unwrap()
    } else {
        vec![GPUInfo {
            manufacturer: "Unknown".to_string(),
            model: "Unknown".to_string(),
            memory: None,
            cores: None,
        }]
    };

    // RAM Information
    let ram_info = get_ram_info();

    // OS Information
    let os_info = get_os_info();

    // Combine all information
    SystemInfo {
        cpu: cpu_info,
        gpu: gpu_info,
        ram: ram_info,
        os: os_info,
    }
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

/// Get GPU information for Linux.
fn get_linux_gpu_info_old() -> Result<Vec<GPUInfo>, Box<dyn std::error::Error>> {
    // 执行 lshw 命令
    let output = std::process::Command::new("lshw")
        .arg("-C")
        .arg("display") // 只获取显示设备信息
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to execute lshw: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    // 将输出转换为字符串
    let stdout = String::from_utf8(output.stdout)?;

    // 解析 vendor 和 product 信息
    let mut gpus = Vec::new();
    let mut current_vendor = None;
    let mut current_product = None;

    for line in stdout.lines() {
        // 去掉多余的空格
        let line = line.trim();

        // 匹配 vendor 信息
        if line.starts_with("vendor:") {
            current_vendor = Some(line.trim_start_matches("vendor:").trim().to_string());
        }

        // 匹配 product 信息
        if line.starts_with("product:") {
            current_product = Some(line.trim_start_matches("product:").trim().to_string());
        }

        // 如果找到 vendor 和 product，将其加入结果
        if let (Some(vendor), Some(product)) = (&current_vendor, &current_product) {
            let gpu = GPUInfo {
                manufacturer: vendor.clone(),
                model: product.clone(),
                memory: None,
                cores: None,
            };
            gpus.push(gpu);

            // reset current_vendor and current_product for next gpu
            current_vendor = None;
            current_product = None;
        }
    }

    Ok(gpus)
}

/// 检测是否安装了 nvidia-smi
fn is_nvidia_smi_installed() -> bool {
    Command::new("nvidia-smi")
        .arg("--version")
        .output()
        .map_or(false, |output| output.status.success())
}

/// 通过 nvidia-smi 获取 GPU 信息
fn get_gpu_info_from_nvidia_smi() -> Result<Vec<GPUInfo>, Box<dyn Error>> {
    let output = Command::new("nvidia-smi")
        .arg("--query-gpu=name,memory.total") // 查询 GPU 名称、显存和核心数
        .arg("--format=csv,noheader,nounits") // 格式化输出为 CSV
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to execute nvidia-smi: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    let mut gpus = Vec::new();

    for line in stdout.lines() {
        let fields: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if fields.len() == 2 {
            let gpu = GPUInfo {
                manufacturer: "NVIDIA".to_string(),
                model: fields[0].to_string(),
                memory: fields[1].parse().ok(),
                cores: None,
            };
            gpus.push(gpu);
        }
    }

    Ok(gpus)
}

/// 通过 lshw 获取 GPU 信息
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
                memory: None, // lshw 无法提供显存大小
                cores: None,  // lshw 无法提供核心数
            };
            gpus.push(gpu);
            current_vendor = None;
            current_product = None;
        }
    }

    Ok(gpus)
}

/// 综合获取 GPU 信息
fn get_linux_gpu_info() -> Result<Vec<GPUInfo>, Box<dyn Error>> {
    if is_nvidia_smi_installed() {
        println!("Using nvidia-smi to retrieve GPU information.");
        get_gpu_info_from_nvidia_smi()
    } else {
        println!("nvidia-smi not found. Falling back to lshw.");
        get_gpu_info_from_lshw()
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_get_linux_gpu_info() {
    let info = get_linux_gpu_info().unwrap();
    println!("{}", serde_json::json!(info));
}

#[test]
fn test_get_system_info() {
    let info = get_system_info();
    println!("{}", serde_json::json!(info));
}
