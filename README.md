# dMeter

A fast, beautiful terminal system monitor built with Rust and Ratatui.

## Features

- **CPU**: Per-core and overall usage with history graph
- **Memory**: RAM and swap usage visualization
- **GPU**: NVIDIA GPU monitoring (Windows with NVML)
- **Disk**: Disk usage for mounted volumes
- **Network**: Network I/O statistics
- **Processes**: Top processes by CPU/memory usage

## Installation

### From Source

```bash
cargo build --release
```

The binary will be at `target/release/dmeter.exe`.

### Prerequisites

- Windows with NVML support (for GPU monitoring)
- Rust toolchain

## Usage

```bash
dmeter              # Run with defaults
dmeter -i 1        # Refresh every 1 second
dmeter --no-gpu    # Skip GPU panel
dmeter -t F       # Temperature in Fahrenheit
```

### Controls

- `q` - Quit
- `p` - Pause/Resume updates
- `d` - Toggle disk info
- `n` - Toggle network info
- `a` - Toggle apps view
- `s` - Toggle swap display

## Configuration

Config file location: `~/.config/dmeter/config.toml`

```toml
interval = 2
temp_unit = "C"
show_swap = true
show_per_core = false
theme = "default"
```

## License

MIT
