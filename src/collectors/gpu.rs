use crate::state::GpuData;

pub fn collect_gpu_data() -> GpuData {
    #[cfg(windows)]
    {
        match try_nvidia() {
            Ok(gpu) => return gpu,
            Err(e) => {
                log::warn!("GPU detection failed: {}", e);
            }
        }
    }

    GpuData {
        available: false,
        name: "No GPU / Not detected".to_string(),
        usage: 0.0,
        memory_used: 0,
        memory_total: 0,
        temperature: None,
        fan_speed: None,
        power_draw: None,
    }
}

#[cfg(windows)]
fn try_nvidia() -> Result<GpuData, String> {
    use nvml_wrapper::Nvml;

    let nvml = Nvml::init().map_err(|e| format!("NVML init failed: {}", e))?;
    let device = nvml
        .device_by_index(0)
        .map_err(|e| format!("No GPU found: {}", e))?;

    let name = device
        .name()
        .map_err(|e| format!("Failed to get GPU name: {}", e))?
        .to_string();
    let usage = device
        .utilization_rates()
        .map_err(|e| format!("Failed to get utilization: {}", e))?
        .gpu as f32;

    let (memory_used, memory_total) = {
        let mem = device
            .memory_info()
            .map_err(|e| format!("Failed to get memory info: {}", e))?;
        (mem.used, mem.total)
    };

    let temperature = device
        .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
        .ok()
        .map(|t| t as f32);

    let fan_speed = device.fan_speed(0).ok();

    let power_draw = device.power_usage().ok().map(|p| p / 1000);

    Ok(GpuData {
        available: true,
        name,
        usage,
        memory_used,
        memory_total,
        temperature,
        fan_speed,
        power_draw,
    })
}

#[cfg(not(windows))]
fn try_nvidia() -> Result<GpuData, String> {
    use std::fs;
    use std::process::Command;

    // Try nvidia-smi command (most reliable on Linux)
    let output = Command::new("nvidia-smi")
        .args(&[
            "--query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu",
            "--format=csv,noheader,nounits",
        ])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let data = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = data.trim().split(',').map(|s| s.trim()).collect();

            if parts.len() >= 5 {
                let name = parts[0].to_string();
                let usage = parts[1].parse::<f32>().unwrap_or(0.0);
                let memory_used = parts[2].parse::<u64>().unwrap_or(0) * 1024 * 1024; // MB to bytes
                let memory_total = parts[3].parse::<u64>().unwrap_or(0) * 1024 * 1024;
                let temperature = parts[4].parse::<f32>().ok();

                return Ok(GpuData {
                    available: true,
                    name,
                    usage,
                    memory_used,
                    memory_total,
                    temperature,
                    fan_speed: None,
                    power_draw: None,
                });
            }
        }
    }

    // Fallback: Try reading from sysfs (AMD/Intel)
    if let Ok(name) = fs::read_to_string("/sys/class/drm/card0/device/product_name") {
        return Ok(GpuData {
            available: true,
            name: name.trim().to_string(),
            usage: 0.0,
            memory_used: 0,
            memory_total: 0,
            temperature: None,
            fan_speed: None,
            power_draw: None,
        });
    }

    Err("No GPU detected on Linux".to_string())
}
