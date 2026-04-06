use crate::state::{CpuData, DiskData, MemoryData, NetworkData, SystemData, SystemInfoData};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use sysinfo::{Disks, Networks, System};

pub struct SystemCollector {
    sys: Mutex<System>,
    networks: Mutex<Networks>,
    disks: Mutex<Disks>,
    last_network_rx: Mutex<u64>,
    last_network_tx: Mutex<u64>,
    #[cfg_attr(windows, allow(dead_code))]
    last_disk_read: Mutex<u64>,
    #[cfg_attr(windows, allow(dead_code))]
    last_disk_write: Mutex<u64>,
    memory_receiver: Receiver<(u64, u64, u64)>, // (commit_total, commit_used, cached)
    cached_memory_info: Mutex<(u64, u64, u64)>,
    memory_thread_handle: Option<JoinHandle<()>>,
    shutdown_signal: Arc<AtomicBool>,
    // CPU temperature TTL cache: only call the slow subprocess every 10s
    cached_cpu_temp: Mutex<Option<f32>>,
    cpu_temp_updated: Mutex<std::time::Instant>,
    // CPU fan speed TTL cache: only call the slow subprocess every 10s
    cached_cpu_fan: Mutex<Option<u32>>,
    cpu_fan_updated: Mutex<std::time::Instant>,
    // CPU power draw TTL cache: only call the slow subprocess every 10s
    cached_cpu_power: Mutex<Option<f32>>,
    cpu_power_updated: Mutex<std::time::Instant>,
    // Local IP cache: doesn't change at runtime
    cached_ip: Mutex<Option<String>>,
}

impl Drop for SystemCollector {
    fn drop(&mut self) {
        // Signal the background thread to shutdown
        self.shutdown_signal.store(true, Ordering::Relaxed);
        
        // Wait for the thread to finish (with timeout)
        if let Some(handle) = self.memory_thread_handle.take() {
            let _ = handle.join();
        }
    }
}

