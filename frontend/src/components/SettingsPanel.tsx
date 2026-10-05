import { Button, TextField, DropdownSelector, DropdownMenuItem, Checkbox } from '@charcoal-ui/react'
import { useState } from 'react'
import { useAppContext } from '../context/AppContext'

type Theme = 'light' | 'dark' | 'system'

export function SettingsPanel({ onClose }: { onClose: () => void }) {
  const { t, i18n, theme, setTheme, config, saveConfig, status } = useAppContext()

  const [language, setLanguage] = useState(i18n.resolvedLanguage ?? i18n.language)
  const [pendingTheme, setPendingTheme] = useState<Theme>(theme)
  const [dim, setDim] = useState(String(config.dim_delay_hours))
  const [interval, setIntervalValue] = useState(String(config.keep_awake_interval_secs))
  const [debug, setDebug] = useState(config.debug_mode)
  const [saving, setSaving] = useState(false)

  const dimNum = Number(dim)
  const intervalNum = Number(interval)
  const dimValid = dim.trim() !== '' && Number.isFinite(dimNum) && dimNum >= 0
  const intervalValid = Number.isInteger(intervalNum) && intervalNum >= 1

  const handleApply = async () => {
    if (!dimValid || !intervalValid) return
    setSaving(true)
    try {
      if (language !== i18n.language) await i18n.changeLanguage(language)
      if (pendingTheme !== theme) setTheme(pendingTheme)
      await saveConfig({
        ...config,
        dim_delay_hours: dimNum,
        keep_awake_interval_secs: intervalNum,
        debug_mode: debug,
      })
      onClose()
    } finally {
      setSaving(false)
    }
  }

  return (
    <div className='absolute inset-0 z-10 flex flex-col bg-gray-50 animate-fadeIn dark:bg-gray-900'>
      <header className='shrink-0 px-5 pt-4 pb-2'>
        <h2 className='text-lg font-bold text-gray-900 dark:text-gray-100'>
          {t('settings.title')}
        </h2>
      </header>

      <div className='custom-scrollbar flex-1 overflow-y-auto px-5'>
        <div className='flex flex-col gap-6 pt-2 pb-6'>
          {status.keep_awake.running && (
            <p className='rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-800 dark:bg-amber-950 dark:text-amber-300'>
              {t('settings.running_note')}
            </p>
          )}

          <DropdownSelector
            label={t('settings.language')}
            value={language}
            onChange={setLanguage}
            className='w-full'
            showLabel
          >
            <DropdownMenuItem value='ja'>日本語</DropdownMenuItem>
            <DropdownMenuItem value='en'>English</DropdownMenuItem>
          </DropdownSelector>

          <DropdownSelector
            label={t('settings.theme')}
            value={pendingTheme}
            onChange={(val) => setPendingTheme(val as Theme)}
            className='w-full'
            showLabel
          >
            <DropdownMenuItem value='light'>{t('settings.theme_light')}</DropdownMenuItem>
            <DropdownMenuItem value='dark'>{t('settings.theme_dark')}</DropdownMenuItem>
            <DropdownMenuItem value='system'>{t('settings.theme_system')}</DropdownMenuItem>
          </DropdownSelector>

          <TextField
            label={t('settings.dim')}
            type='number'
            min={0}
            step={0.5}
            value={dim}
            onChange={setDim}
            invalid={!dimValid}
            assistiveText={t('settings.dim_note')}
            className='w-full text-left'
            showLabel
          />

          <TextField
            label={t('settings.interval')}
            type='number'
            min={1}
            value={interval}
            onChange={setIntervalValue}
            invalid={!intervalValid}
            assistiveText={t('settings.interval_note')}
            className='w-full text-left'
            showLabel
          />

          <div className='text-sm font-bold text-gray-900 dark:text-gray-100'>
            <Checkbox checked={debug} onChange={setDebug}>
              {t('settings.debug')}
            </Checkbox>
          </div>
        </div>
      </div>

      <div className='flex shrink-0 flex-col gap-3 border-t border-gray-200 bg-gray-50 px-5 pt-4 pb-6 dark:border-gray-800 dark:bg-gray-900'>
        <Button
          onClick={handleApply}
          variant='Primary'
          fullWidth
          disabled={saving || !dimValid || !intervalValid}
        >
          {t('settings.apply')}
        </Button>
        <Button onClick={onClose} variant='Default' fullWidth>
          {t('settings.cancel')}
        </Button>
      </div>
    </div>
  )
}
