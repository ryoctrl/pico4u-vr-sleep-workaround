use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tokio::time::sleep;

use crate::adb_client::{
    DeviceEntry, ProbeResult, device_command, host_command, is_connect_success, list_devices,
    tcp_probe,
};
use crate::logs::{Channel, Level, log};
use crate::state::{AdbServerState, AppState, UsbState, WirelessState, next_state, update_status};

const BASE_INTERVAL: Duration = Duration::from_secs(3);
const SLOW_INTERVAL: Duration = Duration::from_secs(10);
/// Consecutive `unreachable` results before polling slows down.
const SLOW_AFTER: u32 = 5;
/// Windows retries a SYN after a RST for roughly two seconds before reporting "refused",
/// so the probe must wait longer than that to tell `refused` from `unreachable`.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(4);

pub fn spawn(app: AppHandle) {
    let handle = tauri::async_runtime::spawn(run(app.clone()));
    let state = app.state::<AppState>();
    *state.monitor_task.lock().unwrap_or_else(|e| e.into_inner()) = Some(handle);
}

/// Asks the monitor to check right away instead of waiting for the next tick.
pub fn request_check(app: &AppHandle) {
    app.state::<AppState>().monitor_wake.notify_one();
}

async fn run(app: AppHandle) {
    let mut ctx = MonitorContext::default();
    loop {
        let wait = ctx.tick(&app).await;
        let state = app.state::<AppState>();
        tokio::select! {
            _ = sleep(wait) => {}
            _ = state.monitor_wake.notified() => {
                ctx.unreachable_streak = 0;
            }
        }
    }
}

#[derive(Default)]
struct MonitorContext {
    last_target: Option<String>,
    ready_failures: u32,
    unreachable_streak: u32,
    adb_failures: u32,
    models: HashMap<String, String>,
}

impl MonitorContext {
    /// Runs one observation and returns how long to wait before the next one.
    async fn tick(&mut self, app: &AppHandle) -> Duration {
        let state = app.state::<AppState>();
        let devices = match list_devices(app).await {
            Ok(devices) => {
                if self.adb_failures > 0 {
                    log(app, Channel::App, Level::Debug, "adb_server_ok", json!({}));
                }
                self.adb_failures = 0;
                devices
            }
            Err(e) => {
                self.adb_failures += 1;
                if self.adb_failures == 1 {
                    log(
                        app,
                        Channel::App,
                        Level::Error,
                        "adb_server_unavailable",
                        json!({ "error": e }),
                    );
                }
                update_status(app, |s| s.adb_server = AdbServerState::Unavailable);
                return adb_backoff(self.adb_failures);
            }
        };
        update_status(app, |s| s.adb_server = AdbServerState::Ok);

        self.observe_usb(app, &devices).await;

        let target = state.config().wireless_target();
        if target != self.last_target {
            self.last_target = target.clone();
            self.ready_failures = 0;
            self.unreachable_streak = 0;
            // A new target must not inherit the grace period of the old one.
            update_status(app, |s| s.wireless.state = WirelessState::Checking);
        }
        let setup_running = state.setup_running.load(Ordering::SeqCst);
        let observed = match &target {
            None => WirelessState::NoIp,
            Some(t) => observe_wireless(app, &devices, t, setup_running).await,
        };

        let prev = state.status().wireless.state;
        let shown = next_state(prev, observed, &mut self.ready_failures);
        if shown != prev {
            log(
                app,
                Channel::App,
                level_for(shown),
                "wireless_state",
                json!({ "state": shown }),
            );
        }
        log(
            app,
            Channel::App,
            Level::Debug,
            "monitor_observed",
            json!({ "state": observed }),
        );

        let checked_at = chrono::Local::now().format("%H:%M:%S").to_string();
        update_status(app, |s| {
            s.wireless.state = shown;
            s.wireless.target = target.clone();
            s.wireless.checked_at = Some(checked_at);
        });

        if observed == WirelessState::Unreachable {
            self.unreachable_streak += 1;
        } else {
            self.unreachable_streak = 0;
        }
        if self.unreachable_streak >= SLOW_AFTER {
            SLOW_INTERVAL
        } else {
            BASE_INTERVAL
        }
    }

    async fn observe_usb(&mut self, app: &AppHandle, devices: &[DeviceEntry]) {
        let (usb_state, serial) = classify_usb(devices);
        let model = match (&serial, usb_state) {
            (Some(serial), UsbState::Device) => {
                if !self.models.contains_key(serial)
                    && let Ok(out) =
                        device_command(app, serial, "shell:getprop ro.product.model").await
                {
                    self.models.insert(serial.clone(), out.trim().to_string());
                }
                self.models.get(serial).cloned()
            }
            _ => None,
        };

        let prev = app.state::<AppState>().status().usb;
        if prev.state != usb_state || prev.model != model {
            log(
                app,
                Channel::Setup,
                usb_level(usb_state),
                "usb_state",
                json!({ "state": usb_state, "model": model.clone().unwrap_or_default() }),
            );
        }
        update_status(app, |s| {
            s.usb.state = usb_state;
            s.usb.serial = serial;
            s.usb.model = model;
        });
    }
}

fn adb_backoff(failures: u32) -> Duration {
    match failures {
        0 | 1 => Duration::from_secs(3),
        2 => Duration::from_secs(6),
        3 => Duration::from_secs(12),
        _ => Duration::from_secs(30),
    }
}

fn level_for(state: WirelessState) -> Level {
    match state {
        // Normal progress is only interesting in debug mode; the status card already shows it.
        WirelessState::Ready | WirelessState::Checking | WirelessState::Connecting => Level::Debug,
        _ => Level::Warn,
    }
}

