import { memo } from 'react'

export interface Line {
  text: string
  key: number
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

function HighlightedText({
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
  let className = 'text-text'
  let heading: string | null = null
  if (line.text.startsWith('> ')) {
    className = 'text-foam font-mono text-xs'
  } else if (line.text.startsWith('== ')) {
    // A room observation begins with a `== Title ==` heading. Color only the
    // heading as the accent, leaving the body in the default text color so an
    // entire room block isn't tinted.
    const match = line.text.match(/^(== [^\n]+)\n?([\s\S]*)$/)
    heading = match?.[1] ?? null
    className = 'text-iris font-bold'
  } else if (line.text.startsWith('[error:')) {
    className = 'text-love italic text-xs'
  }

  const body = heading ? line.text.slice(heading.length).replace(/^\n/, '') : line.text

  return (
    <div className="whitespace-pre-wrap text-sm leading-relaxed">
      {heading && <span className="text-iris font-bold">{heading}{'\n'}</span>}
      <span className={heading ? 'text-text' : className}>
        <HighlightedText
          text={body}
          query={searchQuery ?? ''}
          craftedLabels={craftedLabels ?? []}
          interactableLabels={interactableLabels ?? []}
        />
      </span>
    </div>
  )
})

export default TranscriptLine
