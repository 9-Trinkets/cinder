import { memo, type CSSProperties } from 'react'
import type { LineKind } from '../api'

export type { LineKind }

export interface Line {
  text: string
  key: number
  kind?: LineKind
}

type SegmentKind = 'plain' | 'match' | 'crafted' | 'interactable'

type TextSegment = {
  text: string
  kind: SegmentKind
}

type LabelHighlight = {
  label: string
  kind: Exclude<SegmentKind, 'plain' | 'match'>
}

function splitByQuery(text: string, query: string): TextSegment[] {
  if (!query) return [{ text, kind: 'plain' }]

  const lower = text.toLowerCase()
  const qLower = query.toLowerCase()
  const parts: TextSegment[] = []
  let lastIdx = 0

  let idx = lower.indexOf(qLower, lastIdx)
  while (idx !== -1) {
    if (idx > lastIdx) {
      parts.push({ text: text.slice(lastIdx, idx), kind: 'plain' })
    }
    parts.push({ text: text.slice(idx, idx + query.length), kind: 'match' })
    lastIdx = idx + query.length
    idx = lower.indexOf(qLower, lastIdx)
  }
  if (lastIdx < text.length) {
    parts.push({ text: text.slice(lastIdx), kind: 'plain' })
  }
  return parts
}

function splitSegmentByLabels(
  segment: TextSegment,
  highlights: LabelHighlight[],
): TextSegment[] {
  if (segment.kind !== 'plain' || highlights.length === 0) return [segment]

  let earliestIndex = -1
  let matched: LabelHighlight | null = null
  const lower = segment.text.toLowerCase()
  for (const highlight of highlights) {
    if (!highlight.label) continue
    const idx = lower.indexOf(highlight.label.toLowerCase())
    if (
      idx !== -1 &&
      (earliestIndex === -1 ||
        idx < earliestIndex ||
        (idx === earliestIndex && matched !== null && highlight.label.length > matched.label.length))
    ) {
      earliestIndex = idx
      matched = highlight
    }
  }
  if (matched === null || earliestIndex === -1) return [segment]

  const parts: TextSegment[] = []
  if (earliestIndex > 0) {
    parts.push(
      ...splitSegmentByLabels(
        { text: segment.text.slice(0, earliestIndex), kind: 'plain' },
        highlights,
      ),
    )
  }
  parts.push({
    text: segment.text.slice(earliestIndex, earliestIndex + matched.label.length),
    kind: matched.kind,
  })
  const rest = segment.text.slice(earliestIndex + matched.label.length)
  if (rest) {
    parts.push(...splitSegmentByLabels({ text: rest, kind: 'plain' }, highlights))
  }
  return parts
}

export function HighlightedText({
  text,
  query,
  craftedLabels,
  interactableLabels,
}: {
  text: string
  query: string
  craftedLabels: string[]
  interactableLabels: string[]
}) {
  const highlights: LabelHighlight[] = [
    ...craftedLabels.map(label => ({ label, kind: 'crafted' as const })),
    ...interactableLabels.map(label => ({ label, kind: 'interactable' as const })),
  ]
  const parts = splitByQuery(text, query).flatMap(part =>
    splitSegmentByLabels(part, highlights),
  )

  return (
    <>
      {parts.map((part, i) => {
        if (part.kind === 'match') {
          return <mark key={i} className="bg-gold/30 text-text rounded px-0.5">{part.text}</mark>
        }
        if (part.kind === 'crafted') {
          return <span key={i} className="font-medium" style={{ color: 'var(--color-crafted-highlight)' }}>{part.text}</span>
        }
        if (part.kind === 'interactable') {
          return <span key={i} className="font-medium" style={{ color: 'var(--color-crafted-highlight)' }}>{part.text}</span>
        }
        return <span key={i}>{part.text}</span>
      })}
    </>
  )
}