fn usb_level(state: UsbState) -> Level {
    match state {
        UsbState::Multiple | UsbState::Offline => Level::Warn,
        _ => Level::Debug,
    }
}

pub fn classify_usb(devices: &[DeviceEntry]) -> (UsbState, Option<String>) {
    let usb: Vec<&DeviceEntry> = devices.iter().filter(|d| d.is_usb()).collect();
    match usb.as_slice() {
        [] => (UsbState::None, None),
        [one] => {
            let state = match one.state.as_str() {
                "device" => UsbState::Device,
                "unauthorized" | "authorizing" => UsbState::Unauthorized,
                _ => UsbState::Offline,
            };
            (state, Some(one.serial.clone()))
        }
        _ => (UsbState::Multiple, None),
    }
}

pub fn wireless_from_adb_state(state: &str) -> WirelessState {
    match state {
        "device" => WirelessState::Ready,
        "unauthorized" | "authorizing" => WirelessState::Unauthorized,
        "connecting" => WirelessState::Connecting,
        _ => WirelessState::Offline,
    }
}

/// Determines the wireless state for `target`. Repairs the ADB connection unless setup is running,
/// in which case the setup flow owns connect/disconnect and the monitor only observes.
async fn observe_wireless(
    app: &AppHandle,
    devices: &[DeviceEntry],
    target: &str,
    setup_running: bool,
) -> WirelessState {
    let entry = devices.iter().find(|d| d.serial == target);
    if let Some(e) = entry
        && e.state == "device"
    {
        return WirelessState::Ready;
    }

    match tcp_probe(target, PROBE_TIMEOUT).await {
        ProbeResult::Refused | ProbeResult::Unreachable if setup_running => {
            WirelessState::Connecting
        }
        result @ (ProbeResult::Refused | ProbeResult::Unreachable) => {
            if entry.is_some() {
                // A stale entry would otherwise keep showing "offline" for a dead endpoint.
                let _ = host_command(app, &format!("host:disconnect:{}", target)).await;
            }
            if result == ProbeResult::Refused {
                WirelessState::Refused
            } else {
                WirelessState::Unreachable
            }
        }
        ProbeResult::Open => {
            let adb_state = entry.map(|e| wireless_from_adb_state(&e.state));
            if setup_running || adb_state == Some(WirelessState::Unauthorized) {
                // Waiting for the user to accept the prompt; reconnecting would only re-trigger it.
                return adb_state.unwrap_or(WirelessState::Connecting);
            }
            if entry.is_some() {
                // "already connected" would be returned for a stale entry without reconnecting.
                let _ = host_command(app, &format!("host:disconnect:{}", target)).await;
            }
            reconnect(app, target).await
        }
    }
}

async fn reconnect(app: &AppHandle, target: &str) -> WirelessState {
    match host_command(app, &format!("host:connect:{}", target)).await {
        Ok(out) if is_connect_success(&out) => {
            log(
                app,
                Channel::App,
                Level::Debug,
                "connect_result",
                json!({ "output": out.trim() }),
            );
        }
        Ok(out) => {
            log(
                app,
                Channel::App,
                Level::Debug,
                "connect_result",
                json!({ "output": out.trim() }),
            );
            return WirelessState::Connecting;
        }
        Err(e) => {
            log(
                app,
                Channel::App,
                Level::Debug,
                "connect_result",
                json!({ "output": e }),
            );
            return WirelessState::Connecting;
        }
    }
    match list_devices(app).await {
        Ok(devices) => devices
            .iter()
            .find(|d| d.serial == target)
            .map(|d| wireless_from_adb_state(&d.state))
            .unwrap_or(WirelessState::Connecting),
        Err(_) => WirelessState::Connecting,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(serial: &str, state: &str) -> DeviceEntry {
        DeviceEntry {
            serial: serial.into(),
            state: state.into(),
        }
    }

    #[test]
    fn usb_classification() {
        assert_eq!(classify_usb(&[]), (UsbState::None, None));
        assert_eq!(
            classify_usb(&[dev("192.168.1.10:5555", "device")]),
            (UsbState::None, None)
        );
        assert_eq!(
            classify_usb(&[dev("PA94", "device"), dev("192.168.1.10:5555", "device")]),
            (UsbState::Device, Some("PA94".into()))
        );
        assert_eq!(
            classify_usb(&[dev("PA94", "authorizing")]).0,
            UsbState::Unauthorized
        );
        assert_eq!(
            classify_usb(&[dev("PA94", "recovery")]).0,
            UsbState::Offline
        );
        assert_eq!(
            classify_usb(&[dev("PA94", "device"), dev("R58M", "device")]),
            (UsbState::Multiple, None)
        );
    }

    #[test]
    fn adb_state_mapping() {
        assert_eq!(wireless_from_adb_state("device"), WirelessState::Ready);
        assert_eq!(
            wireless_from_adb_state("unauthorized"),
            WirelessState::Unauthorized
        );
        assert_eq!(
            wireless_from_adb_state("authorizing"),
            WirelessState::Unauthorized
        );
        assert_eq!(
            wireless_from_adb_state("connecting"),
            WirelessState::Connecting
        );
        assert_eq!(wireless_from_adb_state("offline"), WirelessState::Offline);
        assert_eq!(
            wireless_from_adb_state("no permissions"),
            WirelessState::Offline
        );
    }

    #[test]
    fn adb_backoff_grows_and_caps() {
        assert_eq!(adb_backoff(1), Duration::from_secs(3));
        assert_eq!(adb_backoff(2), Duration::from_secs(6));
        assert_eq!(adb_backoff(3), Duration::from_secs(12));
        assert_eq!(adb_backoff(10), Duration::from_secs(30));
    }
}
