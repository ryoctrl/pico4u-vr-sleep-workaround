import { useAppContext } from '../context/AppContext'
import { LogChannel, LogEntry } from '../types'

const LEVEL_CLASS: Record<LogEntry['level'], string> = {
  warn: 'text-amber-300',
  error: 'text-red-400',
  debug: 'text-gray-500',
}

interface Props {
  channel: LogChannel
  onChannelChange: (channel: LogChannel) => void
}

export function LogView({ channel, onChannelChange }: Props) {
  const { t, logs, clearLogs, translateError } = useAppContext()

  const format = (entry: LogEntry) => {
    const params: Record<string, unknown> = { ...entry.params }
    if (typeof params.state === 'string') {
      params.state =
        entry.key === 'usb_state'
          ? params.state === 'device' && params.model
            ? t('usb.device', { model: params.model })
            : t(`usb.${params.state}`)
          : t(`wireless.${params.state}.title`)
    }
    if (params.error !== undefined) params.error = translateError(params.error)
    return t(`log.${entry.key}`, params)
  }

  const visible = logs.filter((e) => e.channel === channel).reverse()
  const tabs: { key: LogChannel; label: string }[] = [
    { key: 'app', label: t('logs.tab_app') },
    { key: 'setup', label: t('logs.tab_setup') },
  ]

  return (
    <section className='flex min-h-[220px] flex-1 flex-col gap-2'>
      <div className='flex items-center justify-between'>
        <div className='flex gap-1 rounded-lg bg-gray-100 p-1 dark:bg-gray-800'>
          {tabs.map((tab) => (
            <button
              key={tab.key}
              onClick={() => onChannelChange(tab.key)}
              className={`rounded-md px-3 py-1 text-xs font-semibold transition-all ${
                channel === tab.key
                  ? 'bg-white text-gray-900 shadow-sm dark:bg-gray-700 dark:text-gray-100'
                  : 'text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>
        <button
          onClick={() => clearLogs(channel)}
          className='rounded-md px-2 py-1 text-xs text-gray-500 transition-colors hover:bg-gray-100 hover:text-gray-700 dark:text-gray-400 dark:hover:bg-gray-800 dark:hover:text-gray-200'
        >
          {t('logs.clear')}
        </button>
      </div>
      <div className='min-h-0 flex-1 overflow-y-auto rounded-lg border border-gray-800 bg-gray-900 p-3 font-mono text-[11px] leading-relaxed dark:bg-black'>
        {visible.map((entry) => (
          <div key={entry.id} className={`mb-1 break-all ${LEVEL_CLASS[entry.level]}`}>
            <span className='text-gray-500'>[{entry.ts}]</span> {format(entry)}
          </div>
        ))}
        {visible.length === 0 && <div className='text-gray-500 italic'>{t('logs.empty')}</div>}
      </div>
    </section>
  )
}
