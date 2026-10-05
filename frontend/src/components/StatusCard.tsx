import { Button } from '@charcoal-ui/react'
import { useAppContext } from '../context/AppContext'
import { WirelessState } from '../types'

const DOT_CLASS: Record<WirelessState, string> = {
  checking: 'bg-gray-300 dark:bg-gray-600 animate-pulse',
  no_ip: 'bg-gray-300 dark:bg-gray-600',
  unreachable: 'bg-red-500',
  refused: 'bg-red-500',
  connecting: 'bg-amber-400 animate-pulse',
  unauthorized: 'bg-amber-400',
  offline: 'bg-amber-400',
  ready: 'bg-green-500',
}

export function StatusCard() {
  const { t, status, busy, toggleKeepAwake } = useAppContext()
  const { wireless, keep_awake: keepAwake } = status
  const adbDown = status.adb_server === 'unavailable'

  const title = adbDown ? t('adb_unavailable.title') : t(`wireless.${wireless.state}.title`)
  const hint = adbDown ? t('adb_unavailable.hint') : t(`wireless.${wireless.state}.hint`)
  const isReady = !adbDown && wireless.state === 'ready'
  const canStart = isReady && !busy

  const footer = (() => {
    if (keepAwake.running) {
      if (keepAwake.waiting) return t('keep_awake.waiting', { reason: title })
      return keepAwake.last_wake_at
        ? `${t('keep_awake.running')} · ${t('keep_awake.last_wake', { time: keepAwake.last_wake_at })}`
        : t('keep_awake.running')
    }
    // When starting is not possible, the reason is shown right under the button.
    return isReady ? t('keep_awake.stopped') : hint
  })()

  return (
    <section className='mx-5 mb-3 rounded-xl border border-gray-200 bg-white p-4 shadow-sm dark:border-gray-700 dark:bg-gray-800'>
      <div className='flex items-center gap-2.5'>
        <span
          className={`h-3 w-3 shrink-0 rounded-full ${adbDown ? 'bg-red-500' : DOT_CLASS[wireless.state]}`}
        />
        <h2 className='text-base font-bold text-gray-900 dark:text-gray-100'>{title}</h2>
      </div>
      <div className='mt-1 flex items-center justify-between gap-2 pl-5.5 text-[11px] text-gray-500 dark:text-gray-400'>
        <span className='truncate font-mono'>{wireless.target ?? '—'}</span>
        {wireless.checked_at && (
          <span className='shrink-0'>
            {t('keep_awake.checked_at', { time: wireless.checked_at })}
          </span>
        )}
      </div>

      <div className='mt-4'>
        {keepAwake.running ? (
          <Button onClick={toggleKeepAwake} variant='Danger' fullWidth disabled={busy}>
            {t('keep_awake.stop')}
          </Button>
        ) : (
          <Button onClick={toggleKeepAwake} variant='Primary' fullWidth disabled={!canStart}>
            {t('keep_awake.start')}
          </Button>
        )}
      </div>

      <p
        className={`mt-2 text-xs leading-relaxed ${
          keepAwake.running && !keepAwake.waiting
            ? 'font-semibold text-brand'
            : keepAwake.waiting
              ? 'font-semibold text-amber-600 dark:text-amber-400'
              : 'text-gray-500 dark:text-gray-400'
        }`}
      >
        {footer}
      </p>
    </section>
  )
}
