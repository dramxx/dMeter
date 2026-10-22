use crate::state::GpuData;

/// Holds the NVML handle so the library is loaded once instead of on every collection.
pub struct GpuCollector {
    #[cfg(windows)]
    nvml: Option<nvml_wrapper::Nvml>,
}

impl GpuCollector {
    pub fn new() -> Self {
        Self {
            // Fails gracefully (no nvml.dll) on machines without an NVIDIA driver
            #[cfg(windows)]
            nvml: nvml_wrapper::Nvml::init().ok(),
        }
    }

    pub fn collect(&self) -> GpuData {
        #[cfg(windows)]
        let result = match &self.nvml {
            Some(nvml) => try_nvidia(nvml),
            None => Err("NVML not available".to_string()),
        };

        #[cfg(not(windows))]
        let result = try_nvidia();

        result.unwrap_or_else(|_| default_gpu_data())
    }
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
fn try_nvidia(nvml: &nvml_wrapper::Nvml) -> Result<GpuData, String> {
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
                let memory_used = parts[2].parse::<u64>().unwrap_or(0) * 1024 * 1024;
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

    if let Ok(name) = fs::read_to_string("/sys/class/drm/card0/device/product_name") {
        let name = name.trim().to_string();
        let temperature = read_amdgpu_temp();
        let memory_total = read_amdgpu_memory_total();
        
        return Ok(GpuData {
            available: true,
            name,
            usage: 0.0,
            memory_used: 0,
            memory_total,
            temperature,
            fan_speed: None,
            power_draw: None,
        });
    }

    Err("No GPU detected on Linux".to_string())
}

#[cfg(not(windows))]
fn read_amdgpu_temp() -> Option<f32> {
    use std::fs;
    
    for hwmon_idx in 0..10 {
        let temp_path = format!("/sys/class/hwmon/hwmon{}/temp1_input", hwmon_idx);
        if let Ok(content) = fs::read_to_string(&temp_path) {
            if let Ok(temp_millidegrees) = content.trim().parse::<i32>() {
                return Some(temp_millidegrees as f32 / 1000.0);
            }
        }
    }
    None
}

#[cfg(not(windows))]
fn read_amdgpu_memory_total() -> u64 {
    use std::fs;
    
    if let Ok(content) = fs::read_to_string("/sys/class/drm/card0/device/mem_info_vram_total") {
        if let Ok(bytes) = content.trim().parse::<u64>() {
            return bytes;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_gpu_data_no_crash() {
        // Should not panic even if no GPU is present
        let gpu_data = GpuCollector::new().collect();
        
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
        let gpu_data = GpuCollector::new().collect();
        
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
        let collector = GpuCollector::new();
        for _ in 0..10 {
            let gpu_data = collector.collect();
            assert!(gpu_data.usage >= 0.0);
        }
    }

    #[test]
    fn test_gpu_data_memory_consistency() {
        let gpu_data = GpuCollector::new().collect();
        
        // Memory used should never exceed memory total
        assert!(gpu_data.memory_used <= gpu_data.memory_total);
        
        // If no GPU, both should be 0
        if !gpu_data.available {
            assert_eq!(gpu_data.memory_used, 0);
            assert_eq!(gpu_data.memory_total, 0);
        }
    }

    #[test]
    fn test_gpu_optional_fields() {
        let gpu_data = GpuCollector::new().collect();
        
        // Optional fields should be None if GPU is not available
        if !gpu_data.available {
            assert!(gpu_data.temperature.is_none());
            assert!(gpu_data.fan_speed.is_none());
            assert!(gpu_data.power_draw.is_none());
        }
        
        // If temperature is present, it should be reasonable
        if let Some(temp) = gpu_data.temperature {
            assert!((0.0..=150.0).contains(&temp)); // Reasonable GPU temp range
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
