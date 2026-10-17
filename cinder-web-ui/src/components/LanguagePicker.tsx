import { useState } from 'react'
import Modal from './Modal'
import { useI18n } from '../i18n'
import { LOCALE_LABELS } from '../i18n/strings'

/**
 * Account-level display-language picker. Offers the union of locales shipped by
 * any pack (from the account profile) and persists the choice to the account.
 */
export default function LanguagePicker() {
  const { locale, setLocale, localeOptions, t } = useI18n()
  const [open, setOpen] = useState(false)
  const [busy, setBusy] = useState(false)

  const currentLabel = LOCALE_LABELS[locale] ?? locale
  const options =
    localeOptions.length > 0 ? localeOptions : [{ code: locale, label: currentLabel }]

  async function choose(code: string) {
    if (code === locale || busy) {
      setOpen(false)
      return
    }
    setBusy(true)
    try {
      await setLocale(code)
    } finally {
      setBusy(false)
      setOpen(false)
    }
  }

  return (
    <>
      <button
        type="button"
        onClick={() => setOpen(true)}
        className="text-xs font-mono uppercase tracking-widest text-muted hover:text-text transition-colors cursor-pointer"
        title={t('games.languagePickerTitle')}
      >
        {t('games.language')}: {currentLabel}
      </button>

      {open && (
        <Modal title={t('games.languagePickerTitle')} onClose={() => setOpen(false)}>
          <div className="divide-y divide-subtle/40 border-t border-b border-subtle/40">
            {options.map(option => (
              <button
                key={option.code}
                type="button"
                onClick={() => void choose(option.code)}
                disabled={busy && option.code !== locale}
                className={`w-full py-3 px-1 flex items-center justify-between text-left transition-colors cursor-pointer ${
                  option.code === locale
                    ? 'text-foam font-semibold'
                    : 'text-text hover:text-foam'
                }`}
              >
                <span className="text-sm">{option.label}</span>
                {option.code === locale && (
                  <span className="text-xs font-mono uppercase tracking-wider text-foam bg-pine/15 px-2 py-0.5 rounded border border-pine/30">
                    {t('common.active')}
                  </span>
                )}
              </button>
            ))}
          </div>
        </Modal>
      )}
    </>
  )
}
