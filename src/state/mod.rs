#[derive(Default)]
pub struct MemoryData {
    pub total: u64,
    pub used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub commit_total: u64,
    pub commit_used: u64,
    pub cached: u64,
}

pub struct CpuData {
    pub usage: f32,
    pub temperature: Option<f32>,
    pub fan_speed: Option<u32>,
    pub power_draw: Option<f32>,
    pub name: String,
    pub frequency: f32,
}

impl Default for CpuData {
    fn default() -> Self {
        Self {
            usage: 0.0,
            temperature: None,
            fan_speed: None,
            power_draw: None,
            name: String::new(),
            frequency: 0.0,
        }
    }
}

pub struct GpuData {
    pub available: bool,
    pub name: String,
    pub usage: f32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub temperature: Option<f32>,
    pub fan_speed: Option<u32>,
    pub power_draw: Option<f32>,
}

impl Default for GpuData {
    fn default() -> Self {
        Self {
            available: false,
            name: String::new(),
            usage: 0.0,
            memory_used: 0,
            memory_total: 0,
            temperature: None,
            fan_speed: None,
            power_draw: None,
        }
    }
}

pub struct NetworkData {
    pub upload_speed: f64,
    pub download_speed: f64,
    pub adapter_name: String,
    pub ip_address: String,
}

impl Default for NetworkData {
    fn default() -> Self {
        Self {
            upload_speed: 0.0,
            download_speed: 0.0,
            adapter_name: String::new(),
            ip_address: String::new(),
        }
    }
}

#[derive(Default)]
pub struct DiskData {
    pub name: String,
    pub mount_point: String,
    pub total: u64,
    pub used: u64,
    pub filesystem: String,
}

pub struct DiskIOData {
    pub read_speed: f64,
    pub write_speed: f64,
}

impl Default for DiskIOData {
    fn default() -> Self {
        Self {
            read_speed: 0.0,
            write_speed: 0.0,
        }
    }
}

#[derive(Default)]
pub struct SystemInfoData {
    pub hostname: String,
    pub os_name: String,
    pub os_version: String,
    pub uptime: u64,
}

pub struct ProcessData {
    pub name: String,
    pub cpu_usage: f32,
    pub memory_usage: f32,  // Percentage
}

#[derive(Default)]
pub struct SystemData {
    pub cpu: CpuData,
    pub memory: MemoryData,
    pub gpu: GpuData,
    pub network: NetworkData,
    pub disks: Vec<DiskData>,
    pub disk_io: DiskIOData,
    pub system: SystemInfoData,
    pub processes: Vec<ProcessData>,
}

pub struct HistoryBuffer {
    data: Vec<f32>,
    max_size: usize,
}

impl HistoryBuffer {
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![0.0; size],
            max_size: size,
        }
    }

    pub fn push(&mut self, value: f32) {
        if self.max_size == 0 {
            return;
        }
        self.data.remove(0);
        self.data.push(value);
    }

    pub fn get(&self) -> Vec<f32> {
        self.data.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_data_default() {
        let mem = MemoryData::default();
        assert_eq!(mem.total, 0);
        assert_eq!(mem.used, 0);
        assert_eq!(mem.swap_total, 0);
        assert_eq!(mem.swap_used, 0);
    }

    #[test]
    fn test_cpu_data_default() {
        let cpu = CpuData::default();
        assert_eq!(cpu.usage, 0.0);
        assert_eq!(cpu.temperature, None);
        assert_eq!(cpu.fan_speed, None);
        assert_eq!(cpu.name, "");
        assert_eq!(cpu.frequency, 0.0);
    }

    #[test]
    fn test_gpu_data_default() {
        let gpu = GpuData::default();
        assert!(!gpu.available);
        assert_eq!(gpu.name, "");
        assert_eq!(gpu.usage, 0.0);
        assert_eq!(gpu.memory_used, 0);
        assert_eq!(gpu.memory_total, 0);
    }

    #[test]
    fn test_network_data_default() {
        let net = NetworkData::default();
        assert_eq!(net.upload_speed, 0.0);
        assert_eq!(net.download_speed, 0.0);
        assert_eq!(net.adapter_name, "");
        assert_eq!(net.ip_address, "");
    }

    #[test]
    fn test_system_data_default() {
        let system = SystemData::default();
        assert_eq!(system.cpu.usage, 0.0);
        assert_eq!(system.memory.total, 0);
        assert!(!system.gpu.available);
        assert!(system.disks.is_empty());
    }

    #[test]
    fn test_history_buffer_new() {
        let buffer = HistoryBuffer::new(5);
        assert_eq!(buffer.get().len(), 5);
        assert!(buffer.get().iter().all(|&x| x == 0.0));
    }

    #[test]
    fn test_history_buffer_push() {
        let mut buffer = HistoryBuffer::new(3);
        
        buffer.push(1.0);
        assert_eq!(buffer.get(), vec![0.0, 0.0, 1.0]);
        
        buffer.push(2.0);
        buffer.push(3.0);
        assert_eq!(buffer.get(), vec![1.0, 2.0, 3.0]);
        
        buffer.push(4.0);
        assert_eq!(buffer.get(), vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_history_buffer_zero_capacity() {
        let mut buffer = HistoryBuffer::new(0);
        assert_eq!(buffer.get().len(), 0);
        
        buffer.push(1.0);
        assert_eq!(buffer.get().len(), 0);
    }

    #[test]
    fn test_system_info_data_default() {
        let info = SystemInfoData::default();
        assert_eq!(info.hostname, "");
        assert_eq!(info.os_name, "");
        assert_eq!(info.os_version, "");
        assert_eq!(info.uptime, 0);
    }
}
