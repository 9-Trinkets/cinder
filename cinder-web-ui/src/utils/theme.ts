import type { ThemeDefinition } from '../api'

export function hexToHsl(hex: string): string {
  const raw = hex.replace('#', '')
  const r = parseInt(raw.substring(0, 2), 16) / 255
  const g = parseInt(raw.substring(2, 4), 16) / 255
  const b = parseInt(raw.substring(4, 6), 16) / 255

  const max = Math.max(r, g, b)
  const min = Math.min(r, g, b)
  const l = (max + min) / 2
  let h = 0
  let s = 0

  if (max !== min) {
    const d = max - min
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min)
    switch (max) {
      case r: h = ((g - b) / d + (g < b ? 6 : 0)) / 6; break
      case g: h = ((b - r) / d + 2) / 6; break
      case b: h = ((r - g) / d + 4) / 6; break
    }
  }

  return `${Math.round(h * 360)} ${Math.round(s * 100)}% ${Math.round(l * 100)}%`
}

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
    ['--color-crt-glow', theme.crt_glow],
    ['--color-crt-dim', theme.crt_dim],
    ['--color-crt-bez', theme.crt_bez],
  ]
  for (const [key, hex] of vars) {
    root.style.setProperty(key, hexToHsl(hex))
  }
}
