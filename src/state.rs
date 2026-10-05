use serde::Serialize;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Notify;

use crate::config::AppConfig;
use crate::logs::LogBuffer;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WirelessState {
    /// Waiting for the first monitor result.
    Checking,
    NoIp,
    Unreachable,
    Refused,
    Connecting,
    Unauthorized,
    Offline,
    Ready,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UsbState {
    None,
    Unauthorized,
    Offline,
    Device,
    Multiple,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdbServerState {
    Ok,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WirelessStatus {
    pub state: WirelessState,
    pub target: Option<String>,
    pub checked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct UsbStatus {
    pub state: UsbState,
    pub serial: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct KeepAwakeStatus {
    pub running: bool,
    /// Running, but the headset is not reachable so nothing is being sent.
    pub waiting: bool,
    pub last_wake_at: Option<String>,
    /// Last cycle that reached the headset (power state checked or wake sent).
    pub last_check_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ConnectionStatus {
    pub adb_server: AdbServerState,
    pub wireless: WirelessStatus,
    pub usb: UsbStatus,
    pub keep_awake: KeepAwakeStatus,
    pub setup_running: bool,
    /// Outcome of the last Wireless Debug setup in this session (`None` until one finishes).
    pub setup_succeeded: Option<bool>,
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        Self {
            adb_server: AdbServerState::Ok,
            wireless: WirelessStatus {
                state: WirelessState::Checking,
                target: None,
                checked_at: None,
            },
            usb: UsbStatus {
                state: UsbState::None,
                serial: None,
                model: None,
            },
            keep_awake: KeepAwakeStatus::default(),
            setup_running: false,
            setup_succeeded: None,
        }
    }
}

pub struct AppState {
    pub status: Mutex<ConnectionStatus>,
    pub config: Mutex<AppConfig>,
    /// Wakes the monitor for an immediate check (config change, setup finished).
    pub monitor_wake: Notify,
    pub monitor_task: Mutex<Option<JoinHandle<()>>>,
    pub keep_awake_task: Mutex<Option<JoinHandle<()>>>,
    pub debug_mode: AtomicBool,
    pub setup_running: AtomicBool,
    pub adb_started_by_us: AtomicBool,
    pub logs: LogBuffer,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            status: Mutex::new(ConnectionStatus::default()),
            config: Mutex::new(AppConfig::default()),
            monitor_wake: Notify::new(),
            monitor_task: Mutex::new(None),
            keep_awake_task: Mutex::new(None),
            debug_mode: AtomicBool::new(false),
            setup_running: AtomicBool::new(false),
            adb_started_by_us: AtomicBool::new(false),
            logs: LogBuffer::default(),
        }
    }
}

impl AppState {
    pub fn config(&self) -> AppConfig {
        self.config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn status(&self) -> ConnectionStatus {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

/// Applies `f` to the shared status and notifies the UI if anything changed.
pub fn update_status(app: &AppHandle, f: impl FnOnce(&mut ConnectionStatus)) {
    let state = app.state::<AppState>();
    let snapshot = {
        let mut guard = state.status.lock().unwrap_or_else(|e| e.into_inner());
        let before = guard.clone();
        f(&mut guard);
        if *guard == before {
            return;
        }
        guard.clone()
    };
    let _ = app.emit("connection-status", snapshot);
}

/// Number of consecutive failed observations tolerated before leaving `Ready`,
/// so a short Wi-Fi hiccup does not flip the UI back and forth.
pub const READY_GRACE: u32 = 2;

/// Decides the displayed state from the previous one and a fresh observation.
/// `failures` counts consecutive non-ready observations while the previous state was `Ready`.
pub fn next_state(
    prev: WirelessState,
    observed: WirelessState,
    failures: &mut u32,
) -> WirelessState {
    if observed == WirelessState::Ready || observed == WirelessState::NoIp {
        *failures = 0;
        return observed;
    }
    if prev == WirelessState::Ready {
        *failures += 1;
        if *failures < READY_GRACE {
            return WirelessState::Ready;
        }
    }
    *failures = 0;
    observed
}

#[cfg(test)]
mod tests {
    use super::WirelessState::*;
    use super::*;

    #[test]
    fn ready_survives_a_single_failure() {
        let mut failures = 0;
        assert_eq!(next_state(Ready, Unreachable, &mut failures), Ready);
        assert_eq!(next_state(Ready, Unreachable, &mut failures), Unreachable);
        assert_eq!(failures, 0);
    }

    #[test]
    fn recovery_to_ready_is_immediate() {
        let mut failures = 0;
        assert_eq!(next_state(Refused, Ready, &mut failures), Ready);
        assert_eq!(next_state(Checking, Ready, &mut failures), Ready);
    }

    #[test]
    fn failure_counter_resets_on_success() {
        let mut failures = 0;
        assert_eq!(next_state(Ready, Offline, &mut failures), Ready);
        assert_eq!(next_state(Ready, Ready, &mut failures), Ready);
        assert_eq!(next_state(Ready, Offline, &mut failures), Ready);
        assert_eq!(next_state(Ready, Offline, &mut failures), Offline);
    }

    #[test]
    fn non_ready_states_follow_observation() {
        let mut failures = 0;
        assert_eq!(
            next_state(Checking, Unreachable, &mut failures),
            Unreachable
        );
        assert_eq!(next_state(Unreachable, Refused, &mut failures), Refused);
        assert_eq!(
            next_state(Refused, Unauthorized, &mut failures),
            Unauthorized
        );
    }

    #[test]
    fn clearing_ip_is_immediate() {
        let mut failures = 0;
        assert_eq!(next_state(Ready, NoIp, &mut failures), NoIp);
    }
}
