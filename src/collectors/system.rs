use crate::collectors::gpu::GpuCollector;
use crate::state::{
    CpuData, DiskData, DiskIOData, MemoryData, NetworkData, ProcessData, SystemData,
    SystemInfoData,
};
use std::time::Instant;
use sysinfo::{Disks, Networks, System};

pub struct SystemCollector {
    sys: System,
    networks: Networks,
    disks: Disks,
    gpu: GpuCollector,
    #[cfg(windows)]
    cpu_clock: Option<crate::collectors::cpu_clock::CpuClock>,
    // Network and disk counters are deltas since the previous refresh; this turns them into rates
    last_refresh: Instant,
    // Local IP cache: doesn't change at runtime
    cached_ip: Option<String>,
}

impl SystemCollector {
    pub fn new() -> Self {
        Self {
            sys: System::new_all(),
            networks: Networks::new_with_refreshed_list(),
            disks: Disks::new_with_refreshed_list(),
            gpu: GpuCollector::new(),
            #[cfg(windows)]
            cpu_clock: crate::collectors::cpu_clock::CpuClock::new(),
            last_refresh: Instant::now(),
            cached_ip: None,
        }
    }

    pub fn collect(&mut self, collect_processes: bool) -> SystemData {
        // Use targeted refresh instead of refresh_all() for better performance
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        // Only refresh processes if we need them
        if collect_processes {
            self.sys
                .refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        }

        let elapsed_secs = self.last_refresh.elapsed().as_secs_f64().max(0.001);
        self.networks.refresh(true);
        self.disks.refresh(true);
        self.last_refresh = Instant::now();

        SystemData {
            cpu: self.collect_cpu(),
            gpu: self.gpu.collect(),
            memory: self.collect_memory(),
            network: self.collect_network(elapsed_secs),
            disks: self.collect_disks(),
            disk_io: self.collect_disk_io(elapsed_secs),
            system: Self::collect_system(),
            processes: if collect_processes {
                self.collect_processes()
            } else {
                Vec::new()
            },
        }
    }

    fn collect_cpu(&self) -> CpuData {
        let cpus = self.sys.cpus();

        let name = cpus
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string());
        let frequency = cpus.first().map(|c| c.frequency() as f32).unwrap_or(0.0);
        #[cfg(windows)]
        let frequency = self
            .cpu_clock
            .as_ref()
            .and_then(|clock| clock.sample())
            .unwrap_or(frequency);

        // Windows exposes no real CPU temperature/fan/power source without a kernel driver
        #[cfg(windows)]
        let (temperature, fan_speed, power_draw) = (None, None, None);
        #[cfg(not(windows))]
        let (temperature, fan_speed, power_draw) =
            (get_cpu_temperature(), get_cpu_fan_speed(), get_cpu_power_draw());

        CpuData {
            name,
            usage: self.sys.global_cpu_usage(),
            temperature,
            frequency,
            fan_speed,
            power_draw,
        }
    }

    fn collect_memory(&self) -> MemoryData {
        let (commit_total, commit_used, cached) =
            crate::collectors::memory_info::collect_extended_memory_info();

        MemoryData {
            total: self.sys.total_memory(),
            used: self.sys.used_memory(),
            swap_total: self.sys.total_swap(),
            swap_used: self.sys.used_swap(),
            commit_total,
            commit_used,
            cached,
        }
    }

    fn collect_network(&mut self, elapsed_secs: f64) -> NetworkData {
        let (mut sent, mut received) = (0u64, 0u64);
        for data in self.networks.values() {
            sent += data.transmitted();
            received += data.received();
        }

        // Show the adapter that has carried the most traffic overall
        let adapter_name = self
            .networks
            .iter()
            .filter(|(_, d)| d.total_received() > 0 || d.total_transmitted() > 0)
            .max_by_key(|(_, d)| d.total_received() + d.total_transmitted())
            .map(|(name, _)| name.clone())
            .unwrap_or_default();

        if self.cached_ip.is_none() {
            self.cached_ip = get_local_ip().ok();
        }

        NetworkData {
            adapter_name,
            ip_address: self.cached_ip.clone().unwrap_or_default(),
            upload_speed: sent as f64 / elapsed_secs,
            download_speed: received as f64 / elapsed_secs,
        }
    }

    fn collect_disk_io(&self, elapsed_secs: f64) -> DiskIOData {
        let (mut read, mut written) = (0u64, 0u64);
        for disk in self.disks.list() {
            let usage = disk.usage();
            read += usage.read_bytes;
            written += usage.written_bytes;
        }

        DiskIOData {
            read_speed: read as f64 / elapsed_secs,
            write_speed: written as f64 / elapsed_secs,
        }
    }

    fn collect_disks(&self) -> Vec<DiskData> {
        self.disks
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

    fn collect_system() -> SystemInfoData {
        SystemInfoData {
            os_name: System::name().unwrap_or_else(|| "Unknown".to_string()),
            os_version: System::os_version().unwrap_or_else(|| "Unknown".to_string()),
            hostname: System::host_name().unwrap_or_else(|| "Unknown".to_string()),
            uptime: System::uptime(),
        }
    }

    fn collect_processes(&self) -> Vec<ProcessData> {
        let total_memory = self.sys.total_memory();

        let mut processes: Vec<ProcessData> = self
            .sys
            .processes()
            .values()
            .map(|process| {
                let memory_usage = if total_memory > 0 {
                    (process.memory() as f32 / total_memory as f32) * 100.0
                } else {
                    0.0
                };

                ProcessData {
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

    #[test]
    fn test_system_collector_collect_data() {
        let mut collector = SystemCollector::new();

        // Should not panic
        let data = collector.collect(false);

        // Verify we got valid data
        assert!(data.cpu.usage >= 0.0 && data.cpu.usage <= 100.0);
        assert!(data.memory.total > 0);
        assert!(data.cpu.frequency >= 0.0);
    }

    #[test]
    fn test_system_collector_multiple_collections() {
        let mut collector = SystemCollector::new();

        // Collect multiple times - should not crash or leak
        for _ in 0..5 {
            let data = collector.collect(false);
            assert!(data.cpu.usage >= 0.0);
            assert!(data.network.upload_speed >= 0.0);
            assert!(data.disk_io.read_speed >= 0.0);
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_commit_memory() {
        let mut collector = SystemCollector::new();
        let data = collector.collect(false);

        assert!(data.memory.commit_total > 0);
        assert!(data.memory.commit_used <= data.memory.commit_total);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_has_no_fake_cpu_sensors() {
        let mut collector = SystemCollector::new();
        let data = collector.collect(false);

        assert!(data.cpu.temperature.is_none());
        assert!(data.cpu.fan_speed.is_none());
        assert!(data.cpu.power_draw.is_none());
    }

    #[test]
    fn test_process_collection_limit() {
        let mut collector = SystemCollector::new();

        // Collect with processes enabled
        let data = collector.collect(true);

        // Should have processes, but limited to 100
        assert!(!data.processes.is_empty());
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
