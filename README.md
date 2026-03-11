# dMeter

A fast, beautiful terminal system monitor built with Rust and Ratatui.

## Features

### System Monitoring

- **CPU**: Real-time per-core and overall usage with sparkline history graphs
- **Memory**:
  - RAM and SWAP usage with progress bars
  - Commit memory tracking (Windows)
  - Cached memory display (Windows)
- **GPU**: NVIDIA GPU monitoring with VRAM tracking
  - Windows: NVML support
  - Linux: nvidia-smi integration
  - Real-time usage, temperature, and memory statistics
  - GPU and VRAM history sparklines
- **Disk**: Multi-disk usage for all mounted volumes
- **Network**: Real-time network I/O statistics with adapter information
- **Disk I/O**: Read/write speeds monitoring

### Visualization

- **History Graphs**: CPU, RAM, GPU, and VRAM sparkline visualizations
- **Conway's Game of Life**: Interactive cellular automaton background animation
- **Color-coded Metrics**: Intuitive color scheme for quick status assessment
- **Responsive Layout**: Adapts to terminal size with compact mode support

## Installation

### Windows (winget)

```powershell
winget install dMeter
```

### From Source

```bash
cargo build --release
```

The binary will be at `target/release/dmeter` (Linux/macOS) or `target/release/dmeter.exe` (Windows).

### Prerequisites

- Rust toolchain (1.70+)
- For GPU monitoring:
  - **Windows**: NVIDIA drivers with NVML support
  - **Linux**: nvidia-smi command (NVIDIA drivers)
- For Windows memory metrics: WMI access (standard on Windows)

## Usage

```bash
dmeter                    # Run with defaults (2-second refresh)
dmeter --interval 5       # Custom refresh interval (seconds)
dmeter --no-gpu           # Skip GPU panel
```

### Controls

- `q` or `Ctrl+C` - Quit
- `p` - Pause/Resume updates
- `r` - Force refresh

## Configuration

Config file location: `~/.config/dmeter/config.toml` (Linux/macOS) or `%APPDATA%\dmeter\config.toml` (Windows)

```toml
interval = 2        # Refresh interval in seconds
show_swap = true    # Show swap memory usage
```

## Performance

- **Low CPU overhead**: Optimized data collection with caching
- **Memory efficient**: Minimal memory footprint (~10-20 MB)
- **Background processing**: Non-blocking memory metrics collection (Windows)
- **Smart caching**: Extended memory info cached for 10 seconds to reduce WMI overhead

## Technical Details

### Windows-Specific Features

- **Commit Memory**: Total virtual memory committed by the OS
- **Cached Memory**: Standby/cached RAM that can be freed if needed
- **WMI Integration**: Fast WMIC queries with PowerShell fallback
- **NVML Support**: Direct GPU monitoring via NVIDIA Management Library

### Cross-Platform Support

- **Linux**: sysfs, /proc, nvidia-smi integration
- **Windows**: WMI, Performance Counters, NVML
- **Adaptive UI**: Platform-aware color schemes and layouts

## License

MIT
