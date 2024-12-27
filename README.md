# System-info-lite: A library for Getting System Information

In `system-info-lite`, a group of APIs are defined to get some system information, including CPU, GPU, RAM, and OS information.

## Usage

```rust
use system_info_lite::{get_system_info, get_cpu_info, get_ram_info, get_os_info};

// Get system information, including CPU, GPU, RAM, and OS information.
let info = get_system_info();
println!("{:#?}", info);

// Get CPU information.
let cpu_info = get_cpu_info();
println!("{:#?}", cpu_info);

// Get RAM information.
let ram_info = get_ram_info();
println!("{:#?}", ram_info);

// Get GPU information.
let gpu_info = get_gpu_info();
println!("{:#?}", gpu_info);

// Get OS information.
let os_info = get_os_info();
println!("{:#?}", os_info);
```
