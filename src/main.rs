#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod adb_client;
mod commands;
mod config;
mod keep_awake;
mod logs;
mod monitor;
mod setup;
mod state;

use crate::commands::*;
use crate::config::{LoadWarning, load_config};
use crate::logs::{Channel, Level, log};
use crate::state::AppState;
use serde_json::json;
use std::sync::atomic::Ordering;
use tauri::Manager;
use tauri_plugin_shell::ShellExt;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .manage(AppState::default())
        .setup(|app| {
            let handle = app.handle().clone();
            let (config, warning) = load_config(&handle);
            {
                let state = handle.state::<AppState>();
                state.debug_mode.store(config.debug_mode, Ordering::SeqCst);
                *state.config.lock().unwrap_or_else(|e| e.into_inner()) = config;
            }
            match warning {
                Some(LoadWarning::Corrupted) => log(
                    &handle,
                    Channel::App,
                    Level::Warn,
                    "config_corrupted",
                    json!({}),
                ),
                Some(LoadWarning::Sanitized) => log(
                    &handle,
                    Channel::App,
                    Level::Warn,
                    "config_sanitized",
                    json!({}),
                ),
                None => {}
            }
            monitor::spawn(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_logs,
            clear_logs,
            get_config,
            save_config_cmd,
            enable_wireless_debug,
            start_keep_awake,
            stop_keep_awake
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                let state = app_handle.state::<AppState>();
                for slot in [&state.keep_awake_task, &state.monitor_task] {
                    if let Some(task) = slot.lock().unwrap_or_else(|e| e.into_inner()).take() {
                        task.abort();
                    }
                }
                if state.adb_started_by_us.load(Ordering::SeqCst)
                    && let Ok(sidecar) = app_handle.shell().sidecar("adb")
                {
                    tauri::async_runtime::block_on(async {
                        let _ = sidecar.args(["kill-server"]).output().await;
                    });
                }
            }
        });
}
