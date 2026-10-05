use serde_json::json;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use tokio::time::sleep;

use crate::adb_client::device_command;
use crate::logs::{Channel, Level, log};
use crate::state::{AppState, WirelessState, update_status};

pub fn start(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let status = state.status();
    if status.keep_awake.running {
        return Err("keep_awake_already_running".into());
    }
    if status.wireless.state != WirelessState::Ready {
        return Err("not_ready".into());
    }

    // Interval and dim delay are read once; changes apply from the next start.
    let config = state.config();
    let interval = Duration::from_secs(config.keep_awake_interval_secs.max(1));
    let dim_after = (config.dim_delay_hours > 0.0)
        .then(|| Duration::from_secs_f64(config.dim_delay_hours * 3600.0));

    update_status(app, |s| {
        s.keep_awake.running = true;
        s.keep_awake.waiting = false;
        s.keep_awake.last_wake_at = None;
        s.keep_awake.last_check_at = None;
    });
    log(
        app,
        Channel::App,
        Level::Debug,
        "keep_awake_started",
        json!({ "interval": interval.as_secs() }),
    );

    let handle = tauri::async_runtime::spawn(run(app.clone(), interval, dim_after));
    *state
        .keep_awake_task
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some(handle);
    Ok(())
}

pub fn stop(app: &AppHandle) {
    let state = app.state::<AppState>();
    if let Some(task) = state
        .keep_awake_task
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
    {
        task.abort();
    }
    if state.status().keep_awake.running {
        log(
            app,
            Channel::App,
            Level::Debug,
            "keep_awake_stopped",
            json!({}),
        );
    }
    update_status(app, |s| {
        s.keep_awake.running = false;
        s.keep_awake.waiting = false;
    });
}

async fn run(app: AppHandle, interval: Duration, dim_after: Option<Duration>) {
    let started = Instant::now();
    let mut dimmed = false;
    // Only the first failure of a streak is logged so a persistent problem does not flood the log.
    let mut failing = false;
    loop {
        let status = app.state::<AppState>().status();
        let target = status.wireless.target.clone();
        match (status.wireless.state, target) {
            (WirelessState::Ready, Some(target)) => {
                if status.keep_awake.waiting {
                    log(
                        &app,
                        Channel::App,
                        Level::Debug,
                        "keep_awake_resumed",
                        json!({}),
                    );
                    update_status(&app, |s| s.keep_awake.waiting = false);
                }
                match send_wake(&app, &target).await {
                    Ok(()) => {
                        failing = false;
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        update_status(&app, |s| s.keep_awake.last_check_at = Some(now));
                    }
                    Err(e) => {
                        if !failing {
                            log(
                                &app,
                                Channel::App,
                                Level::Warn,
                                "wake_failed",
                                json!({ "error": e }),
                            );
                        }
                        failing = true;
                    }
                }

                if !dimmed && dim_after.is_some_and(|d| started.elapsed() >= d) {
                    dimmed = true;
                    let res = device_command(
                        &app,
                        &target,
                        "shell:settings put system screen_brightness 1",
                    )
                    .await;
                    match res {
                        Ok(_) => log(&app, Channel::App, Level::Debug, "dim_done", json!({})),
                        Err(e) => log(
                            &app,
                            Channel::App,
                            Level::Warn,
                            "dim_failed",
                            json!({ "error": e }),
                        ),
                    }
                }
            }
            (wireless, _) => {
                if !status.keep_awake.waiting {
                    log(
                        &app,
                        Channel::App,
                        Level::Warn,
                        "keep_awake_waiting",
                        json!({ "state": wireless }),
                    );
                    update_status(&app, |s| s.keep_awake.waiting = true);
                }
            }
        }
        // Sleep after the work so a slow ADB call never causes a burst of catch-up ticks.
        sleep(interval).await;
    }
}

/// Checks the power state and sends a wake key event if the headset is not awake.
async fn send_wake(app: &AppHandle, target: &str) -> Result<(), String> {
    let awake = match device_command(app, target, "shell:dumpsys power").await {
        Ok(out) => out.contains("mWakefulness=Awake"),
        Err(e) => {
            log(
                app,
                Channel::App,
                Level::Debug,
                "power_check_failed",
                json!({ "error": e }),
            );
            false
        }
    };
    if awake {
        log(app, Channel::App, Level::Debug, "already_awake", json!({}));
        return Ok(());
    }

    device_command(app, target, "shell:input keyevent 224").await?;
    let now = chrono::Local::now().format("%H:%M:%S").to_string();
    update_status(app, |s| s.keep_awake.last_wake_at = Some(now));
    log(app, Channel::App, Level::Debug, "wake_sent", json!({}));
    Ok(())
}
