# dMeter

A fast, beautiful terminal system monitor built with Rust and Ratatui, featuring **Conway's Game of Life** as the centerpiece.

## Features

### Conway's Game of Life <3

### System Monitoring

- **CPU**: Real-time usage with temperature, fan speed, and power monitoring
- **Memory**: RAM usage with progress bars and extended memory tracking (Windows)
- **GPU**: NVIDIA GPU monitoring with VRAM tracking and history graphs
- **Disk**: Multi-disk usage for all mounted volumes with responsive bars
- **Network**: Real-time network I/O statistics with history sparklines
- **Disk I/O**: Read/write speeds monitoring with visual graphs

### Visualization

- **History Graphs**: CPU, RAM, GPU, VRAM, Network, and Disk I/O sparklines
- **Game of Life**: The star feature - mesmerizing cellular automaton with half-block rendering for 2x vertical resolution
- **Color-coded Metrics**: Intuitive color scheme for quick status assessment
- **Dynamic Layout**: Automatically adapts UI when no GPU is detected - widgets expand to fill freed space
- **Responsive Design**: Adapts to terminal size with proper spacing

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
```

### Controls

- `q` or `Ctrl+C` - Quit
- `r` - Force refresh

## Configuration

Config file location: `~/.config/dmeter/config.toml` (Linux/macOS) or `%APPDATA%\dmeter\config.toml` (Windows)

```toml
interval = 2        # Refresh interval in seconds (default: 2)
```

### Command-Line Options

```bash
dmeter --interval 5  # Set refresh interval to 5 seconds
dmeter -i 3          # Short form
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
