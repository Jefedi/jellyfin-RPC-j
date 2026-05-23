use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub minimize_to_tray: bool,
    pub start_minimized: bool,
    pub start_with_windows: bool,
    pub autostart_rpc: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            minimize_to_tray: true,
            start_minimized: false,
            start_with_windows: false,
            autostart_rpc: false,
        }
    }
}

impl Settings {
    pub fn path() -> PathBuf {
        super::config::config_dir().join("gui_settings.json")
    }

    pub fn load() -> Self {
        let path = Self::path();
        match std::fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let s = serde_json::to_string_pretty(self).unwrap();
        std::fs::write(path, s)
    }
}