impl SystemCollector {
    pub fn new() -> Self {
        let sys = System::new_all();
        let networks = Networks::new_with_refreshed_list();
        let disks = Disks::new_with_refreshed_list();

        let (tx, rx) = Self::get_network_totals(&networks);
        #[cfg(not(windows))]
        let (read, write) = Self::get_disk_totals_linux(&disks);
        #[cfg(windows)]
        let (read, write) = (0u64, 0u64); // Windows uses typeperf rate directly, no cumulative init needed

        // Create background thread for memory collection with shutdown signal
        let (memory_tx, memory_rx) = mpsc::channel();
        let shutdown_signal = Arc::new(AtomicBool::new(false));
        let shutdown_clone = Arc::clone(&shutdown_signal);
        
        let memory_thread_handle = thread::spawn(move || {
            loop {
                if shutdown_clone.load(Ordering::Relaxed) {
                    break;
                }
                
                let memory_info = crate::collectors::memory_info::collect_extended_memory_info();
                if memory_tx.send(memory_info).is_err() {
                    break;
                }
                
                // Sleep in small increments to check shutdown signal more frequently
                for _ in 0..20 {
                    if shutdown_clone.load(Ordering::Relaxed) {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        });

        Self {
            sys: Mutex::new(sys),
            networks: Mutex::new(networks),
            disks: Mutex::new(disks),
            last_network_rx: Mutex::new(rx),
            last_network_tx: Mutex::new(tx),
            last_disk_read: Mutex::new(read),
            last_disk_write: Mutex::new(write),
            memory_receiver: memory_rx,
            cached_memory_info: Mutex::new((0, 0, 0)),
            memory_thread_handle: Some(memory_thread_handle),
            shutdown_signal,
            cached_cpu_temp: Mutex::new(None),
            cpu_temp_updated: Mutex::new(Instant::now() - Duration::from_secs(60)),
            cached_cpu_fan: Mutex::new(None),
            cpu_fan_updated: Mutex::new(Instant::now() - Duration::from_secs(60)),
            cached_cpu_power: Mutex::new(None),
            cpu_power_updated: Mutex::new(Instant::now() - Duration::from_secs(60)),
            cached_ip: Mutex::new(None),
        }
    }

    fn get_network_totals(networks: &Networks) -> (u64, u64) {
        let mut total_tx = 0u64;
        let mut total_rx = 0u64;

        for (_, data) in networks.iter() {
            total_tx += data.transmitted();
            total_rx += data.received();
        }

        (total_tx, total_rx)
    }

    #[cfg(not(windows))]
    fn get_disk_totals_linux(_disks: &Disks) -> (u64, u64) {
        use std::fs;

        let mut total_read = 0u64;
        let mut total_write = 0u64;

        let sector_size = Self::get_sector_size_linux();

        if let Ok(content) = fs::read_to_string("/proc/diskstats") {
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 10 {
                    if let (Ok(read_sectors), Ok(write_sectors)) =
                        (parts[5].parse::<u64>(), parts[9].parse::<u64>())
                    {
                        total_read += read_sectors * sector_size;
                        total_write += write_sectors * sector_size;
                    }
                }
            }
        }

        (total_read, total_write)
    }

    #[cfg(not(windows))]
    fn get_sector_size_linux() -> u64 {
        use std::fs;

        let disk_names = ["sda", "nvme0n1", "vda", "sdb", "sdc"];
        
        for disk in disk_names.iter() {
            let path = format!("/sys/block/{}/queue/hw_sector_size", disk);
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(size) = content.trim().parse::<u64>() {
                    return size;
                }
            }
        }
        4096
    }

    #[cfg(windows)]
    fn get_disk_rate_windows() -> (u64, u64) {
        use std::process::Command;

        // Use -si 0.01 -sc 1: sample interval 10ms, 1 sample - returns in ~50ms instead of ~1s
        // The "Disk Read Bytes/sec" counter is a rate maintained by the OS, so 1 sample is sufficient
        if let Ok(output) = Command::new("typeperf")
            .args([
                "\"\\PhysicalDisk(_Total)\\Disk Read Bytes/sec\",\"\\PhysicalDisk(_Total)\\Disk Write Bytes/sec\"",
                "-si", "0.01",
                "-sc", "1",
            ])
            .output()
        {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                // Skip header line, take last data line
                for line in output_str.lines().rev() {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() >= 3 {
                        if let (Ok(read), Ok(write)) = (
                            parts[1].trim_matches('"').trim().parse::<f64>(),
                            parts[2].trim_matches('"').trim().parse::<f64>(),
                        ) {
                            if read >= 0.0 {
                                return (read as u64, write as u64);
                            }
                        }
                    }
                }
            }
        }

        (0, 0)
    }

    pub fn collect(&mut self, collect_processes: bool) -> SystemData {
        {
            let mut sys = self.sys.lock().unwrap_or_else(|e| {
                log::error!("System mutex poisoned, recovering: {}", e);
                e.into_inner()
            });
            // Use targeted refresh instead of refresh_all() for better performance
            sys.refresh_cpu_all();
            sys.refresh_memory();
            // Only refresh processes if we need them
            if collect_processes {
                sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
                // Second refresh needed for CPU usage data to be populated
                sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
            }
        }

        {
            let mut networks = self.networks.lock().unwrap_or_else(|e| {
                log::error!("Networks mutex poisoned, recovering: {}", e);
                e.into_inner()
            });
            networks.refresh();
        }

        {
            let mut disks = self.disks.lock().unwrap_or_else(|e| {
                log::error!("Disks mutex poisoned, recovering: {}", e);
                e.into_inner()
            });
            disks.refresh();
        }

        let cpu = self.collect_cpu();
        let memory = self.collect_memory();
        let network = self.collect_network();
        let disks = self.collect_disks();
        let disk_io = self.collect_disk_io();
        let system = self.collect_system();
        let processes = if collect_processes {
            self.collect_processes()
        } else {
            Vec::new()
        };

        SystemData {
            cpu,
            gpu: crate::collectors::gpu::collect_gpu_data(),
            memory,
            network,
            disks,
            disk_io,
            system,
            processes,
        }
    }

    fn collect_cpu(&self) -> CpuData {
        let sys = self.sys.lock().unwrap_or_else(|e| e.into_inner());
        let cpus = sys.cpus();

        let name = cpus
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string());

