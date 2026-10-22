#[cfg(windows)]
pub mod cpu_clock;
pub mod gpu;
pub mod memory_info;
pub mod system;

pub use system::SystemCollector;