const TranscriptLine = memo(function TranscriptLine({
  line,
  searchQuery,
  craftedLabels,
  interactableLabels,
}: {
  line: Line
  searchQuery?: string
  craftedLabels?: string[]
  interactableLabels?: string[]
}) {
  // 1. Room Transition Banner (Heading)
  if (line.kind === 'heading') {
    const cleanHeading = line.text.replace(/^==\s*|\s*==$/g, '').trim()
    return (
      <div className="my-5 py-3 px-4 rounded-xl bg-overlay/80 border border-subtle flex items-center justify-between gap-3 shadow-xs">
        <div className="flex items-center gap-2.5 min-w-0">
          <span className="w-2.5 h-2.5 rounded-full bg-foam shadow-[0_0_8px_rgba(156,207,216,0.4)] shrink-0" aria-hidden="true" />
          <h2 className="font-bold text-text text-base sm:text-lg tracking-wide truncate font-prose">
            <HighlightedText
              text={cleanHeading}
              query={searchQuery ?? ''}
              craftedLabels={[]}
              interactableLabels={[]}
            />
          </h2>
        </div>
        <span className="text-[10px] font-mono uppercase tracking-widest text-foam bg-pine/15 px-2 py-0.5 rounded border border-pine/30 shrink-0">
          Room
        </span>
      </div>
    )
  }

  // 2. Channel / Handler Comms
  if (line.kind === 'channel') {
    const match = line.text.match(/^([^:]+):\s*(.*)$/s)
    const speaker = match ? match[1].trim() : 'Comms'
    const content = match ? match[2].trim() : line.text

    return (
      <div className="my-2.5 pl-3.5 pr-4 py-2 rounded-r-xl border-l-2 border-rose/80 bg-rose/5 text-sm leading-relaxed font-prose">
        <div className="flex items-center gap-2 mb-1">
          <span className="w-1.5 h-1.5 rounded-full bg-rose shrink-0" aria-hidden="true" />
          <span className="font-mono text-[11px] font-semibold tracking-wider uppercase text-rose">
            Comms &bull; {speaker}
          </span>
        </div>
        <div className="text-text italic text-sm">
          <HighlightedText
            text={content}
            query={searchQuery ?? ''}
            craftedLabels={craftedLabels ?? []}
            interactableLabels={interactableLabels ?? []}
          />
        </div>
      </div>
    )
  }

  // 3. Player Command Echo
  if (line.kind === 'player') {
    const rawText = line.text.startsWith('>') ? line.text.slice(1).trim() : line.text
    return (
      <div className="my-1.5 py-0.5 font-mono text-xs text-foam flex items-center gap-2">
        <span className="text-muted/60 select-none font-bold font-mono" aria-hidden="true">&gt;</span>
        <span className="font-medium">
          <HighlightedText
            text={rawText}
            query={searchQuery ?? ''}
            craftedLabels={craftedLabels ?? []}
            interactableLabels={interactableLabels ?? []}
          />
        </span>
      </div>
    )
  }

  // 4. System / Teaching Line
  if (line.kind === 'system') {
    return (
      <div
        className="my-1.5 py-1 px-3 rounded bg-crt-glow/5 border border-crt-glow/20 text-xs font-mono"
        style={{ color: 'var(--color-crt-glow)' }}
      >
        <HighlightedText
          text={line.text}
          query={searchQuery ?? ''}
          craftedLabels={craftedLabels ?? []}
          interactableLabels={interactableLabels ?? []}
        />
      </div>
    )
  }

  // 5. Error Feedback
  if (line.kind === 'error') {
    return (
      <div className="my-1.5 pl-3 pr-2 py-1.5 border-l-2 border-love bg-love/10 text-xs text-love font-mono flex items-center gap-2 rounded-r">
        <span className="font-bold text-[11px] uppercase tracking-wide opacity-80 shrink-0">[Error]</span>
        <div>
          <HighlightedText
            text={line.text}
            query={searchQuery ?? ''}
            craftedLabels={craftedLabels ?? []}
            interactableLabels={interactableLabels ?? []}
          />
        </div>
      </div>
    )
  }

  // 6. In-Room Character Dialogue (e.g. "Bess: ...", "Elder Valen: ...")
  const dialogueMatch = line.text.match(/^([A-Z][a-zA-Z0-9_\s]{1,24}):\s*(["“].*|[A-Za-z].*)$/s)
  if (dialogueMatch) {
    const speaker = dialogueMatch[1].trim()
    const speech = dialogueMatch[2].trim()
    return (
      <div className="my-2.5 pl-3.5 pr-4 py-2 rounded-r-xl border-l-2 border-pine/80 bg-pine/5 text-sm leading-relaxed font-prose">
        <div className="flex items-center gap-2 mb-1">
          <span className="w-1.5 h-1.5 rounded-full bg-foam shrink-0" aria-hidden="true" />
          <span className="font-semibold text-foam text-xs tracking-wide uppercase">
            {speaker}
          </span>
        </div>
        <div className="text-text text-sm">
          <HighlightedText
            text={speech}
            query={searchQuery ?? ''}
            craftedLabels={craftedLabels ?? []}
            interactableLabels={interactableLabels ?? []}
          />
        </div>
      </div>
    )
  }

  // 7. Standard Sensory Narration
  return (
    <div className="whitespace-pre-wrap text-sm leading-relaxed py-0.5 text-text font-prose">
      <HighlightedText
        text={line.text}
        query={searchQuery ?? ''}
        craftedLabels={craftedLabels ?? []}
        interactableLabels={interactableLabels ?? []}
      />
    </div>
  )
})

export default TranscriptLine
