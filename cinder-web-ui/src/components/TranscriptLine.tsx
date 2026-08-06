import { memo } from 'react'

export interface Line {
  text: string
  key: number
}

type TextSegment = {
  text: string
  match: boolean
  crafted: boolean
}

function splitByMatch(text: string, query: string): TextSegment[] {
  if (!query) return [{ text, match: false, crafted: false }]

  const lower = text.toLowerCase()
  const qLower = query.toLowerCase()
  const parts: TextSegment[] = []
  let lastIdx = 0

  let idx = lower.indexOf(qLower, lastIdx)
  while (idx !== -1) {
    if (idx > lastIdx) {
      parts.push({ text: text.slice(lastIdx, idx), match: false, crafted: false })
    }
    parts.push({ text: text.slice(idx, idx + query.length), match: true, crafted: false })
    lastIdx = idx + query.length
    idx = lower.indexOf(qLower, lastIdx)
  }
  if (lastIdx < text.length) {
    parts.push({ text: text.slice(lastIdx), match: false, crafted: false })
  }
  return parts
}

function splitSegmentByCrafted(segment: TextSegment, craftedLabels: string[]): TextSegment[] {
  if (segment.match || craftedLabels.length === 0) return [segment]

  let earliestIndex = -1
  let matchedLabel = ''
  const lower = segment.text.toLowerCase()
  for (const label of craftedLabels) {
    if (!label) continue
    const idx = lower.indexOf(label.toLowerCase())
    if (idx !== -1 && (earliestIndex === -1 || idx < earliestIndex || (idx === earliestIndex && label.length > matchedLabel.length))) {
      earliestIndex = idx
      matchedLabel = label
    }
  }
  if (earliestIndex === -1) return [segment]

  const parts: TextSegment[] = []
  if (earliestIndex > 0) {
    parts.push(...splitSegmentByCrafted({
      text: segment.text.slice(0, earliestIndex),
      match: false,
      crafted: false,
    }, craftedLabels))
  }
  parts.push({
    text: segment.text.slice(earliestIndex, earliestIndex + matchedLabel.length),
    match: false,
    crafted: true,
  })
  const rest = segment.text.slice(earliestIndex + matchedLabel.length)
  if (rest) {
    parts.push(...splitSegmentByCrafted({
      text: rest,
      match: false,
      crafted: false,
    }, craftedLabels))
  }
  return parts
}

function HighlightedText({
  text,
  query,
  craftedLabels,
}: {
  text: string
  query: string
  craftedLabels: string[]
}) {
  const parts = splitByMatch(text, query).flatMap(part => splitSegmentByCrafted(part, craftedLabels))

  return (
    <>
      {parts.map((part, i) => {
        if (part.match) {
          return <mark key={i} className="bg-gold/30 text-text rounded px-0.5">{part.text}</mark>
        }
        if (part.crafted) {
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
}: {
  line: Line
  searchQuery?: string
  craftedLabels?: string[]
}) {
  let className = 'text-text'
  if (line.text.startsWith('> ')) {
    className = 'text-foam font-mono text-xs'
  } else if (line.text.startsWith('== ')) {
    className = 'text-iris font-bold'
  } else if (line.text.startsWith('[error:')) {
    className = 'text-love italic text-xs'
  }

  return (
    <div className="whitespace-pre-wrap text-sm leading-relaxed">
      <span className={className}>
        <HighlightedText text={line.text} query={searchQuery ?? ''} craftedLabels={craftedLabels ?? []} />
      </span>
    </div>
  )
})

export default TranscriptLine
