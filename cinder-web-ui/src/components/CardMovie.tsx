import { useEffect, useMemo, useRef, useState } from 'react'
import * as api from '../api'
import { useI18n } from '../i18n'

const THEME_COLORS = new Set([
  'base', 'surface', 'overlay', 'muted', 'text', 'love', 'gold', 'rose', 'pine', 'foam', 'iris',
])

/** A theme color name becomes its CSS variable, anything else is taken as a CSS color. */
function color(value: string | undefined, fallback: string): string {
  const v = (value ?? '').trim()
  if (!v) return fallback
  return THEME_COLORS.has(v) ? `var(--color-${v})` : v
}

function fontFamily(font: string): string {
  if (font === 'hand') return 'var(--font-hand)'
  if (font === 'mono') return 'var(--font-mono)'
  return 'var(--font-prose)'
}

const reducedMotion = () =>
  typeof window !== 'undefined' && window.matchMedia?.('(prefers-reduced-motion: reduce)').matches

/**
 * Shows `text` a letter at a time once `ready`, or all at once when motion is
 * reduced. New text starts again from nothing, with no frame showing the old count.
 */
function useTyped(text: string, on: boolean, msPerChar: number, ready = true): string {
  const [typed, setTyped] = useState({ text, count: on ? 0 : text.length })
  const count = typed.text === text ? typed.count : on ? 0 : text.length
  useEffect(() => {
    if (!on) return
    setTyped({ text, count: 0 })
    if (!ready) return
    const timer = setInterval(() => {
      setTyped(t => {
        if (t.count >= text.length) {
          clearInterval(timer)
          return t
        }
        return { text, count: t.count + 1 }
      })
    }, msPerChar)
    return () => clearInterval(timer)
  }, [text, on, msPerChar, ready])
  return text.slice(0, count)
}

const HEADING_MS = 70

/** A short line is typed slowly enough to be seen being typed. */
function textMs(text: string): number {
  return text.length < 24 ? 90 : 35
}

/**
 * Typed text sits over an invisible copy of the whole text, so the block keeps
 * its final size and nothing shifts while the letters arrive.
 */
function Typed({ full, typed, className }: { full: string; typed: string; className: string }) {
  return (
    <div className={`relative w-full ${className}`}>
      <div className="invisible" aria-hidden>{full}</div>
      <div className="absolute inset-0" aria-label={full}>{typed}</div>
    </div>
  )
}

const PARTICLES = 28

/** A slow layer behind the text. Positions are fixed so it does not jump on re-render. */
function Ambient({ kind }: { kind: string }) {
  const items = useMemo(
    () =>
      Array.from({ length: PARTICLES }, (_, i) => ({
        left: (i * 37 + 11) % 100,
        delay: -((i * 53) % 100) / 10,
        duration: 9 + ((i * 29) % 80) / 10,
        size: 2 + ((i * 17) % 4),
        drift: ((i * 13) % 40) - 20,
      })),
    [],
  )
  if (!kind || kind === 'none' || reducedMotion()) return null
  const style = (p: (typeof items)[number]): React.CSSProperties => ({
    left: `${p.left}%`,
    animationDelay: `${p.delay}s`,
    animationDuration: `${kind === 'rain' ? p.duration / 5 : p.duration}s`,
    ['--drift' as string]: `${p.drift}px`,
  })
  return (
    <div className="pointer-events-none absolute inset-0 overflow-hidden" aria-hidden>
      <style>{`
        @keyframes card-fall { from { transform: translate(0,-6vh) rotate(0deg); opacity: 0 } 10% { opacity: .8 } to { transform: translate(var(--drift),106vh) rotate(180deg); opacity: .6 } }
        @keyframes card-rise { from { transform: translate(0,6vh); opacity: 0 } 20% { opacity: .5 } to { transform: translate(var(--drift),-30vh); opacity: 0 } }
        @keyframes card-fade { from { opacity: 0 } to { opacity: 1 } }
      `}</style>
      {items.map((p, i) => {
        if (kind === 'rain') {
          return (
            <span key={i} className="absolute top-0 w-px bg-text/40" style={{ ...style(p), height: 16 + p.size * 4, animationName: 'card-fall', animationIterationCount: 'infinite', animationTimingFunction: 'linear' }} />
          )
        }
        if (kind === 'leaves') {
          return (
            <span key={i} className="absolute top-0 rounded-[60%_0_60%_0] bg-gold/60" style={{ ...style(p), width: 6 + p.size * 2, height: 4 + p.size, animationName: 'card-fall', animationIterationCount: 'infinite', animationTimingFunction: 'ease-in-out' }} />
          )
        }
        if (kind === 'drift') {
          return (
            <span key={i} className="absolute bottom-0 rounded-full bg-text/30" style={{ ...style(p), width: p.size, height: p.size, animationName: 'card-rise', animationIterationCount: 'infinite', animationTimingFunction: 'ease-out' }} />
          )
        }
        return (
          <span key={i} className="absolute top-0 rounded-full bg-text/70" style={{ ...style(p), width: p.size, height: p.size, animationName: 'card-fall', animationIterationCount: 'infinite', animationTimingFunction: 'ease-in-out' }} />
        )
      })}
    </div>
  )
}

