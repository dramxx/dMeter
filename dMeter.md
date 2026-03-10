# dmeter — System Monitor for Your Rig

> A fast, beautiful terminal dashboard written in Rust.  
> One binary. No dependencies. Just run `dmeter`.

---

## Philosophy

- **Clean over dense** — breathing room, intentional layout, not a wall of numbers
- **Organized by importance** — most critical vitals at a glance, details below
- **Color as signal** — green → yellow → red thresholds, not decoration
- **Inspired aesthetics** — minimal, high-contrast, OpenCode-style typography feel
- **Rust-native** — single compiled binary, no runtime, no Python, no nothing

---

## Tech Stack

| Concern                               | Crate                                       |
| ------------------------------------- | ------------------------------------------- |
| TUI rendering                         | `ratatui`                                   |
| Terminal backend                      | `crossterm` (Windows PowerShell compatible) |
| System info (CPU, RAM, disk, network) | `sysinfo`                                   |
| GPU info (NVIDIA)                     | `nvml-wrapper`                              |
| GPU info (AMD fallback)               | `wgpu` or WMI query                         |
| Windows-specific extras (fan, temps)  | `wmi` crate via COM                         |
| Async runtime                         | `tokio`                                     |
| Config file                           | `toml` + `dirs`                             |
| CLI args                              | `clap`                                      |

---

## Distribution

- Compiled to a single `.exe` (Windows) via `cargo build --release`
- Installable via `cargo install dmeter` (crates.io)
- Or: download prebuilt binary, add to PATH → call `dmeter` anywhere in PowerShell
- No installer, no admin rights required

---

## Terminal Behavior

- **Full takeover on launch** — uses alternate screen buffer via `crossterm`, hiding the shell prompt entirely (same as htop/glances)
- **Full width AND height** — always fills 100% of the terminal window
- **Dynamic resize handling** — listens for terminal resize events, panels reflow and rescale instantly
- **Minimum size warning** — if terminal is too small (< 80×24), show a centered message asking to resize instead of broken layout
- **Clean exit** — on `q` or `Ctrl+C`, restores the original screen buffer and cursor, leaving the terminal exactly as it was

---

## Layout Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│  dmeter                              HOSTNAME  |  Mon 09 Mar  15:33  │
├────────────────┬─────────────────┬─────────────────────────────────--┤
│   CPU          │   GPU           │   MEMORY                          │
│   [bar]        │   [bar]         │   [bar]                           │
│   name/speed   │   name/vram     │   used / total                    │
│   temp / cores │   temp / fan    │   SWAP [bar]                      │
├────────────────┬─────────────────┴──────────────────────────────────-┤
│   CPU history  │   GPU history  (sparklines — auto-shown if height   │
│   ▁▂▄▆▇▆▄▃▂▁  │   ▁▁▂▃▅▇▆▅▃▂   allows, hidden if terminal too short)│
├────────────────┴─────────────────┬───────────────────────────────────┤
│   NETWORK                        │   DISK                            │
│   adapter name · IP              │   C:\  [bar]  used / total        │
│   ↑ upload  ↓ download           │   (each mounted drive)            │
├──────────────────────────────────┴───────────────────────────────────┤
│   SYSTEM  ·  Uptime  ·  OS  ·  Kernel  ·  Load avg                  │
└──────────────────────────────────────────────────────────────────────┘
```

### Proportional Scaling Rules

| Available height | Behavior                                         |
| ---------------- | ------------------------------------------------ |
| < 24 rows        | Minimum size warning shown                       |
| 24–35 rows       | Compact mode — no sparklines, tighter padding    |
| 36–50 rows       | Standard mode — sparklines appear under CPU/GPU  |
| 50+ rows         | Spacious mode — larger bars, more breathing room |

Panels always fill full width. Vertical space is distributed proportionally across sections.

---

## Panels & Data

### 1. Header Bar (always visible)

- App name `dmeter` (left)
- Hostname · Date · Time (right)
- Refreshes every tick

### 2. CPU Panel

- CPU model name + base/boost clock
- Usage % — animated bar with color threshold
- Per-core usage (compact mini-bars if space allows)
- Temperature (°C)
- Core count (physical / logical)

### 3. GPU Panel

- GPU model name
- GPU usage % — bar
- VRAM used / total — bar
- GPU temperature (°C)
- Fan speed (RPM or %)
- Power draw (W) if available
- Graceful fallback if no NVIDIA / no NVML

### 4. Memory Panel

- RAM: used / total — bar
- SWAP: used / total — bar
- Friendly labels (e.g. `12.8 GB / 31.1 GB`)

### 5. Network Panel

- Active adapter name
- IP address (v4)
- Upload speed (↑ KB/s or MB/s, auto-scaled)
- Download speed (↓ KB/s or MB/s, auto-scaled)
- Total sent / received this session

### 6. Disk Panel

- Each mounted drive listed
- Used / Total with usage bar
- Drive label + filesystem type

### 7. System Info Bar (bottom)

- OS name + version
- Uptime (d h m)
- Load average (1 / 5 / 15 min)

---

## Color Thresholds

| Level    | Range   | Color  |
| -------- | ------- | ------ |
| OK       | 0–60%   | Green  |
| Warn     | 60–85%  | Yellow |
| Critical | 85–100% | Red    |

Temperature thresholds (CPU/GPU):
| Level | Range | Color |
|---|---|---|
| Cool | < 60°C | Green |
| Warm | 60–80°C | Yellow |
| Hot | > 80°C | Red |

---

## Refresh Behavior

- Default tick rate: **2 seconds**
- Configurable via CLI flag: `dmeter --interval 1`
- Network speeds calculated as delta between ticks (bytes/tick → per second)

---

## Configuration (optional, `~/.config/dmeter/config.toml`)

```toml
interval = 2          # refresh seconds
temp_unit = "C"       # "C" or "F"
theme = "default"     # future: "dark", "light", "minimal"
show_swap = true
show_per_core = false # show per-core CPU breakdown
```

---

## CLI Interface

```
dmeter                    # run with defaults
dmeter --interval 1       # 1 second refresh
dmeter --no-gpu           # skip GPU panel (e.g. no NVIDIA)
dmeter --version          # print version
dmeter --help             # help
```

---

## Keyboard Shortcuts (in-app)

| Key            | Action                  |
| -------------- | ----------------------- |
| `q` / `Ctrl+C` | Quit                    |
| `p`            | Pause / resume refresh  |
| `r`            | Force immediate refresh |

---

## Platform Notes

- **Primary target**: Windows 10/11, PowerShell
- **Secondary**: Linux (most features work via `sysinfo`)
- GPU support: NVIDIA via NVML. AMD support is best-effort via WMI on Windows.
- Fan speed on non-NVIDIA hardware depends on WMI sensor availability

---

## Future Ideas (not in v1)

- Temperature history sparkline
- Alert/beep on critical threshold breach
- Multiple GPU support
- Network interface selector
- Dark/light/custom themes
- `dmeter --export json` for scripting
