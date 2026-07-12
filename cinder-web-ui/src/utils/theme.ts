import type { ThemeDefinition } from '../api'

export function themeVars(theme: ThemeDefinition): React.CSSProperties {
  return {
    '--color-base': theme.base,
    '--color-surface': theme.surface,
    '--color-overlay': theme.overlay,
    '--color-muted': theme.muted,
    '--color-text': theme.text,
    '--color-love': theme.love,
    '--color-gold': theme.gold,
    '--color-rose': theme.rose,
    '--color-pine': theme.pine,
    '--color-foam': theme.foam,
    '--color-iris': theme.iris,
    '--color-highlight-high': theme.highlight_high,
  } as React.CSSProperties
}
