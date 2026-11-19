//! Per-computer settings: recently opened projects and the last folder.
//! Stored in `<config dir>/blueprint/settings.json`; `BLUEPRINT_CONFIG_DIR`
//! points somewhere else (tests never touch the real settings).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Recent projects listed on the start page.
pub const MAX_RECENT: usize = 10;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub recent: Vec<PathBuf>,
    pub last_dir: Option<PathBuf>,
}

/// The settings folder.
pub fn config_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("BLUEPRINT_CONFIG_DIR") {
        return Some(PathBuf::from(d));
    }
    Some(dirs::config_dir()?.join("blueprint"))
}

/// Where an untitled project is autosaved.
pub fn untitled_autosave() -> Option<PathBuf> {
    Some(config_dir()?.join("untitled.blueprint.autosave"))
}

impl Settings {
    pub fn load() -> Settings {
        config_dir()
            .map(|d| d.join("settings.json"))
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(dir) = config_dir() else { return };
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = bp_core::project::atomic_write(&dir.join("settings.json"), text.as_bytes());
        }
    }

    /// Put `path` first in the recent list.
    pub fn remember(&mut self, path: &Path) {
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_path_buf());
        self.recent.truncate(MAX_RECENT);
        self.last_dir = path.parent().map(Path::to_path_buf);
        self.save();
    }
}
