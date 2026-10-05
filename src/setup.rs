use serde_json::json;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use tokio::time::sleep;

use crate::adb_client::{
    ProbeResult, device_command, host_command, is_connect_success, list_devices, parse_route_src,
    parse_wlan_ip, tcp_probe,
};
use crate::config::{ADB_TCP_PORT, AppConfig};
use crate::logs::{Channel, Level, log};
use crate::monitor::{self, classify_usb};
use crate::state::{AppState, UsbState, update_status};

const PORT_WAIT: Duration = Duration::from_secs(10);

/// Enables `adb tcpip` over USB and connects to the headset wirelessly.
/// The monitor stays in observe-only mode while this runs.
pub async fn enable_wireless_debug(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    if state.setup_running.swap(true, Ordering::SeqCst) {
        return Err("setup_already_running".to_string());
    }
    update_status(&app, |s| s.setup_running = true);
    log(
        &app,
        Channel::Setup,
        Level::Info,
        "setup_started",
        json!({}),
    );

    let result = run(&app).await;
    match &result {
        Ok(target) => log(
            &app,
            Channel::Setup,
            Level::Info,
            "setup_done",
            json!({ "target": target }),
        ),
        Err(e) => log(
            &app,
            Channel::Setup,
            Level::Error,
            "setup_failed",
            json!({ "error": e }),
        ),
    }

    state.setup_running.store(false, Ordering::SeqCst);
    update_status(&app, |s| s.setup_running = false);
    monitor::request_check(&app);
    result.map(|_| ())
}

async fn run(app: &AppHandle) -> Result<String, String> {
    // 1. Exactly one authorized USB device.
    let devices = list_devices(app).await?;
    let (usb_state, serial) = classify_usb(&devices);
    let serial = match (usb_state, serial) {
        (UsbState::Device, Some(serial)) => serial,
        (UsbState::None, _) => return Err("usb_none".into()),
        (UsbState::Multiple, _) => return Err("usb_multiple".into()),
        (UsbState::Unauthorized, _) => return Err("usb_unauthorized".into()),
        _ => return Err("usb_offline".into()),
    };
    log(
        app,
        Channel::Setup,
        Level::Info,
        "setup_usb_found",
        json!({ "serial": serial }),
    );

    // 2. Read the Wi-Fi address before tcpip restarts adbd.
    let ip = get_device_ip(app, &serial).await?;
    log(
        app,
        Channel::Setup,
        Level::Info,
        "setup_ip_found",
        json!({ "ip": ip }),
    );
    let target = format!("{}:{}", ip, ADB_TCP_PORT);
    store_ip(app, &ip).await?;

    // 3. Switch adbd to TCP mode.
    let out = device_command(app, &serial, &format!("tcpip:{}", ADB_TCP_PORT)).await?;
    log(
        app,
        Channel::Setup,
        Level::Info,
        "setup_tcpip",
        json!({ "output": out.trim() }),
    );

    // 4. Wait for the port to open.
    let started = Instant::now();
    loop {
        match tcp_probe(&target, monitor::PROBE_TIMEOUT).await {
            ProbeResult::Open => break,
            _ if started.elapsed() >= PORT_WAIT => {
                return Err(format!("port_timeout:{}", target));
            }
            _ => sleep(Duration::from_millis(500)).await,
        }
    }
    log(
        app,
        Channel::Setup,
        Level::Info,
        "setup_port_open",
        json!({ "target": target }),
    );

    // 5. Connect and confirm.
    let _ = host_command(app, &format!("host:disconnect:{}", target)).await;
    let out = host_command(app, &format!("host:connect:{}", target)).await?;
    if !is_connect_success(&out) {
        return Err(out.trim().to_string());
    }
    let devices = list_devices(app).await?;
    match devices
        .iter()
        .find(|d| d.serial == target)
        .map(|d| d.state.as_str())
    {
        Some("device") => Ok(target),
        Some("unauthorized") | Some("authorizing") => Err("wireless_unauthorized".into()),
        Some(other) => Err(format!("wireless_state:{}", other)),
        None => Err("wireless_missing".into()),
    }
}

async fn get_device_ip(app: &AppHandle, serial: &str) -> Result<String, String> {
    if let Ok(out) = device_command(app, serial, "shell:ip -f inet addr show wlan0").await
        && let Some(ip) = parse_wlan_ip(&out)
    {
        return Ok(ip);
    }
    let out = device_command(app, serial, "shell:ip route get 1.1.1.1").await?;
    parse_route_src(&out).ok_or_else(|| "ip_not_found".to_string())
}

async fn store_ip(app: &AppHandle, ip: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut config = state.config();
    if config.ip_address == ip {
        return Ok(());
    }
    config.ip_address = ip.to_string();
    apply_config(app, config).await
}

/// Persists `config`, makes it live and disconnects a previous wireless target if it changed.
pub async fn apply_config(app: &AppHandle, mut config: AppConfig) -> Result<(), String> {
    config.sanitize();
    crate::config::save_config(app, &config)?;

    let state = app.state::<AppState>();
    let old_target = {
        let mut guard = state.config.lock().unwrap_or_else(|e| e.into_inner());
        let old = guard.wireless_target();
        *guard = config.clone();
        old
    };
    state.debug_mode.store(config.debug_mode, Ordering::SeqCst);

    let new_target = config.wireless_target();
    if old_target != new_target {
        if let Some(old) = old_target {
            let _ = host_command(app, &format!("host:disconnect:{}", old)).await;
        }
        log(
            app,
            Channel::App,
            Level::Info,
            "ip_changed",
            json!({ "target": new_target.clone().unwrap_or_default() }),
        );
    }
    monitor::request_check(app);
    Ok(())
}
