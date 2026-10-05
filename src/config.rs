use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub const ADB_TCP_PORT: u16 = 5555;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct AppConfig {
    pub dim_delay_hours: f64,
    pub ip_address: String,
    pub keep_awake_interval_secs: u64,
    pub debug_mode: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            dim_delay_hours: 1.0,
            ip_address: String::new(),
            keep_awake_interval_secs: 3,
            debug_mode: false,
        }
    }
}

impl AppConfig {
    /// `ip` or `ip:port` normalized to the ADB serial used for wireless transport.
    pub fn wireless_target(&self) -> Option<String> {
        let ip = self.ip_address.trim();
        if ip.is_empty() {
            None
        } else if ip.contains(':') {
            Some(ip.to_string())
        } else {
            Some(format!("{}:{}", ip, ADB_TCP_PORT))
        }
    }

    /// Clamps values that would break the keep-awake loop. Returns true if anything changed.
    pub fn sanitize(&mut self) -> bool {
        let before = self.clone();
        self.ip_address = self.ip_address.trim().to_string();
        if self.keep_awake_interval_secs == 0 {
            self.keep_awake_interval_secs = 1;
        }
        if !self.dim_delay_hours.is_finite() || self.dim_delay_hours < 0.0 {
            self.dim_delay_hours = 0.0;
        }
        *self != before
    }
}

pub enum LoadWarning {
    Corrupted,
    Sanitized,
}

fn get_config_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|p| p.join("pico4u_config.json"))
}

pub fn load_config(app: &AppHandle) -> (AppConfig, Option<LoadWarning>) {
    let Some(path) = get_config_path(app) else {
        return (AppConfig::default(), None);
    };
    let Ok(content) = fs::read_to_string(&path) else {
        return (AppConfig::default(), None);
    };
    match serde_json::from_str::<AppConfig>(&content) {
        Ok(mut config) => {
            let warning = config.sanitize().then_some(LoadWarning::Sanitized);
            (config, warning)
        }
        Err(_) => {
            // Keep the broken file around for inspection instead of silently overwriting it later.
            let _ = fs::rename(&path, path.with_extension("json.bak"));
            (AppConfig::default(), Some(LoadWarning::Corrupted))
        }
    }
}

pub fn save_config(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let path = get_config_path(app).ok_or("Failed to resolve config directory")?;
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    // Write then rename so a crash mid-write never leaves a truncated config behind.
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, content).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_config_loads_with_new_defaults() {
        let legacy = r#"{"dim_delay_hours":2.0,"ip_address":"192.168.1.10","keep_awake_interval_secs":5,"last_connection_mode":"wireless"}"#;
        let config: AppConfig = serde_json::from_str(legacy).unwrap();
        assert_eq!(config.ip_address, "192.168.1.10");
        assert_eq!(config.keep_awake_interval_secs, 5);
        assert!(!config.debug_mode);
    }

    #[test]
    fn wireless_target_appends_default_port() {
        let mut config = AppConfig::default();
        assert_eq!(config.wireless_target(), None);
        config.ip_address = " 192.168.1.10 ".into();
        assert_eq!(
            config.wireless_target().as_deref(),
            Some("192.168.1.10:5555")
        );
        config.ip_address = "192.168.1.10:5556".into();
        assert_eq!(
            config.wireless_target().as_deref(),
            Some("192.168.1.10:5556")
        );
    }

    #[test]
    fn sanitize_fixes_invalid_values() {
        let mut config = AppConfig {
            keep_awake_interval_secs: 0,
            dim_delay_hours: f64::NAN,
            ..AppConfig::default()
        };
        assert!(config.sanitize());
        assert_eq!(config.keep_awake_interval_secs, 1);
        assert_eq!(config.dim_delay_hours, 0.0);

        config.dim_delay_hours = -1.0;
        assert!(config.sanitize());
        assert_eq!(config.dim_delay_hours, 0.0);

        let mut ok = AppConfig::default();
        assert!(!ok.sanitize());
    }
}
