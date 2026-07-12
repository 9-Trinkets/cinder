import type { ThemeDefinition } from '../api'

export function applyTheme(theme: ThemeDefinition) {
  const root = document.documentElement
  const vars: [string, string][] = [
    ['--color-base', theme.base],
    ['--color-surface', theme.surface],
    ['--color-overlay', theme.overlay],
    ['--color-muted', theme.muted],
    ['--color-text', theme.text],
    ['--color-love', theme.love],
    ['--color-gold', theme.gold],
    ['--color-rose', theme.rose],
    ['--color-pine', theme.pine],
    ['--color-foam', theme.foam],
    ['--color-iris', theme.iris],
    ['--color-highlight-high', theme.highlight_high],
  ]
  for (const [key, value] of vars) {
    root.style.setProperty(key, value)
  }
}
