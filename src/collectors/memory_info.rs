#[cfg(windows)]
pub fn get_windows_memory_info() -> Option<(u64, u64, u64)> {
    // Try WMIC first (fast), fallback to PowerShell (slower but more reliable)
    if let Some(result) = try_wmic_memory() {
        return Some(result);
    }

    // Fallback to PowerShell if WMIC fails
    try_powershell_memory()
}

#[cfg(windows)]
fn try_wmic_memory() -> Option<(u64, u64, u64)> {
    use std::process::Command;

    if let Ok(output) = Command::new("cmd")
        .args([
            "/C",
            "wmic OS get TotalVirtualMemorySize,FreeVirtualMemory /format:csv",
        ])
        .output()
    {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);

            // Parse WMIC CSV output
            for line in output_str.lines() {
                if line.contains("TotalVirtualMemorySize") && !line.trim().is_empty() {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() >= 3 {
                        if let (Ok(total_kb), Ok(free_kb)) = (
                            parts[1].trim().parse::<u64>(),
                            parts[2].trim().parse::<u64>(),
                        ) {
                            let commit_total = total_kb * 1024; // Convert KB to bytes
                            let commit_used = (total_kb - free_kb) * 1024;
                            let cached = commit_total / 10; // Rough estimate: 10% of total
                            return Some((commit_total, commit_used, cached));
                        }
                    }
                }
            }
        }
    }

    None
}

#[cfg(windows)]
fn try_powershell_memory() -> Option<(u64, u64, u64)> {
    use std::process::Command;

    if let Ok(output) = Command::new("powershell")
        .args([
            "-Command",
            "Get-WmiObject -Class Win32_OperatingSystem | Select-Object TotalVirtualMemorySize, FreeVirtualMemory | ConvertTo-Csv -NoTypeInformation"
        ])
        .output()
    {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = output_str.lines().collect();
            if lines.len() >= 2 {
                let data_line = lines[1];
                let parts: Vec<&str> = data_line.split(',').collect();
                if parts.len() >= 2 {
                    if let (Ok(total_kb), Ok(free_kb)) = (parts[0].trim_matches('"').parse::<u64>(), parts[1].trim_matches('"').parse::<u64>()) {
                        let commit_total = total_kb * 1024;
                        let commit_used = (total_kb - free_kb) * 1024;
                        let cached = commit_total / 10;
                        return Some((commit_total, commit_used, cached));
                    }
                }
            }
        }
    }

    None
}

#[cfg(not(windows))]
pub fn get_windows_memory_info() -> Option<(u64, u64, u64)> {
    None
}

pub fn collect_extended_memory_info() -> (u64, u64, u64) {
    get_windows_memory_info().unwrap_or((0, 0, 0))
}
