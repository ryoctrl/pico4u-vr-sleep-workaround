import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useTranslation } from 'react-i18next'
import { useTheme } from './useTheme'
import {
  AppConfig,
  ConnectionStatus,
  DEFAULT_CONFIG,
  INITIAL_STATUS,
  LogChannel,
  LogEntry,
} from '../types'

const MAX_LOGS = 500

const mergeLogs = (current: LogEntry[], incoming: LogEntry[]) => {
  const byId = new Map(current.map((e) => [e.id, e]))
  for (const e of incoming) byId.set(e.id, e)
  return [...byId.values()].sort((a, b) => a.id - b.id).slice(-MAX_LOGS)
}

export function useAppLogic() {
  const { t, i18n } = useTranslation()
  const { theme, setTheme } = useTheme()
  const [status, setStatus] = useState<ConnectionStatus>(INITIAL_STATUS)
  const [config, setConfig] = useState<AppConfig>(DEFAULT_CONFIG)
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [busy, setBusy] = useState(false)

  // Subscribe before fetching snapshots so nothing emitted in between is lost; ids dedupe overlaps.
  useEffect(() => {
    let disposed = false
    const unlisteners: Array<() => void> = []

    const init = async () => {
      try {
        unlisteners.push(
          await listen<ConnectionStatus>('connection-status', (e) => setStatus(e.payload)),
          await listen<LogEntry>('log', (e) => setLogs((prev) => mergeLogs(prev, [e.payload]))),
        )
        if (disposed) return
        const [s, l, c] = await Promise.all([
          invoke<ConnectionStatus>('get_status'),
          invoke<LogEntry[]>('get_logs'),
          invoke<AppConfig>('get_config'),
        ])
        if (disposed) return
        setStatus(s)
        setLogs((prev) => mergeLogs(prev, l))
        setConfig(c)
      } catch (e) {
        console.warn('Backend not available (ignore if running in a browser)', e)
      }
    }
    init()

    return () => {
      disposed = true
      unlisteners.forEach((u) => u())
    }
  }, [])

  const translateError = useCallback(
    (raw: unknown) => {
      const text = String(raw)
      const [code, detail] = text.split(/:(.*)/s)
      if (i18n.exists(`error.${code}`)) return t(`error.${code}`, { detail })
      return text
    },
    [t, i18n],
  )

  const saveConfig = useCallback(async (next: AppConfig) => {
    await invoke('save_config_cmd', { config: next })
    setConfig(await invoke<AppConfig>('get_config'))
  }, [])

  const enableWirelessDebug = useCallback(async () => {
    // Failures are reported through the setup log.
    await invoke('enable_wireless_debug').catch(() => {})
  }, [])

  const toggleKeepAwake = useCallback(async () => {
    setBusy(true)
    try {
      if (status.keep_awake.running) {
        await invoke('stop_keep_awake')
      } else {
        await invoke('start_keep_awake')
      }
    } catch (e) {
      console.warn('keep-awake toggle failed', e)
    } finally {
      setBusy(false)
    }
  }, [status.keep_awake.running])

  const clearLogs = useCallback(async (channel: LogChannel) => {
    await invoke('clear_logs', { channel }).catch(() => {})
    setLogs((prev) => prev.filter((e) => e.channel !== channel))
  }, [])

  return {
    t,
    i18n,
    theme,
    setTheme,
    status,
    config,
    logs,
    busy,
    saveConfig,
    enableWirelessDebug,
    toggleKeepAwake,
    clearLogs,
    translateError,
  }
}