/** A full-screen card for a passing of time, a title or a page. Each frame fades into the next. */
export default function CardMovie({ movie, frame, onAdvance, onClose, contained = false }: {
  movie: api.MovieData
  frame: number
  onAdvance: () => void
  onClose: () => void
  /** Cover only the area it is placed in, not the whole screen. */
  contained?: boolean
}) {
  const { t } = useI18n()
  const card = movie.card
  const f = movie.frames[frame]
  const isLast = frame >= movie.frames.length - 1
  const motion = !reducedMotion()
  const typing = motion && card.reveal === 'type'
  const fadeMs = motion ? card.fade_ms || 900 : 0
  // The heading is typed first, and the text begins when it is done.
  const heading = useTyped(f?.heading ?? '', typing, HEADING_MS)
  const headingDone = heading.length >= (f?.heading?.length ?? 0)
  const text = useTyped(f?.text ?? '', typing, textMs(f?.text ?? ''), headingDone)
  const dialogRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    dialogRef.current?.focus()
  }, [])

  // Hold each frame long enough to be read, and to be typed when it is typed.
  const holdMs = useMemo(() => {
    const base = Math.max(1200, f?.duration_ms ?? 0)
    if (!typing || !f) return base
    return Math.max(base, (f.heading?.length ?? 0) * HEADING_MS + (f.text?.length ?? 0) * textMs(f.text ?? '') + 1200)
  }, [f, typing])

  const waits = !!f?.wait_for_click && !isLast
  const textDone = text.length >= (f?.text?.length ?? 0) && headingDone

  useEffect(() => {
    if (!f || isLast || waits) return
    const timer = setTimeout(onAdvance, holdMs)
    return () => clearTimeout(timer)
  }, [frame, f, isLast, waits, onAdvance, holdMs])

  // A frame that waits moves on to the next frame, any other click skips the card.
  const proceed = waits ? onAdvance : onClose

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose()
      else if (event.key === 'Enter' || event.key === ' ') proceed()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onClose, proceed])

  if (!f) return null

  const background = color(f.tint || card.background, 'var(--color-base)')
  const align = card.align === 'left' ? 'text-left items-start' : 'text-center items-center'

  return (
    <div
      ref={dialogRef}
      role="dialog"
      aria-modal={contained ? undefined : true}
      aria-label={movie.title}
      tabIndex={-1}
      onClick={proceed}
      className={`${contained ? 'absolute inset-0 z-20' : 'fixed inset-0 z-50'} flex cursor-pointer items-center justify-center outline-none`}
      style={{ background, transition: `background-color ${fadeMs}ms ease` }}
    >
      <Ambient kind={f.ambient || card.ambient} />
      <div
        key={frame}
        className={`relative mx-6 flex w-full max-w-xl flex-col gap-4 ${align}`}
        style={{
          color: color(f.text_color || card.text_color, 'var(--color-text)'),
          fontFamily: fontFamily(card.font),
          animation: fadeMs ? `card-fade ${fadeMs}ms ease both` : undefined,
        }}
      >
        {f.heading && <Typed full={f.heading} typed={heading} className="text-2xl tracking-wide" />}
        {f.text && <Typed full={f.text} typed={text} className="whitespace-pre-wrap text-lg leading-relaxed" />}
      </div>
      {(isLast || (waits && textDone)) && (
        <div className="absolute bottom-6 left-0 right-0 text-center text-sm text-muted animate-pulse">
          {t('hud.clickToContinue')}
        </div>
      )}
    </div>
  )
}
