// Mirrors the Rust types in src/state.rs, src/logs.rs and src/config.rs.

export type WirelessState =
  | 'checking'
  | 'no_ip'
  | 'unreachable'
  | 'refused'
  | 'connecting'
  | 'unauthorized'
  | 'offline'
  | 'ready'

export type UsbState = 'none' | 'unauthorized' | 'offline' | 'device' | 'multiple'

export interface ConnectionStatus {
  adb_server: 'ok' | 'unavailable'
  wireless: { state: WirelessState; target: string | null; checked_at: string | null }
  usb: { state: UsbState; serial: string | null; model: string | null }
  keep_awake: { running: boolean; waiting: boolean; last_wake_at: string | null }
  setup_running: boolean
}

export type LogChannel = 'app' | 'setup'

export interface LogEntry {
  id: number
  ts: string
  channel: LogChannel
  level: 'info' | 'warn' | 'error' | 'debug'
  key: string
  params: Record<string, unknown>
}

export interface AppConfig {
  dim_delay_hours: number
  ip_address: string
  keep_awake_interval_secs: number
  debug_mode: boolean
}

export const INITIAL_STATUS: ConnectionStatus = {
  adb_server: 'ok',
  wireless: { state: 'checking', target: null, checked_at: null },
  usb: { state: 'none', serial: null, model: null },
  keep_awake: { running: false, waiting: false, last_wake_at: null },
  setup_running: false,
}

export const DEFAULT_CONFIG: AppConfig = {
  dim_delay_hours: 1,
  ip_address: '',
  keep_awake_interval_secs: 3,
  debug_mode: false,
}

export const isValidIpv4 = (value: string) => {
  const parts = value.trim().split('.')
  return parts.length === 4 && parts.every((p) => /^\d{1,3}$/.test(p) && Number(p) <= 255)
}
