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
      <div className="pt-8 pb-3 my-2">
        <div className="flex items-center gap-2 mb-1.5">
          <span className="text-[10px] font-mono uppercase tracking-widest text-muted">
            Location
          </span>
        </div>
        <h2 className="text-xl sm:text-2xl font-bold font-prose text-text tracking-tight">
          <HighlightedText
            text={cleanHeading}
            query={searchQuery ?? ''}
            craftedLabels={[]}
            interactableLabels={[]}
          />
        </h2>
        <div className="h-px bg-gradient-to-r from-subtle/90 via-subtle/40 to-transparent mt-3" />
      </div>
    )
  }

  // 2. Channel / Handler Comms
  if (line.kind === 'channel') {
    const match = line.text.match(/^([^:]+):\s*(.*)$/s)
    const speaker = match ? match[1].trim() : 'Comms'
    const content = match ? match[2].trim() : line.text

    return (
      <div className="my-3.5 pl-4 border-l-2 border-rose/60 font-prose">
        <span className="font-mono text-[10px] font-semibold tracking-widest uppercase text-rose block mb-1">
          Dispatch &bull; {speaker}
        </span>
        <div className="text-text text-base leading-[1.8] italic">
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
      <div className="my-2 py-0.5 font-mono text-xs text-muted/80 flex items-center gap-2">
        <span className="text-muted/40 select-none font-bold" aria-hidden="true">&rsaquo;</span>
        <span className="font-medium text-foam/90">
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
        className="my-2 pl-3 py-1 border-l border-crt-glow/40 text-xs font-mono"
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
      <div className="my-2 pl-3 py-1 border-l-2 border-love text-xs text-love font-mono flex items-center gap-2">
        <span className="font-semibold uppercase tracking-wider opacity-80 shrink-0">[Note]</span>
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

  // 6. In-Room Character Dialogue (e.g. "Bess: ...", "Daichi (to Ren): ...")
  const dialogueMatch = line.text.match(
    /^([A-Z\u4e00-\u9fa5][a-zA-Z0-9_\s.'-\u4e00-\u9fa5]{0,24}?)(?:\s*(?:\(\s*(?:to\s+)?([^)]+)\)|（\s*(?:對\s*)?([^）]+)）))?\s*[:：]\s*(.*)$/s
  )
  if (dialogueMatch) {
    const speaker = dialogueMatch[1].trim()
    const target = (dialogueMatch[2] || dialogueMatch[3])?.trim()
    const speech = dialogueMatch[4].trim()

    if (speaker && speech) {
      return (
        <div className="my-3.5 pl-4 border-l-2 border-pine/60 font-prose">
          <div className="flex items-baseline gap-1.5 mb-1">
            <span className="font-mono text-[10px] font-semibold tracking-widest uppercase text-foam">
              {speaker}
            </span>
            {target && (
              <span className="font-prose text-xs text-muted italic">
                (to {target})
              </span>
            )}
          </div>
          <div className="text-text text-base leading-[1.8]">
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
  }

  // 7. Standard Sensory Narration
  return (
    <div className="whitespace-pre-wrap text-base leading-[1.8] py-1 text-text font-prose">
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
