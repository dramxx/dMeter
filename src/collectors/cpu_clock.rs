//! Effective CPU clock on Windows.
//!
//! sysinfo reports the nominal (base) frequency on Windows, which never changes. Task Manager's
//! "Speed" is the base frequency scaled by the `% Processor Performance` counter, read here via PDH.

use windows_sys::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterValue,
    PdhOpenQueryW, PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY,
};

pub struct CpuClock {
    query: PDH_HQUERY,
    performance: PDH_HCOUNTER,
    base_frequency: PDH_HCOUNTER,
}

// PDH handles may be used from any thread; the collector only uses them from one at a time
unsafe impl Send for CpuClock {}

impl CpuClock {
    pub fn new() -> Option<Self> {
        let mut query: PDH_HQUERY = std::ptr::null_mut();
        if unsafe { PdhOpenQueryW(std::ptr::null(), 0, &mut query) } != 0 {
            return None;
        }

        // English counter names work regardless of the Windows display language
        let clock = Self {
            query,
            performance: add_counter(query, r"\Processor Information(_Total)\% Processor Performance")?,
            base_frequency: add_counter(query, r"\Processor Information(_Total)\Processor Frequency")?,
        };

        // % Processor Performance is a rate counter: prime it so the next sample has a baseline
        unsafe { PdhCollectQueryData(query) };
        Some(clock)
    }

    /// Current effective clock in MHz.
    pub fn sample(&self) -> Option<f32> {
        if unsafe { PdhCollectQueryData(self.query) } != 0 {
            return None;
        }
        let performance = read_counter(self.performance)?;
        let base_frequency = read_counter(self.base_frequency)?;
        (performance > 0.0 && base_frequency > 0.0)
            .then(|| (base_frequency * performance / 100.0) as f32)
    }
}

impl Drop for CpuClock {
    fn drop(&mut self) {
        unsafe { PdhCloseQuery(self.query) };
    }
}

fn add_counter(query: PDH_HQUERY, path: &str) -> Option<PDH_HCOUNTER> {
    let path: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let mut counter: PDH_HCOUNTER = std::ptr::null_mut();
    (unsafe { PdhAddEnglishCounterW(query, path.as_ptr(), 0, &mut counter) } == 0).then_some(counter)
}

fn read_counter(counter: PDH_HCOUNTER) -> Option<f64> {
    let mut value: PDH_FMT_COUNTERVALUE = unsafe { std::mem::zeroed() };
    let status = unsafe {
        PdhGetFormattedCounterValue(counter, PDH_FMT_DOUBLE, std::ptr::null_mut(), &mut value)
    };
    (status == 0).then_some(unsafe { value.Anonymous.doubleValue })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effective_clock_is_plausible() {
        let clock = CpuClock::new().expect("PDH processor counters should be available");
        std::thread::sleep(std::time::Duration::from_millis(500));
        let mhz = clock.sample().expect("clock sample");
        assert!((100.0..=10_000.0).contains(&mhz), "{mhz} MHz");
    }
}
