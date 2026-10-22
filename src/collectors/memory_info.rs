/// Returns (commit_limit, commit_used, cached) in bytes.
#[cfg(windows)]
pub fn collect_extended_memory_info() -> (u64, u64, u64) {
    use windows_sys::Win32::System::ProcessStatus::{GetPerformanceInfo, PERFORMANCE_INFORMATION};

    let mut info: PERFORMANCE_INFORMATION = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<PERFORMANCE_INFORMATION>() as u32;
    if unsafe { GetPerformanceInfo(&mut info, size) } == 0 {
        return (0, 0, 0);
    }

    // Commit and cache values are reported in pages
    let page = info.PageSize as u64;
    (
        info.CommitLimit as u64 * page,
        info.CommitTotal as u64 * page,
        info.SystemCache as u64 * page,
    )
}

#[cfg(not(windows))]
pub fn collect_extended_memory_info() -> (u64, u64, u64) {
    (0, 0, 0)
}
