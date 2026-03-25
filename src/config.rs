use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── Config structures ──────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScheduleConfig {
    /// Hour work day starts (0–23). Default: 8
    pub start_hour: u8,
    /// Minute work day starts (0–59). Default: 0
    pub start_minute: u8,
    /// Hour work day ends (0–23). Default: 16  (4 pm)
    pub end_hour: u8,
    /// Minute work day ends (0–59). Default: 0
    pub end_minute: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ColorsConfig {
    /// Hex color for filled Day blocks.    Default: "#4CAF50" (green)
    pub day_filled: String,
    /// Hex color for empty  Day blocks.    Default: "#2D2D2D"
    pub day_empty: String,
    /// Hex color for filled Week blocks.   Default: "#2196F3" (blue)
    pub week_filled: String,
    /// Hex color for empty  Week blocks.   Default: "#2D2D2D"
    pub week_empty: String,
    /// Window background color.            Default: "#1A1A1A"
    pub background: String,
    /// Label and percentage text color.    Default: "#CCCCCC"
    pub text_color: String,
    /// Block fill color on weekends.       Default: "#9C27B0" (purple)
    pub weekend_filled: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DisplayConfig {
    /// Number of progress blocks per row (1–20). Default: 10
    pub blocks: u8,
    /// Show "XX%" percentage after each row.     Default: true
    pub show_percentage: bool,
    /// Text shown on weekends instead of a %.    Default: "Weekend!"
    pub weekend_label: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub schedule: ScheduleConfig,
    pub colors: ColorsConfig,
    pub display: DisplayConfig,
}

// ── Defaults ───────────────────────────────────────────────────────────────

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            start_hour: 8,
            start_minute: 0,
            end_hour: 16,
            end_minute: 0,
        }
    }
}

impl Default for ColorsConfig {
    fn default() -> Self {
        Self {
            day_filled: "#4CAF50".into(),
            day_empty: "#2D2D2D".into(),
            week_filled: "#2196F3".into(),
            week_empty: "#2D2D2D".into(),
            background: "#1A1A1A".into(),
            text_color: "#CCCCCC".into(),
            weekend_filled: "#9C27B0".into(),
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            blocks: 10,
            show_percentage: true,
            weekend_label: "Weekend!".into(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schedule: ScheduleConfig::default(),
            colors: ColorsConfig::default(),
            display: DisplayConfig::default(),
        }
    }
}

// ── File I/O ───────────────────────────────────────────────────────────────

/// Returns the path to the config file:
///   %APPDATA%\WorkTimeVisualizer\config.json
pub fn config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("WorkTimeVisualizer");
    path.push("config.json");
    path
}

/// Load config from disk, creating a default one if it does not exist.
pub fn load() -> Config {
    let path = config_path();

    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<Config>(&text) {
                Ok(cfg) => return cfg,
                Err(e) => {
                    // Malformed JSON – log to stderr and fall through to default
                    eprintln!("work-time-visualizer: config parse error: {e}");
                }
            },
            Err(e) => eprintln!("work-time-visualizer: config read error: {e}"),
        }
    }

    // Write default config so the user can edit it
    let default = Config::default();
    let _ = save(&default);
    default
}

/// Persist config to disk.
pub fn save(cfg: &Config) -> std::io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(cfg).expect("serialize config");
    std::fs::write(path, json)
}
