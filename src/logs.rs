use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;

const MAX_ENTRIES: usize = 500;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    App,
    Setup,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Info,
    Warn,
    Error,
    Debug,
}

/// `key` is a translation key resolved by the frontend (`log.<key>`), `params` its interpolation values.
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub id: u64,
    pub ts: String,
    pub channel: Channel,
    pub level: Level,
    pub key: String,
    pub params: Value,
}

#[derive(Default)]
pub struct LogBuffer {
    inner: Mutex<(u64, VecDeque<LogEntry>)>,
}

impl LogBuffer {
    fn push(&self, channel: Channel, level: Level, key: &str, params: Value) -> LogEntry {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.0 += 1;
        let entry = LogEntry {
            id: guard.0,
            ts: chrono::Local::now().format("%H:%M:%S").to_string(),
            channel,
            level,
            key: key.to_string(),
            params,
        };
        if guard.1.len() >= MAX_ENTRIES {
            guard.1.pop_front();
        }
        guard.1.push_back(entry.clone());
        entry
    }

    pub fn snapshot(&self) -> Vec<LogEntry> {
        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.1.iter().cloned().collect()
    }

    pub fn clear(&self, channel: Channel) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.1.retain(|e| e.channel != channel);
    }
}

/// Records a log entry and pushes it to the UI. Debug entries are dropped unless debug mode is on.
pub fn log(app: &AppHandle, channel: Channel, level: Level, key: &str, params: Value) {
    let state = app.state::<AppState>();
    if level == Level::Debug && !state.debug_mode.load(Ordering::Relaxed) {
        return;
    }
    let entry = state.logs.push(channel, level, key, params);
    let _ = app.emit("log", entry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn buffer_keeps_latest_entries_with_increasing_ids() {
        let buf = LogBuffer::default();
        for i in 0..(MAX_ENTRIES + 5) {
            buf.push(Channel::App, Level::Info, "k", json!({ "i": i }));
        }
        let snap = buf.snapshot();
        assert_eq!(snap.len(), MAX_ENTRIES);
        assert_eq!(snap.first().unwrap().id, 6);
        assert_eq!(snap.last().unwrap().id, (MAX_ENTRIES + 5) as u64);
    }

    #[test]
    fn clear_only_removes_one_channel() {
        let buf = LogBuffer::default();
        buf.push(Channel::App, Level::Info, "a", Value::Null);
        buf.push(Channel::Setup, Level::Info, "s", Value::Null);
        buf.clear(Channel::Setup);
        let snap = buf.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].channel, Channel::App);
    }
}
