import { useEffect, useRef, useState } from 'react'
import { Button, Icon, TextField } from '@charcoal-ui/react'
import { useAppContext } from '../context/AppContext'
import { UsbState, isValidIpv4 } from '../types'

const USB_DOT: Record<UsbState, string> = {
  none: 'bg-gray-300 dark:bg-gray-600',
  unauthorized: 'bg-amber-400',
  offline: 'bg-amber-400',
  device: 'bg-green-500',
  multiple: 'bg-red-500',
}

interface Props {
  open: boolean
  onToggle: () => void
}

export function WirelessSetupPanel({ open, onToggle }: Props) {
  const { t, status, config, logs, saveConfig, enableWirelessDebug } = useAppContext()
  const { usb, wireless, setup_running: setupRunning } = status

  const usbLabel = (() => {
    if (usb.state === 'device') {
      return usb.model ? t('usb.device', { model: usb.model }) : t('usb.device_unknown')
    }
    return t(`usb.${usb.state}`)
  })()
  const usbNote = (() => {
    if (usb.state === 'unauthorized') return t('usb.unauthorized_hint')
    if (usb.state === 'device' && usb.model && !usb.model.includes('A9210'))
      return t('usb.not_pico')
    return null
  })()

  // "Unplug USB" stays visible after a successful setup until the cable is actually removed.
  const [showDone, setShowDone] = useState(false)
  const prevSetupRunning = useRef(setupRunning)
  useEffect(() => {
    if (prevSetupRunning.current && !setupRunning) {
      const last = [...logs]
        .reverse()
        .find((e) => e.channel === 'setup' && e.key.startsWith('setup_'))
      setShowDone(last?.key === 'setup_done')
    }
    prevSetupRunning.current = setupRunning
  }, [setupRunning, logs])
  useEffect(() => {
    if (usb.state === 'none') setShowDone(false)
  }, [usb.state])

  const [ipDraft, setIpDraft] = useState(config.ip_address)
  useEffect(() => setIpDraft(config.ip_address), [config.ip_address])
  const ipValid = isValidIpv4(ipDraft)
  const ipChanged = ipDraft.trim() !== config.ip_address

  const canEnable = usb.state === 'device' && !setupRunning
  const emphasizeEnable = usb.state === 'device' && wireless.state !== 'ready'

  return (
    <section className='rounded-xl border border-gray-200 bg-white shadow-sm dark:border-gray-700 dark:bg-gray-800'>
      <button
        onClick={onToggle}
        aria-expanded={open}
        className='flex w-full items-center justify-between rounded-xl px-4 py-3 text-sm font-bold text-gray-800 transition-colors hover:bg-gray-50 dark:text-gray-200 dark:hover:bg-gray-700/50'
      >
        <span>{t('setup.header')}</span>
        <span className='flex items-center gap-2 text-xs font-normal text-gray-500 dark:text-gray-400'>
          <span className={`h-2 w-2 rounded-full ${USB_DOT[usb.state]}`} />
          {t('setup.usb_label')}
          <Icon name={open ? '16/Up' : '16/Down'} />
        </span>
      </button>

      {open && (
        <div className='flex flex-col gap-4 border-t border-gray-100 px-4 pt-3 pb-4 dark:border-gray-700'>
          {wireless.state === 'no_ip' && (
            <div className='rounded-lg bg-gray-50 p-3 text-xs text-gray-700 dark:bg-gray-900/60 dark:text-gray-300'>
              <p className='mb-1.5 font-bold'>{t('setup.steps_title')}</p>
              <ol className='list-decimal space-y-1 pl-4 leading-relaxed'>
                <li>{t('setup.step1')}</li>
                <li>{t('setup.step2')}</li>
                <li>{t('setup.step3')}</li>
              </ol>
            </div>
          )}

          <div>
            <div className='flex items-center gap-2 text-sm text-gray-800 dark:text-gray-200'>
              <span className={`h-2.5 w-2.5 shrink-0 rounded-full ${USB_DOT[usb.state]}`} />
              <span className='font-semibold'>{t('setup.usb_label')}:</span>
              <span className='truncate'>{usbLabel}</span>
            </div>
            {usbNote && (
              <p className='mt-1 pl-4.5 text-xs leading-relaxed text-amber-700 dark:text-amber-400'>
                {usbNote}
              </p>
            )}
          </div>

          <div className='flex flex-col gap-1.5'>
            <Button
              onClick={enableWirelessDebug}
              variant={emphasizeEnable ? 'Primary' : 'Default'}
              fullWidth
              disabled={!canEnable}
            >
              {setupRunning ? t('setup.enabling') : t('setup.enable')}
            </Button>
            {!setupRunning && usb.state !== 'device' && (
              <p className='text-xs text-gray-500 dark:text-gray-400'>
                {t('setup.enable_needs_usb')}
              </p>
            )}
            {showDone && (
              <p className='rounded-lg border border-green-200 bg-green-50 px-3 py-2 text-xs font-semibold text-green-700 dark:border-green-800 dark:bg-green-950 dark:text-green-300'>
                {t('setup.done_unplug')}
              </p>
            )}
          </div>

          <div className='flex items-start gap-2'>
            <div className='min-w-0 flex-1'>
              <TextField
                label={t('setup.ip_label')}
                showLabel
                value={ipDraft}
                onChange={setIpDraft}
                placeholder={t('setup.ip_placeholder')}
                invalid={ipDraft !== '' && !ipValid}
                assistiveText={ipDraft !== '' && !ipValid ? t('setup.ip_invalid') : undefined}
              />
            </div>
            <div className='pt-[26px]'>
              <Button
                size='M'
                disabled={!ipValid || !ipChanged || setupRunning}
                onClick={() => saveConfig({ ...config, ip_address: ipDraft.trim() })}
              >
                {t('setup.ip_save')}
              </Button>
            </div>
          </div>
        </div>
      )}
    </section>
  )
}
