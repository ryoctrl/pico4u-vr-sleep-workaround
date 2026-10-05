use tauri::{AppHandle, State};

use crate::config::AppConfig;
use crate::logs::{Channel, LogEntry};
use crate::state::{AppState, ConnectionStatus};
use crate::{keep_awake, setup};

#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> ConnectionStatus {
    state.status()
}

#[tauri::command]
pub fn get_logs(state: State<'_, AppState>) -> Vec<LogEntry> {
    state.logs.snapshot()
}

#[tauri::command]
pub fn clear_logs(state: State<'_, AppState>, channel: Channel) {
    state.logs.clear(channel);
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> AppConfig {
    state.config()
}

#[tauri::command]
pub async fn save_config_cmd(app: AppHandle, config: AppConfig) -> Result<(), String> {
    setup::apply_config(&app, config).await
}

#[tauri::command]
pub async fn enable_wireless_debug(app: AppHandle) -> Result<(), String> {
    setup::enable_wireless_debug(app).await
}

#[tauri::command]
pub fn start_keep_awake(app: AppHandle) -> Result<(), String> {
    keep_awake::start(&app)
}

#[tauri::command]
pub fn stop_keep_awake(app: AppHandle) {
    keep_awake::stop(&app)
}