        let usage = sys.global_cpu_usage();
        let frequency = cpus.first().map(|c| c.frequency()).unwrap_or(0);

        drop(sys); // Release mutex before slow subprocess calls

        // TTL cache: only call subprocesses every 10 seconds to avoid blocking
        let temperature = {
            let mut cached = self.cached_cpu_temp.lock().unwrap_or_else(|e| e.into_inner());
            let mut updated = self.cpu_temp_updated.lock().unwrap_or_else(|e| e.into_inner());
            if updated.elapsed() >= Duration::from_secs(10) {
                *cached = get_cpu_temperature();
                *updated = Instant::now();
            }
            *cached
        };

        let fan_speed = {
            let mut cached = self.cached_cpu_fan.lock().unwrap_or_else(|e| e.into_inner());
            let mut updated = self.cpu_fan_updated.lock().unwrap_or_else(|e| e.into_inner());
            if updated.elapsed() >= Duration::from_secs(10) {
                *cached = get_cpu_fan_speed();
                *updated = Instant::now();
            }
            *cached
        };

        let power_draw = {
            let mut cached = self.cached_cpu_power.lock().unwrap_or_else(|e| e.into_inner());
            let mut updated = self.cpu_power_updated.lock().unwrap_or_else(|e| e.into_inner());
            if updated.elapsed() >= Duration::from_secs(10) {
                *cached = get_cpu_power_draw();
                *updated = Instant::now();
            }
            *cached
        };

