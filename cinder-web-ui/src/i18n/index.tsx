import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from 'react'
import { useLocation } from 'react-router-dom'
import { useAuth } from '../auth'
import * as api from '../api'
import type { LocaleItem } from '../api'
import { LOCALES, resolveLocale, type MessageKey } from './strings'

const STORAGE_KEY = 'uiLocale'

interface I18nValue {
  /** The resolved UI locale code (never unsupported; falls back to `en`). */
  locale: string
  /** The account's chosen locale, or null when no preference is set. */
  preferredLocale: string | null
  /** Locales the account picker should offer (union across packs). */
  localeOptions: LocaleItem[]
  /** True once the account profile (and its preference) has loaded. */
  ready: boolean
  /** Change the display language, persisting it to the account when signed in. */
  setLocale: (locale: string | null) => Promise<void>
  reload: () => Promise<void>
  t: (key: MessageKey, vars?: Record<string, string | number>) => string
}

const I18nContext = createContext<I18nValue | null>(null)

function initialLocale(): string {
  const stored = localStorage.getItem(STORAGE_KEY)
  if (stored) return resolveLocale(stored)
  if (typeof navigator !== 'undefined' && navigator.language?.toLowerCase().startsWith('zh')) {
    return 'zh-TW'
  }
  return resolveLocale(null)
}

function interpolate(template: string, vars?: Record<string, string | number>): string {
  if (!vars) return template
  return template.replace(/\{(\w+)\}/g, (match, key: string) =>
    key in vars ? String(vars[key]) : match,
  )
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const { token } = useAuth()
  const { pathname } = useLocation()
  const [locale, setLocaleState] = useState<string>(initialLocale)
  const [preferredLocale, setPreferredLocale] = useState<string | null>(null)
  const [localeOptions, setLocaleOptions] = useState<LocaleItem[]>([])
  const [ready, setReady] = useState(false)

  const applyLocale = useCallback((next: string | null) => {
    const resolved = resolveLocale(next)
    setLocaleState(resolved)
    localStorage.setItem(STORAGE_KEY, resolved)
  }, [])

  const reload = useCallback(async () => {
    if (!token) {
      setReady(true)
      return
    }
    try {
      const profile = await api.getMe(token)
      setPreferredLocale(profile.preferred_locale)
      setLocaleOptions(profile.locale_options)
      if (profile.preferred_locale) {
        applyLocale(profile.preferred_locale)
      }
    } catch {
      // Non-fatal: keep the locally detected locale.
    } finally {
      setReady(true)
    }
  }, [token, applyLocale])

  // Re-sync on navigation so an in-game language change (which also updates the
  // account preference) is reflected in the library chrome.
  useEffect(() => {
    void reload()
  }, [reload, pathname])

  const setLocale = useCallback(
    async (next: string | null) => {
      applyLocale(next)
      setPreferredLocale(next)
      if (!token) return
      try {
        const profile = await api.updateMe(token, next)
        setPreferredLocale(profile.preferred_locale)
        setLocaleOptions(profile.locale_options)
        applyLocale(profile.preferred_locale ?? next)
      } catch {
        // Keep the optimistic local change; the server will re-sync on reload.
      }
    },
    [token, applyLocale],
  )

  const t = useCallback(
    (key: MessageKey, vars?: Record<string, string | number>) =>
      interpolate((LOCALES[locale] ?? LOCALES.en)[key], vars),
    [locale],
  )

  const value = useMemo<I18nValue>(
    () => ({ locale, preferredLocale, localeOptions, ready, setLocale, reload, t }),
    [locale, preferredLocale, localeOptions, ready, setLocale, reload, t],
  )

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>
}

export function useI18n(): I18nValue {
  const ctx = useContext(I18nContext)
  if (!ctx) throw new Error('useI18n must be used within I18nProvider')
  return ctx
}

export { LOCALE_LABELS } from './strings'
