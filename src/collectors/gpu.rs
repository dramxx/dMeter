use crate::state::GpuData;

#[cfg(windows)]
use std::sync::OnceLock;

#[cfg(windows)]
static NVIDIA_DRIVER_CHECKED: OnceLock<bool> = OnceLock::new();

pub fn collect_gpu_data() -> GpuData {
    #[cfg(windows)]
    {
        // Quick check if NVIDIA driver might be present before attempting NVML init
        let driver_present = check_nvidia_driver_present();
        if !driver_present {
            return GpuData {
                available: false,
                name: "No NVIDIA GPU".to_string(),
                usage: 0.0,
                memory_used: 0,
                memory_total: 0,
                temperature: None,
                fan_speed: None,
                power_draw: None,
            };
        }

        match try_nvidia() {
            Ok(gpu) => return gpu,
            Err(e) => {
                log::warn!("GPU detection failed: {}", e);
            }
        }
    }

    default_gpu_data()
}

fn default_gpu_data() -> GpuData {
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
fn check_nvidia_driver_present() -> bool {
    *NVIDIA_DRIVER_CHECKED.get_or_init(|| {
        let possible_paths = [
            std::path::Path::new(r"C:\Windows\System32\nvml.dll"),
            std::path::Path::new(r"C:\Windows\SysWOW64\nvml.dll"),
        ];

        possible_paths.iter().any(|p| p.exists())
    })
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

    let power_draw = device.power_usage().ok().map(|p| (p / 1000) as f32);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_gpu_data_no_crash() {
        // Should not panic even if no GPU is present
        let gpu_data = collect_gpu_data();
        
        // Should return valid GpuData struct
        assert!(gpu_data.usage >= 0.0);
        assert!(gpu_data.memory_used <= gpu_data.memory_total);
    }

    #[test]
    fn test_default_gpu_data() {
        let gpu_data = default_gpu_data();
        
        // Verify default values
        assert!(!gpu_data.available);
        assert_eq!(gpu_data.name, "No GPU / Not detected");
        assert_eq!(gpu_data.usage, 0.0);
        assert_eq!(gpu_data.memory_used, 0);
        assert_eq!(gpu_data.memory_total, 0);
        assert!(gpu_data.temperature.is_none());
        assert!(gpu_data.fan_speed.is_none());
        assert!(gpu_data.power_draw.is_none());
    }

    #[test]
    fn test_gpu_data_available_flag() {
        let gpu_data = collect_gpu_data();
        
        // If GPU is not available, all optional fields should be None or 0
        if !gpu_data.available {
            assert_eq!(gpu_data.usage, 0.0);
            assert_eq!(gpu_data.memory_used, 0);
            assert_eq!(gpu_data.memory_total, 0);
        } else {
            // If GPU is available, should have valid data
            assert!(!gpu_data.name.is_empty());
            assert!(gpu_data.usage >= 0.0 && gpu_data.usage <= 100.0);
        }
    }

    #[test]
    fn test_multiple_gpu_collections() {
        // Collect GPU data multiple times - should not crash or leak
        for _ in 0..10 {
            let gpu_data = collect_gpu_data();
            assert!(gpu_data.usage >= 0.0);
        }
    }

    #[test]
    fn test_gpu_data_memory_consistency() {
        let gpu_data = collect_gpu_data();
        
        // Memory used should never exceed memory total
        assert!(gpu_data.memory_used <= gpu_data.memory_total);
        
        // If no GPU, both should be 0
        if !gpu_data.available {
            assert_eq!(gpu_data.memory_used, 0);
            assert_eq!(gpu_data.memory_total, 0);
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_nvidia_driver_check_caching() {
        // First call
        let result1 = check_nvidia_driver_present();
        
        // Second call should return cached result
        let result2 = check_nvidia_driver_present();
        
        // Results should be consistent
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_gpu_optional_fields() {
        let gpu_data = collect_gpu_data();
        
        // Optional fields should be None if GPU is not available
        if !gpu_data.available {
            assert!(gpu_data.temperature.is_none());
            assert!(gpu_data.fan_speed.is_none());
            assert!(gpu_data.power_draw.is_none());
        }
        
        // If temperature is present, it should be reasonable
        if let Some(temp) = gpu_data.temperature {
            assert!(temp >= 0.0 && temp <= 150.0); // Reasonable GPU temp range
        }
        
        // If fan speed is present, it should be 0-100%
        if let Some(fan) = gpu_data.fan_speed {
            assert!(fan <= 100);
        }
        
        // If power draw is present, it should be positive
        if let Some(power) = gpu_data.power_draw {
            assert!(power >= 0.0);
        }
    }
}
