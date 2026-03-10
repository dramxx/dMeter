# dmeter Implementation Plan

## Project Overview
A fast, beautiful terminal system monitor written in Rust. Single binary, no dependencies, cross-platform (Windows primary, Linux secondary).

---

## Phase 1: Project Setup (Day 1)

### 1.1 Initialize Rust Project
```bash
cargo new dmeter
cd dmeter
```

### 1.2 Add Dependencies (Cargo.toml)
- `ratatui` - TUI rendering
- `crossterm` - Terminal backend
- `sysinfo` - CPU, RAM, disk, network
- `nvml-wrapper` - NVIDIA GPU info
- `tokio` - Async runtime
- `toml` - Config file parsing
- `dirs` - Config directory detection
- `clap` - CLI argument parsing
- `chrono` - Date/time handling
- `log` + `env_logger` - Logging

### 1.3 Project Structure
```
src/
├── main.rs           # Entry point, CLI args
├── lib.rs            # Module declarations
├── config.rs         # Configuration loading
├── state.rs          # App state management
├── ui/
│   ├── mod.rs        # UI module
│   ├── layout.rs     # Panel layout calculations
│   ├── header.rs    # Header bar
│   ├── cpu.rs       # CPU panel
│   ├── gpu.rs       # GPU panel  
│   ├── memory.rs    # Memory panel
│   ├── network.rs   # Network panel
│   ├── disk.rs      # Disk panel
│   └── system.rs    # System info bar
├── collectors/
│   ├── mod.rs       # Data collection module
│   ├── cpu.rs       # CPU data collector
│   ├── gpu.rs       # GPU data collector (NVIDIA/WMI)
│   ├── memory.rs    # Memory collector
│   ├── network.rs   # Network collector
│   └── disk.rs      # Disk collector
├── utils/
│   ├── mod.rs
│   ├── thresholds.rs # Color thresholds
│   └── format.rs    # Human-readable formatting
```

---

## Phase 2: Core Infrastructure (Day 1-2)

### 2.1 Configuration System
- Create `Config` struct with fields: `interval`, `temp_unit`, `theme`, `show_swap`, `show_per_core`
- Load from `~/.config/dmeter/config.toml` (create if missing with defaults)
- CLI args override config file values

### 2.2 Data Collectors
- Implement trait-based collector pattern for extensibility
- `SystemCollector` trait with `collect()` returning `SystemData`
- Thread-safe interior mutability using `Arc<Mutex<T>>`

### 2.3 App State
```rust
struct AppState {
    config: Config,
    data: SystemData,
    history: HistoryBuffer,  // For sparklines
    is_paused: bool,
    last_network: Option<NetworkSnapshot>,
}
```

### 2.4 TUI Framework
- Initialize ratatui `Terminal` with crossterm backend
- Set up alternate screen buffer
- Handle resize events
- Implement minimum size check (< 80x24)

---

## Phase 3: Panel Implementation (Day 2-4)

### 3.1 Header Bar
- Left: "dmeter" app name
- Right: hostname | date | time
- Full width, fixed height (1 row)

### 3.2 CPU Panel
- Model name + clock speed
- Overall usage bar with color thresholds (green/yellow/red)
- Per-core mini-bars (conditional on space)
- Temperature (°C)
- Physical/logical core count

### 3.3 GPU Panel
- Model name
- Usage bar
- VRAM used/total bar
- Temperature
- Fan speed (RPM/%)
- Power draw (W)
- Graceful fallback: "No GPU / NVIDIA not detected"

### 3.4 Memory Panel
- RAM bar: used / total
- SWAP bar: used / total (configurable display)
- Human-readable values (GB/MB)

### 3.5 Network Panel
- Active adapter name
- IPv4 address
- Upload speed (↑ auto-scaled)
- Download speed (↓ auto-scaled)
- Session totals (sent/received)

### 3.6 Disk Panel
- Iterate all mounted drives
- Per-drive: label, mount point, used/total, filesystem type
- Usage bar per drive

### 3.7 System Info Bar (bottom)
- OS name + version
- Uptime (days:hours:minutes)
- Load average (1/5/15 min)

---

## Phase 4: Advanced Features (Day 4-5)

### 4.1 Sparklines (CPU/GPU history)
- Circular buffer of last 60 readings
- Render as ASCII sparkline: `▁▂▄▆▇▆▄▃▂▁`
- Show only if terminal height >= 36 rows

### 4.2 Proportional Scaling
| Height | Mode |
|--------|------|
| < 24 | Show resize warning |
| 24-35 | Compact - no sparklines, tight padding |
| 36-50 | Standard - sparklines visible |
| 50+ | Spacious - larger bars, more padding |

### 4.3 Color Thresholds
- **Usage**: 0-60% green, 60-85% yellow, 85-100% red
- **Temperature**: <60°C green, 60-80°C yellow, >80°C red

### 4.4 Keyboard Shortcuts
- `q` - Quit
- `Ctrl+C` - Quit
- `p` - Pause/resume
- `r` - Force refresh

---

## Phase 5: Polish & Distribution (Day 5)

### 5.1 Clean Exit
- Restore original screen buffer
- Restore cursor position
- Reset terminal to original state

### 5.2 Logging
- Debug mode: log to stderr
- Release mode: silent unless error

### 5.3 Build & Release
```bash
cargo build --release
# Produces: target/release/dmeter.exe
```

---

## Implementation Order

1. **Setup**: Project init, dependencies, basic structure
2. **Config + CLI**: config.rs, main.rs with clap
3. **Collectors**: sysinfo integration for CPU/Memory/Disk/Network
4. **GPU**: nvml-wrapper for NVIDIA, fallback handling
5. **UI Shell**: ratatui setup, layout, resize handling
6. **Panels**: Implement one by one (CPU → Memory → GPU → Network → Disk → System)
7. **Sparklines**: History buffer + rendering
8. **Polish**: Thresholds, keyboard shortcuts, clean exit
9. **Test**: Manual testing on Windows, verify all features work

---

## Key Technical Decisions

| Decision | Rationale |
|----------|-----------|
| ratatui over tui-rs | Active maintenance, better docs |
| sysinfo for system data | Mature, cross-platform |
| nvml-wrapper for NVIDIA | Official NVIDIA API, best coverage |
| tokio for async | Required for non-blocking refresh loop |
| 2-second default tick | Balance between responsiveness and readability |

---

## Testing Checklist

- [ ] Terminal resize handling (minimum size warning)
- [ ] All panels render correctly at different terminal sizes
- [ ] Color thresholds change appropriately
- [ ] Pause/resume works
- [ ] Clean exit restores terminal
- [ ] Config file loads/saves correctly
- [ ] CLI args override config
- [ ] Network speed calculation accurate
- [ ] GPU fallback when no NVIDIA
- [ ] Works in PowerShell (Windows)
