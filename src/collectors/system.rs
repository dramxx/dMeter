use crate::state::{
    CpuData, DiskData, MemoryData, NetworkData, ProcessData, SystemData, SystemInfoData,
};
use std::collections::HashMap;
use std::sync::Mutex;
use sysinfo::{Disks, Networks, System};

pub struct SystemCollector {
    sys: Mutex<System>,
    networks: Mutex<Networks>,
    disks: Mutex<Disks>,
    last_network_rx: Mutex<u64>,
    last_network_tx: Mutex<u64>,
}

impl SystemCollector {
    pub fn new() -> Self {
        let sys = System::new_all();
        let networks = Networks::new_with_refreshed_list();
        let disks = Disks::new_with_refreshed_list();

        let (tx, rx) = Self::get_network_totals(&networks);

        Self {
            sys: Mutex::new(sys),
            networks: Mutex::new(networks),
            disks: Mutex::new(disks),
            last_network_rx: Mutex::new(rx),
            last_network_tx: Mutex::new(tx),
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

    pub fn collect(&mut self, _show_swap: bool) -> SystemData {
        {
            let mut sys = self.sys.lock().unwrap();
            sys.refresh_all();
        }

        {
            let mut networks = self.networks.lock().unwrap();
            networks.refresh();
        }

        {
            let mut disks = self.disks.lock().unwrap();
            disks.refresh();
        }

        let cpu = self.collect_cpu();
        let memory = self.collect_memory();
        let network = self.collect_network();
        let disks = self.collect_disks();
        let system = self.collect_system();
        let processes = self.collect_processes();

        SystemData {
            cpu,
            gpu: crate::collectors::gpu::collect_gpu_data(),
            memory,
            network,
            disks,
            system,
            processes,
        }
    }

    fn collect_cpu(&self) -> CpuData {
        let sys = self.sys.lock().unwrap();
        let cpus = sys.cpus();

        let name = cpus
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string());

        let usage = sys.global_cpu_usage();
        let frequency = cpus.first().map(|c| c.frequency()).unwrap_or(0);

        let core_usage: Vec<f32> = cpus.iter().map(|c| c.cpu_usage()).collect();

        let physical_cores = sys.physical_core_count().unwrap_or(1);
        let logical_cores = cpus.len();

        CpuData {
            name,
            usage,
            core_usage,
            temperature: None,
            frequency,
            physical_cores,
            logical_cores,
        }
    }

    fn collect_memory(&self) -> MemoryData {
        let sys = self.sys.lock().unwrap();
        MemoryData {
            total: sys.total_memory(),
            used: sys.used_memory(),
            swap_total: sys.total_swap(),
            swap_used: sys.used_swap(),
        }
    }

    fn collect_network(&self) -> NetworkData {
        let networks = self.networks.lock().unwrap();
        let (total_tx, total_rx) = Self::get_network_totals(&networks);
        drop(networks);

        let mut last_tx = self.last_network_tx.lock().unwrap();
        let mut last_rx = self.last_network_rx.lock().unwrap();

        let upload_speed = total_tx.saturating_sub(*last_tx);
        let download_speed = total_rx.saturating_sub(*last_rx);

        *last_tx = total_tx;
        *last_rx = total_rx;

        let networks = self.networks.lock().unwrap();
        let mut adapter_name = String::new();
        let mut ip_address = String::new();

        for (name, data) in networks.iter() {
            if data.received() > 0 || data.transmitted() > 0 {
                if adapter_name.is_empty() {
                    adapter_name = name.clone();
                    ip_address = "127.0.0.1".to_string();
                }
            }
        }

        NetworkData {
            adapter_name,
            ip_address,
            upload_speed,
            download_speed,
            total_sent: total_tx,
            total_received: total_rx,
        }
    }

    fn collect_disks(&self) -> Vec<DiskData> {
        let disks = self.disks.lock().unwrap();
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
        let _sys = self.sys.lock().unwrap();

        let os_name = System::name().unwrap_or_else(|| "Unknown".to_string());
        let os_version = System::os_version().unwrap_or_else(|| "Unknown".to_string());
        let hostname = System::host_name().unwrap_or_else(|| "Unknown".to_string());
        let uptime = System::uptime();

        let load_avg = sysinfo::System::load_average();

        SystemInfoData {
            os_name,
            os_version,
            hostname,
            uptime,
            load_avg: (
                load_avg.one as f32,
                load_avg.five as f32,
                load_avg.fifteen as f32,
            ),
        }
    }

    fn collect_processes(&self) -> Vec<ProcessData> {
        let sys = self.sys.lock().unwrap();

        let our_pid = std::process::id();

        let mut process_map: HashMap<String, (f32, u64)> = HashMap::new();

        for (pid, process) in sys.processes().iter() {
            let pid_u32 = pid.as_u32();

            let name = process.name().to_string_lossy().to_string();
            let name_lower = name.to_lowercase();

            if pid_u32 == our_pid
                || name_lower.contains("opencode")
                || name_lower.contains("dmeter")
            {
                continue;
            }

            if name_lower.contains("nvgwls")
                || name_lower.contains("nvcontainer")
                || name_lower.contains("nvdisplay")
                || name_lower.contains("nvidia")
                || name_lower.contains("textinputhost")
                || name_lower.contains("phoneexperiencehost")
                || name_lower.contains("startmenuexperience")
                || name_lower.contains("searchhost")
                || name_lower.contains("widgetboard")
                || name_lower.contains("lockapp")
                || name_lower.contains("shellhost")
                || name_lower.contains("applicationframehost")
                || name_lower.contains("crossdevice")
                || name_lower.contains("vctip")
                || name_lower.contains("microsoftstartfeed")
                || name_lower.contains("msedgewebview2")
                || name_lower.contains("webview2")
                || name_lower.contains("systemsettings")
                || name_lower.contains("_runtime")
                || name_lower.contains("gamebar")
                || name_lower.contains("pca")
                || name_lower.contains("securityhealth")
                || name_lower.contains("widgetservice")
                || name_lower.contains("sihost")
                || name_lower.contains("ctfmon")
                || name_lower.contains("dllhost")
                || name_lower.contains("conhost")
                || name_lower.contains("runtimebroker")
                || name_lower.contains("shellexperience")
                || name_lower.contains("desktopimgui")
                || name_lower.contains("system")
                || name_lower.contains("registry")
                || name_lower.contains("smss")
                || name_lower.contains("csrss")
                || name_lower.contains("wininit")
                || name_lower.contains("services")
                || name_lower.contains("lsass")
                || name_lower.contains("svchost")
                || name_lower.contains("dwm")
                || name_lower.contains("winlogon")
                || name_lower.contains("fontdrvhost")
                || name_lower.contains("馨")
                || name_lower.contains("mpmusic")
                || name_lower.contains("muse")
                || name_lower.contains("searchindexer")
                || name_lower.contains("searchui")
                || name_lower.contains("mus")
                || name_lower.contains("msob")
                || name_lower.contains("wmi")
                || name_lower.contains("mpdefender")
                || name_lower.contains("msmpeng")
                || name_lower.contains("mosuso")
                || name_lower.contains("update")
                || name_lower.contains("atiecl")
                || name_lower.contains("taskhostw")
                || name_lower.contains("memory compression")
                || name_lower.contains("oobe")
                || name_lower.contains("userlistbroker")
                || name_lower.contains("user oobe")
                || name_lower.contains("bash")
                || name_lower.contains("explorer")
            {
                continue;
            }

            let cpu_usage = process.cpu_usage();
            let memory_mb = process.memory() / (1024 * 1024);

            // Only show processes using some memory (likely user apps)
            if memory_mb < 10 {
                continue;
            }

            let entry = process_map.entry(name).or_insert((0.0, 0));
            entry.0 = entry.0.max(cpu_usage);
            entry.1 += memory_mb;
        }

        let mut processes: Vec<ProcessData> = process_map
            .into_iter()
            .map(|(name, (cpu_usage, memory_mb))| ProcessData {
                name,
                cpu_usage,
                memory_mb,
            })
            .collect();

        processes.sort_by(|a, b| b.memory_mb.cmp(&a.memory_mb));
        processes.truncate(30);

        processes
    }
}

impl Default for SystemCollector {
    fn default() -> Self {
        Self::new()
    }
}
