import { useEffect, useRef, useState } from 'react'
import { IconButton } from '@charcoal-ui/react'
import { useAppContext } from '../context/AppContext'
import packageJson from '../../package.json'
import { StatusCard } from './StatusCard'
import { WirelessSetupPanel } from './WirelessSetupPanel'
import { LogView } from './LogView'
import { SettingsPanel } from './SettingsPanel'
import { LogChannel, WirelessState } from '../types'

const NEEDS_SETUP: WirelessState[] = ['no_ip', 'refused', 'unreachable']

export function MainView() {
  const { t, status } = useAppContext()
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [setupOpen, setSetupOpen] = useState(false)
  const [logChannel, setLogChannel] = useState<LogChannel>('app')

  // Accordion rules: decide once after the first check, re-open when a ready headset is lost,
  // never close automatically (closing is always the user's action).
  const prevWireless = useRef<WirelessState>(status.wireless.state)
  useEffect(() => {
    const prev = prevWireless.current
    const next = status.wireless.state
    if (prev === 'checking' && next !== 'checking' && next !== 'ready') setSetupOpen(true)
    if (prev === 'ready' && NEEDS_SETUP.includes(next)) setSetupOpen(true)
    prevWireless.current = next
  }, [status.wireless.state])

  useEffect(() => {
    if (status.setup_running) {
      setSetupOpen(true)
      setLogChannel('setup')
    }
  }, [status.setup_running])

  return (
    <div className='relative flex h-screen flex-col bg-gray-50 text-gray-900 dark:bg-gray-900 dark:text-gray-100'>
      <header className='flex shrink-0 items-center justify-between px-5 pt-3 pb-3'>
        <div className='flex items-baseline gap-2'>
          <h1 className='text-lg leading-normal font-bold'>{t('title')}</h1>
          <span className='font-mono text-xs font-medium text-gray-400 dark:text-gray-500'>
            v{packageJson.version}
            {import.meta.env.DEV ? '-dev' : ''}
          </span>
        </div>
        <IconButton
          icon='24/Settings'
          size='S'
          aria-label={t('open_settings')}
          title={t('open_settings')}
          onClick={() => setSettingsOpen(true)}
        />
      </header>

      <StatusCard />

      <div className='custom-scrollbar flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto px-5 pb-5'>
        <WirelessSetupPanel open={setupOpen} onToggle={() => setSetupOpen((v) => !v)} />
        <LogView channel={logChannel} onChannelChange={setLogChannel} />
      </div>

      {/* Settings overlay keeps the main view mounted so nothing reconnects on close. */}
      {settingsOpen && <SettingsPanel onClose={() => setSettingsOpen(false)} />}
    </div>
  )
}
