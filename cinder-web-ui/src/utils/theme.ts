import type { ThemeDefinition } from '../api'

export function themeVars(theme: ThemeDefinition): React.CSSProperties {
  return {
    '--color-base': theme.base,
    '--color-canvas': theme.base,
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
    '--color-crafted-highlight': theme.crafted_highlight,
    '--color-crt-glow': theme.crt_glow,
    '--font-prose': theme.font_prose || "ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
    '--font-mono': theme.font_mono || "ui-monospace, 'SFMono-Regular', 'SF Mono', Menlo, Consolas, monospace",
  } as React.CSSProperties
}
