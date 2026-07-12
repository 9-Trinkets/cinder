import { memo } from 'react'

export interface Line {
  text: string
  key: number
}

function HighlightedText({ text, query }: { text: string; query: string }) {
  if (!query) return <>{text}</>

  const lower = text.toLowerCase()
  const qLower = query.toLowerCase()
  const parts: { text: string; match: boolean }[] = []
  let lastIdx = 0

  let idx = lower.indexOf(qLower, lastIdx)
  while (idx !== -1) {
    if (idx > lastIdx) {
      parts.push({ text: text.slice(lastIdx, idx), match: false })
    }
    parts.push({ text: text.slice(idx, idx + query.length), match: true })
    lastIdx = idx + query.length
    idx = lower.indexOf(qLower, lastIdx)
  }
  if (lastIdx < text.length) {
    parts.push({ text: text.slice(lastIdx), match: false })
  }

  return (
    <>
      {parts.map((part, i) =>
        part.match
          ? <mark key={i} className="bg-gold/30 text-text rounded px-0.5">{part.text}</mark>
          : <span key={i}>{part.text}</span>
      )}
    </>
  )
}

const TranscriptLine = memo(function TranscriptLine({
  line,
  searchQuery,
}: {
  line: Line
  searchQuery?: string
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
        <HighlightedText text={line.text} query={searchQuery ?? ''} />
      </span>
    </div>
  )
})

export default TranscriptLine