        CpuData {
            name,
            usage,
            temperature,
            frequency: frequency as f32,
            fan_speed,
            power_draw,
        }
    }

    fn collect_memory(&self) -> MemoryData {
        let sys = self.sys.lock().unwrap_or_else(|e| e.into_inner());

        // Check for new memory data from background thread (non-blocking)
        let mut cached_info = self
            .cached_memory_info
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        // Try to receive new data without blocking
        while let Ok(memory_info) = self.memory_receiver.try_recv() {
            *cached_info = memory_info;
        }

        let (commit_total, commit_used, cached) = *cached_info;

        MemoryData {
            total: sys.total_memory(),
            used: sys.used_memory(),
            swap_total: sys.total_swap(),
            swap_used: sys.used_swap(),
            commit_total,
            commit_used,
            cached,
        }
    }

    fn collect_network(&self) -> NetworkData {
        let networks = self.networks.lock().unwrap_or_else(|e| e.into_inner());
        let (total_tx, total_rx) = Self::get_network_totals(&networks);

        let mut last_tx = self.last_network_tx.lock().unwrap_or_else(|e| e.into_inner());
        let mut last_rx = self.last_network_rx.lock().unwrap_or_else(|e| e.into_inner());

        let upload_speed = total_tx.saturating_sub(*last_tx);
        let download_speed = total_rx.saturating_sub(*last_rx);

        let final_upload = if upload_speed > u64::MAX as u64 { 0 } else { upload_speed };
        let final_download = if download_speed > u64::MAX as u64 { 0 } else { download_speed };

        *last_tx = total_tx;
        *last_rx = total_rx;

        let mut adapter_name = String::new();
        for (name, data) in networks.iter() {
            if (data.received() > 0 || data.transmitted() > 0) && adapter_name.is_empty() {
                adapter_name = name.clone();
            }
        }

        // Cache local IP - it doesn't change at runtime
        let ip_address = {
            let mut cached = self.cached_ip.lock().unwrap_or_else(|e| e.into_inner());
            if cached.is_none() {
                *cached = get_local_ip().ok();
            }
            cached.clone().unwrap_or_default()
        };

        NetworkData {
            adapter_name,
            ip_address,
            upload_speed: final_upload as f64,
            download_speed: final_download as f64,
        }
    }

    fn collect_disk_io(&self) -> crate::state::DiskIOData {
        #[cfg(windows)]
        {
            // typeperf returns bytes/sec directly - use rate without computing delta
            let (read_rate, write_rate) = Self::get_disk_rate_windows();
            crate::state::DiskIOData {
                read_speed: read_rate as f64,
                write_speed: write_rate as f64,
            }
        }

        #[cfg(not(windows))]
        {
            let disks = self.disks.lock().unwrap_or_else(|e| e.into_inner());
            let (total_read, total_write) = Self::get_disk_totals_linux(&disks);

            let mut last_read = self.last_disk_read.lock().unwrap_or_else(|e| e.into_inner());
            let mut last_write = self.last_disk_write.lock().unwrap_or_else(|e| e.into_inner());

            let read_speed = total_read.saturating_sub(*last_read);
            let write_speed = total_write.saturating_sub(*last_write);

            *last_read = total_read;
            *last_write = total_write;

            crate::state::DiskIOData {
                read_speed: read_speed as f64,
                write_speed: write_speed as f64,
            }
        }
    }

    fn collect_disks(&self) -> Vec<DiskData> {
        let disks = self.disks.lock().unwrap_or_else(|e| e.into_inner());
        disks
            .iter()
            .map(|disk| DiskData {
                name: disk.name().to_string_lossy().to_string(),
                mount_point: disk.mount_point().to_string_lossy().to_string(),
                total: disk.total_space(),
                used: disk.total_space().saturating_sub(disk.available_space()),
                filesystem: disk.file_system().to_string_lossy().to_string(),
            })
            .collect()
    }

    fn collect_system(&self) -> SystemInfoData {
        // Note: System::name/os_version/host_name/uptime are static methods, no mutex needed
        SystemInfoData {
            os_name: System::name().unwrap_or_else(|| "Unknown".to_string()),
            os_version: System::os_version().unwrap_or_else(|| "Unknown".to_string()),
            hostname: System::host_name().unwrap_or_else(|| "Unknown".to_string()),
            uptime: System::uptime(),
        }
    }

    fn collect_processes(&self) -> Vec<crate::state::ProcessData> {
        let sys = self.sys.lock().unwrap_or_else(|e| e.into_inner());
        let total_memory = sys.total_memory();

        let mut processes: Vec<crate::state::ProcessData> = sys
            .processes()
            .values()
            .map(|process| {
                let memory_usage = if total_memory > 0 {
                    (process.memory() as f32 / total_memory as f32) * 100.0
                } else {
                    0.0
                };

                crate::state::ProcessData {
                    name: process.name().to_string_lossy().to_string(),
                    cpu_usage: process.cpu_usage(),
                    memory_usage,
                }
            })
            .collect();

        // Sort by combined resource usage (CPU% + Memory%) descending
        processes.sort_by(|a, b| {
            let score_a = a.cpu_usage + a.memory_usage;
            let score_b = b.cpu_usage + b.memory_usage;
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit to top 100 processes for performance
        processes.truncate(100);

        processes
    }
}

#[cfg(windows)]
fn get_cpu_temperature() -> Option<f32> {
    use std::process::Command;

    // Method 1: Try MSAcpi_ThermalZoneTemperature (works on some laptops)
    if let Ok(output) = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-WmiObject MSAcpi_ThermalZoneTemperature -Namespace 'root/wmi' -ErrorAction SilentlyContinue | Select-Object -First 1 CurrentTemperature | ForEach-Object {($_.CurrentTemperature - 2732) / 10.0}"
        ])
        .output()
    {
        if output.status.success() {
            let temp_str = String::from_utf8_lossy(&output.stdout);
            let trimmed = temp_str.trim();
            if !trimmed.is_empty() {
                if let Ok(temp) = trimmed.parse::<f32>() {
                    if temp > 0.0 && temp < 150.0 {
                        return Some(temp);
                    }
                }
            }
        }
    }

    // Method 2: Try Win32_TemperatureProbe
    if let Ok(output) = Command::new("wmic")
        .args(["path", "Win32_TemperatureProbe", "get", "CurrentReading", "/value"])
        .output()
    {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.starts_with("CurrentReading=") {
                    if let Some(value) = line.split('=').nth(1) {
                        if let Ok(temp_tenths) = value.trim().parse::<i32>() {
                            let temp = temp_tenths as f32 / 10.0;
                            if temp > 0.0 && temp < 150.0 {
                                return Some(temp);
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

#[cfg(windows)]
fn get_cpu_fan_speed() -> Option<u32> {
    use std::process::Command;

    // Method 1: Try Win32_Fan DesiredSpeed
    if let Ok(output) = Command::new("wmic")
        .args(["path", "Win32_Fan", "get", "DesiredSpeed", "/value"])
        .output()
    {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.starts_with("DesiredSpeed=") {
                    if let Some(value) = line.split('=').nth(1) {
                        let trimmed = value.trim();
                        if !trimmed.is_empty() {
                            if let Ok(speed) = trimmed.parse::<u32>() {
                                if speed > 0 {
                                    let percentage = ((speed as f32 / 3000.0) * 100.0).min(100.0) as u32;
                                    return Some(percentage);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Method 2: Try Win32_Fan ActiveCooling (boolean)
    if let Ok(output) = Command::new("wmic")
        .args(["path", "Win32_Fan", "get", "ActiveCooling", "/value"])
        .output()
    {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.starts_with("ActiveCooling=") {
                    if let Some(value) = line.split('=').nth(1) {
                        if value.trim().to_lowercase() == "true" {
                            // Fan is active, return a generic 50% since we can't get actual speed
                            return Some(50);
                        }
                    }
                }
            }
        }
    }

    None
}

#[cfg(windows)]
fn get_cpu_power_draw() -> Option<f32> {
    use std::process::Command;

    // Method 1: Try Processor Performance counter
    if let Ok(output) = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-Counter '\\Processor Information(_Total)\\% Processor Performance' -ErrorAction SilentlyContinue | Select-Object -ExpandProperty CounterSamples | Select-Object -ExpandProperty CookedValue"
        ])
        .output()
    {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let trimmed = output_str.trim();
            if !trimmed.is_empty() {
                if let Ok(perf) = trimmed.parse::<f32>() {
                    if perf > 0.0 {
                        // Estimate based on typical Ryzen 7 7800X3D TDP (120W)
                        let estimated_power = (perf / 100.0) * 120.0;
                        return Some(estimated_power);
                    }
                }
            }
        }
    }

    // Method 2: Try Win32_Processor CurrentVoltage and LoadPercentage
    if let Ok(output) = Command::new("wmic")
        .args(["cpu", "get", "LoadPercentage", "/value"])
        .output()
    {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.starts_with("LoadPercentage=") {
                    if let Some(value) = line.split('=').nth(1) {
                        if let Ok(load) = value.trim().parse::<f32>() {
                            // Rough estimate: 120W TDP for Ryzen 7 7800X3D
                            let estimated_power = (load / 100.0) * 120.0;
                            return Some(estimated_power);
                        }
                    }
                }
            }
        }
    }

    None
}

#[cfg(not(windows))]
fn get_cpu_temperature() -> Option<f32> {
    use std::fs;

    if let Ok(content) = fs::read_to_string("/sys/class/thermal/thermal_zone0/temp") {
        if let Ok(temp_millidegrees) = content.trim().parse::<i32>() {
            return Some(temp_millidegrees as f32 / 1000.0);
        }
    }

    None
}

#[cfg(not(windows))]
fn get_cpu_fan_speed() -> Option<u32> {
    use std::fs;
    use std::path::Path;

    for hwmon_idx in 0..10 {
        let base_path = format!("/sys/class/hwmon/hwmon{}", hwmon_idx);
        if !Path::new(&base_path).exists() {
            continue;
        }

        for fan_idx in 1..5 {
            let fan_path = format!("{}/fan{}_input", base_path, fan_idx);
            if let Ok(content) = fs::read_to_string(&fan_path) {
                if let Ok(rpm) = content.trim().parse::<u32>() {
                    if rpm > 0 {
                        return Some(((rpm as f32 / 3000.0) * 100.0).min(100.0) as u32);
                    }
                }
            }
        }
    }

    None
}

#[cfg(not(windows))]
fn get_cpu_power_draw() -> Option<f32> {
    use std::fs;
    use std::path::Path;

    for hwmon_idx in 0..10 {
        let base_path = format!("/sys/class/hwmon/hwmon{}", hwmon_idx);
        if !Path::new(&base_path).exists() {
            continue;
        }

        if let Ok(name) = fs::read_to_string(format!("{}/name", base_path)) {
            let name = name.trim().to_lowercase();
            if name.contains("coretemp") || name.contains("k10temp") || name.contains("cpu") {
                if let Ok(content) = fs::read_to_string(format!("{}/power1_input", base_path)) {
                    if let Ok(power_uw) = content.trim().parse::<u64>() {
                        return Some(power_uw as f32 / 1_000_000.0);
                    }
                }
            }
        }
    }

    None
}

impl Default for SystemCollector {
    fn default() -> Self {
        Self::new()
    }
}

// Get local IP address by creating a UDP socket
fn get_local_ip() -> Result<String, std::io::Error> {
    use std::net::UdpSocket;

    // Connect to a public DNS server (doesn't actually send data)
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("8.8.8.8:80")?;

    match socket.local_addr() {
        Ok(addr) => {
            let ip = addr.ip();
            Ok(ip.to_string())
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::Ordering;

    #[test]
    fn test_system_collector_drop_cleanup() {
        // Create a SystemCollector
        let collector = SystemCollector::new();
        
        // Verify shutdown signal is initially false
        assert!(!collector.shutdown_signal.load(Ordering::Relaxed));
        
        // Drop the collector
        drop(collector);
        
        // Thread should have been signaled to shutdown and joined
        // If this test completes without hanging, the cleanup works
    }

    #[test]
    fn test_system_collector_thread_shutdown() {
        let collector = SystemCollector::new();
        let shutdown_clone = Arc::clone(&collector.shutdown_signal);
        
        // Verify thread is running (shutdown signal is false)
        assert!(!shutdown_clone.load(Ordering::Relaxed));
        
        // Drop collector to trigger cleanup
        drop(collector);
        
        // Give a moment for cleanup to complete
        std::thread::sleep(std::time::Duration::from_millis(200));
        
        // Shutdown signal should have been set
        assert!(shutdown_clone.load(Ordering::Relaxed));
    }

    #[test]
    fn test_system_collector_collect_data() {
        let mut collector = SystemCollector::new();
        
        // Should not panic
        let data = collector.collect(false);
        
        // Verify we got valid data
        assert!(data.cpu.usage >= 0.0 && data.cpu.usage <= 100.0);
        assert!(data.memory.total > 0);
        assert!(data.cpu.frequency >= 0.0);
        
        // Cleanup
        drop(collector);
    }

    #[test]
    fn test_system_collector_multiple_collections() {
        let mut collector = SystemCollector::new();
        
        // Collect multiple times - should not crash or leak
        for _ in 0..5 {
            let data = collector.collect(false);
            assert!(data.cpu.usage >= 0.0);
        }
    }

    #[test]
    fn test_system_collector_with_swap() {
        let mut collector = SystemCollector::new();
        
        // Collect data (swap is always shown now)
        let data = collector.collect(false);
        
        // Should return valid data with swap info
        assert!(data.memory.total > 0);
        // swap_total is u64, always >= 0, so just verify it exists
    }

    #[test]
    fn test_memory_receiver_channel() {
        let mut collector = SystemCollector::new();
        
        // Collect data
        let data = collector.collect(false);
        
        // Should have memory data
        assert!(data.memory.total > 0);
    }

    #[test]
    fn test_no_thread_leak_on_drop() {
        // Create and drop multiple collectors
        for _ in 0..5 {
            let mut collector = SystemCollector::new();
            let _data = collector.collect(false);
            drop(collector);
        }
        
        // If we get here, no thread leak
    }

    #[test]
    fn test_process_collection_limit() {
        let mut collector = SystemCollector::new();
        
        // Collect with processes enabled
        let data = collector.collect(true);
        
        // Should have processes, but limited to 100
        assert!(data.processes.len() <= 100);
    }

    #[test]
    fn test_skip_process_collection() {
        let mut collector = SystemCollector::new();
        
        // Collect without processes
        let data = collector.collect(false);
        
        // Should have no processes
        assert!(data.processes.is_empty());
    }
}
