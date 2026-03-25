# Work Time Visualizer

A lightweight Windows taskbar widget that shows your daily and weekly work-time progress as colored block bars — built in Rust using the Win32 API.

![widget showing Day and Week progress bars](docs/screenshot.png)

## What it does

```
Day  [■][■][■][■][■][□][□][□][□][□]  50%
Wk   [■][■][■][■][■][■][□][□][□][□]  62%
```

- **Day bar** — progress from your configured start time (default 8 am) to end time (default 4 pm). Shows 0% before work starts and 100% after it ends.
- **Week bar** — progress from Monday 8 am through Friday 4 pm.
- **Weekends** — both bars fill to 100% and show a configurable label (default `Weekend!`).
- Refreshes every 15 seconds automatically.
- Right-click the widget to open the config file or exit.

## Requirements

- Windows 10 or 11 (x64)
- [Rust toolchain](https://rustup.rs) (stable)

## Build

```powershell
git clone https://github.com/<your-username>/work-time-visualizer-rust.git
cd work-time-visualizer-rust
cargo build --release
```

The compiled binary will be at `target\release\work-time-visualizer.exe`.

## Run

```powershell
cargo run
# or after building:
.\target\release\work-time-visualizer.exe
```

The widget appears at the bottom-right of your screen, just above the taskbar.

## Configuration

On first run a config file is created at:

```
%APPDATA%\WorkTimeVisualizer\config.json
```

Right-click the widget and choose **Settings** to open it in your default editor.

```json
{
  "schedule": {
    "start_hour": 8,
    "start_minute": 0,
    "end_hour": 16,
    "end_minute": 0
  },
  "colors": {
    "day_filled":     "#4CAF50",
    "day_empty":      "#2D2D2D",
    "week_filled":    "#2196F3",
    "week_empty":     "#2D2D2D",
    "background":     "#1A1A1A",
    "text_color":     "#CCCCCC",
    "weekend_filled": "#9C27B0"
  },
  "display": {
    "blocks":          10,
    "show_percentage": true,
    "weekend_label":   "Weekend!"
  }
}
```

Restart the app after saving changes.

### Color reference

| Key | Default | Controls |
|-----|---------|----------|
| `day_filled` | `#4CAF50` | Filled blocks on the Day bar |
| `day_empty` | `#2D2D2D` | Empty blocks on the Day bar |
| `week_filled` | `#2196F3` | Filled blocks on the Week bar |
| `week_empty` | `#2D2D2D` | Empty blocks on the Week bar |
| `background` | `#1A1A1A` | Widget background |
| `text_color` | `#CCCCCC` | Labels and percentage text |
| `weekend_filled` | `#9C27B0` | Block color on weekends |

## Run on startup

1. Build the release binary.
2. Press `Win + R`, type `shell:startup`, press Enter.
3. Create a shortcut to `work-time-visualizer.exe` in that folder.

## Project structure

```
src/
├── main.rs           # Entry point + single-instance guard
├── window.rs         # Win32 window creation, message loop, GDI renderer
├── config.rs         # JSON config — load/save with serde
├── time_calc.rs      # Daily and weekly progress math (uses GetLocalTime)
├── theme.rs          # Windows dark/light mode detection via registry
└── native_interop.rs # Win32 helpers: Color, taskbar rect, DPI, etc.
```

## License

MIT
