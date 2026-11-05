//! Persistent Application Settings (`settings.rs`).
//!
//! Handles saving and loading persistent user configuration:
//! - Runtime language / locale
//! - Dark / Light theme preference
//! - Gridlines visibility
//! - Wire connection (solder dot) visibility
//! - Oscilloscope panel visibility
//! - Circuit export grid and color preferences
//!
//! Settings are saved to standard user directories:
//! - Windows: `%APPDATA%\LogicCraft\settings.json`
//! - macOS: `~/Library/Application Support/LogicCraft/settings.json`
//! - Linux/Other: `~/.config/logiccraft/settings.json`

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::i18n::Locale;
use crate::mechanics::{GRIDLINE_VISIBLE, WIRE_CONN_VISIBLE};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_locale")]
    pub locale: Locale,
    #[serde(default = "default_true")]
    pub dark_mode: bool,
    #[serde(default = "default_gridlines")]
    pub show_gridlines: bool,
    #[serde(default = "default_wire_conn")]
    pub show_wire_conn: bool,
    #[serde(default = "default_true")]
    pub show_oscope: bool,
    #[serde(default = "default_true")]
    pub export_with_grid: bool,
    #[serde(default = "default_true")]
    pub export_with_color: bool,
}

fn default_true() -> bool {
    true
}

fn default_gridlines() -> bool {
    GRIDLINE_VISIBLE
}

fn default_wire_conn() -> bool {
    WIRE_CONN_VISIBLE
}

fn default_locale() -> Locale {
    Locale::En
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            locale: Locale::En,
            dark_mode: true,
            show_gridlines: GRIDLINE_VISIBLE,
            show_wire_conn: WIRE_CONN_VISIBLE,
            show_oscope: true,
            export_with_grid: true,
            export_with_color: true,
        }
    }
}

pub fn settings_path() -> PathBuf {
    #[cfg(windows)]
    if let Ok(app_data) = std::env::var("APPDATA") {
        let dir = PathBuf::from(app_data).join("LogicCraft");
        let _ = std::fs::create_dir_all(&dir);
        return dir.join("settings.json");
    }

    #[cfg(target_os = "macos")]
    if let Ok(home) = std::env::var("HOME") {
        let dir = PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("LogicCraft");
        let _ = std::fs::create_dir_all(&dir);
        return dir.join("settings.json");
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
        let dir = PathBuf::from(config_home).join("logiccraft");
        let _ = std::fs::create_dir_all(&dir);
        return dir.join("settings.json");
    } else if let Ok(home) = std::env::var("HOME") {
        let dir = PathBuf::from(home).join(".config").join("logiccraft");
        let _ = std::fs::create_dir_all(&dir);
        return dir.join("settings.json");
    }

    PathBuf::from("settings.json")
}

impl AppSettings {
    pub fn load() -> Self {
        let path = settings_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(settings) = serde_json::from_str::<AppSettings>(&content) {
                    return settings;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let path = settings_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, content);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_serialization_roundtrip() {
        let original = AppSettings {
            locale: Locale::Pt,
            dark_mode: false,
            show_gridlines: false,
            show_wire_conn: true,
            show_oscope: false,
            export_with_grid: false,
            export_with_color: true,
        };

        let json = serde_json::to_string_pretty(&original).expect("serialize settings");
        let loaded: AppSettings = serde_json::from_str(&json).expect("deserialize settings");

        assert_eq!(loaded.locale, Locale::Pt);
        assert_eq!(loaded.dark_mode, false);
        assert_eq!(loaded.show_gridlines, false);
        assert_eq!(loaded.show_wire_conn, true);
        assert_eq!(loaded.show_oscope, false);
        assert_eq!(loaded.export_with_grid, false);
        assert_eq!(loaded.export_with_color, true);
    }

    #[test]
    fn test_settings_default_fallback_on_partial_json() {
        let partial_json = r#"{"locale": "De", "dark_mode": false}"#;
        let loaded: AppSettings = serde_json::from_str(partial_json).expect("deserialize partial");

        assert_eq!(loaded.locale, Locale::De);
        assert_eq!(loaded.dark_mode, false);
        assert_eq!(loaded.show_gridlines, true);
        assert_eq!(loaded.show_wire_conn, true);
        assert_eq!(loaded.show_oscope, true);
    }
}
