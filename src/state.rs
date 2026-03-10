use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CpuData {
    pub name: String,
    pub usage: f32,
    pub core_usage: Vec<f32>,
    pub temperature: Option<f32>,
    pub frequency: u64,
    pub physical_cores: usize,
    pub logical_cores: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GpuData {
    pub available: bool,
    pub name: String,
    pub usage: f32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub temperature: Option<f32>,
    pub fan_speed: Option<u32>,
    pub power_draw: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryData {
    pub total: u64,
    pub used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkData {
    pub adapter_name: String,
    pub ip_address: String,
    pub upload_speed: u64,
    pub download_speed: u64,
    pub total_sent: u64,
    pub total_received: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiskData {
    pub name: String,
    pub mount_point: String,
    pub total: u64,
    pub used: u64,
    pub filesystem: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProcessData {
    pub name: String,
    pub cpu_usage: f32,
    pub memory_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemInfoData {
    pub os_name: String,
    pub os_version: String,
    pub hostname: String,
    pub uptime: u64,
    pub load_avg: (f32, f32, f32),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemData {
    pub cpu: CpuData,
    pub gpu: GpuData,
    pub memory: MemoryData,
    pub network: NetworkData,
    pub disks: Vec<DiskData>,
    pub system: SystemInfoData,
    pub processes: Vec<ProcessData>,
}

pub struct HistoryBuffer {
    data: Vec<f32>,
    capacity: usize,
}

impl HistoryBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, value: f32) {
        if self.data.len() >= self.capacity {
            self.data.remove(0);
        }
        self.data.push(value);
    }

    pub fn get(&self) -> &[f32] {
        &self.data
    }
}

impl Default for HistoryBuffer {
    fn default() -> Self {
        Self::new(60)
    }
}
