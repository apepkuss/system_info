use regex::Regex;
use serde::Serialize;
use serde_json::json;
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
    memory: u32, // Memory in MB
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
pub fn get_system_info() -> serde_json::Value {
    // CPU Information
    let cpu_info = get_cpu_info();

    // GPU Information
    let gpu_info = if cfg!(target_os = "macos") {
        get_macos_gpu_info()
    } else if cfg!(target_os = "linux") {
        get_linux_gpu_info()
    } else {
        vec![GPUInfo {
            manufacturer: "Unknown".to_string(),
            model: "Unknown".to_string(),
            memory: 0,
            cores: None,
        }]
    };

    // RAM Information
    let ram_info = get_ram_info();

    // OS Information
    let os_info = get_os_info();

    // Combine all information
    let system_info = SystemInfo {
        cpu: cpu_info,
        gpu: gpu_info,
        ram: ram_info,
        os: os_info,
    };

    json!(system_info)
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
    println!("{}", json!(info));
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
    println!("{}", json!(info));
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
    println!("{}", json!(info));
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
        memory: 0,
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
    println!("{}", json!(info));
}

/// Get GPU information for Linux.
pub fn get_linux_gpu_info() -> Vec<GPUInfo> {
    let mut gpus = Vec::new();

    // Try to use `lspci` to get GPU info
    let output = std::process::Command::new("lspci")
        .arg("-vnn")
        .output()
        .unwrap_or_else(|_| panic!("Failed to execute `lspci`. Is it installed?"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines = stdout.lines();

    let mut current_gpu = None;
    for line in lines {
        if line.contains("VGA compatible controller") || line.contains("3D controller") {
            // Extract GPU Model
            if let Some(index) = line.find(":") {
                let model = line[index + 1..].trim().to_string();
                current_gpu = Some(GPUInfo {
                    manufacturer: "Unknown".to_string(),
                    model,
                    memory: 0,
                    cores: None,
                });
            }
        } else if let Some(ref mut gpu) = current_gpu {
            // Extract Memory Info
            if line.contains("Memory at") && line.contains("[size=") {
                let memory_regex = Regex::new(r"\[size=(\d+)([KMGT])B\]").unwrap();
                if let Some(caps) = memory_regex.captures(line) {
                    let size: u32 = caps[1].parse().unwrap_or(0);
                    let unit = &caps[2];
                    let memory_mb = match unit {
                        "K" => size / 1024,
                        "M" => size,
                        "G" => size * 1024,
                        _ => 0,
                    };
                    gpu.memory = memory_mb;
                }
            }
            if line.contains("NVIDIA") {
                // Attempt to get core count for NVIDIA GPUs
                if let Ok(output) = std::process::Command::new("nvidia-smi")
                    .arg("--query-gpu=clocks.cores")
                    .arg("--format=csv,noheader")
                    .output()
                {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if let Ok(core_count) = stdout.trim().parse::<u32>() {
                        gpu.cores = Some(core_count);
                    }
                }
            }
            if !line.contains("Memory") {
                gpus.push(gpu.clone());
                current_gpu = None;
            }
        }
    }

    // if gpus.is_empty() {
    //     // Fallback to `nvidia-smi` if `lspci` fails
    //     if let Ok(output) = std::process::Command::new("nvidia-smi")
    //         .arg("--query-gpu=name,memory.total,clocks.cores")
    //         .arg("--format=csv,noheader")
    //         .output()
    //     {
    //         let stdout = String::from_utf8_lossy(&output.stdout);
    //         for line in stdout.lines() {
    //             let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
    //             if parts.len() >= 3 {
    //                 let model = parts[0].to_string();
    //                 let memory: u32 = parts[1]
    //                     .split_whitespace()
    //                     .next()
    //                     .unwrap_or("0")
    //                     .parse()
    //                     .unwrap_or(0);
    //                 let cores: u32 = parts[2].parse().unwrap_or(0);
    //                 gpus.push(GPUInfo {
    //                     manufacturer: "NVIDIA".to_string(),
    //                     model,
    //                     memory,
    //                     cores: Some(cores),
    //                 });
    //             }
    //         }
    //     }
    // }

    gpus
}

#[cfg(target_os = "linux")]
#[test]
fn test_get_linux_gpu_info() {
    let info = get_linux_gpu_info();
    println!("{}", json!(info));
}
