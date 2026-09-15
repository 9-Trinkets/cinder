import { memo, useMemo, useState, type MutableRefObject, type UIEvent } from 'react'
import * as api from '../api'
import TranscriptLine, { type Line } from './TranscriptLine'
import RoomCard from './RoomCard'
import ActClosureModal from './ActClosureModal'
import Skeleton from './Skeleton'

type TranscriptItem =
  | {
      type: 'room_card'
      key: number
      title: string
      body?: string
    }
  | {
      type: 'line'
      key: number
      line: Line
    }

function groupTranscriptLines(lines: Line[]): TranscriptItem[] {
  const items: TranscriptItem[] = []
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]

    // 1. Single combined line: "== Room Name ==\n\nDescription..."
    const combinedMatch = line.text.match(/^==\s*([^=\n]+?)\s*==\s*\n+([\s\S]+)$/)
    if (combinedMatch) {
      items.push({
        type: 'room_card',
        key: line.key,
        title: combinedMatch[1].trim(),
        body: combinedMatch[2].trim(),
      })
      continue
    }

    // 2. Heading line: "== Room Name =="
    const isHeading = line.kind === 'heading' || /^==\s*[^=\n]+?\s*==$/.test(line.text.trim())
    if (isHeading) {
      const cleanTitle = line.text.replace(/^==\s*|\s*==$/g, '').trim()
      const nextLine = lines[i + 1]

      // Check if following line is the room description
      const nextIsNarration =
        nextLine &&
        (nextLine.kind === 'narration' || nextLine.kind === undefined) &&
        !/^==\s*[^=\n]+?\s*==$/.test(nextLine.text.trim()) &&
        !nextLine.text.match(/^([A-Z][a-zA-Z0-9_\s]{1,24}):\s*(["“].*|[A-Za-z].*)$/s)

      if (nextIsNarration) {
        items.push({
          type: 'room_card',
          key: line.key,
          title: cleanTitle,
          body: nextLine.text,
        })
        i++ // Advance past the paired narration
        continue
      }

      // Standalone heading
      items.push({
        type: 'room_card',
        key: line.key,
        title: cleanTitle,
      })
      continue
    }

    items.push({
      type: 'line',
      key: line.key,
      line,
    })
  }
  return items
}

const TranscriptPane = memo(function TranscriptPane({
  lines,
  busyLabel,
  actClosure,
  gameClosure,
  gameOver,
  transcriptRef,
  bottomRef,
  onScroll,
  onDismissClosure,
  onDismissGameClosure,
  craftedLabels,
  interactableLabels,
}: {
  lines: Line[]
  busyLabel: string | null
  actClosure: api.ActClosureData | null
  gameClosure: api.ActClosureData | null
  gameOver: boolean
  transcriptRef: MutableRefObject<HTMLDivElement | null>
  bottomRef: MutableRefObject<HTMLDivElement | null>
  onScroll: (event: UIEvent<HTMLDivElement>) => void
  onDismissClosure: () => void
  onDismissGameClosure: () => void
  craftedLabels: string[]
  interactableLabels?: string[]
}) {
  const [searchOpen, setSearchOpen] = useState(false)
  const [searchQuery, setSearchQuery] = useState('')

  const transcriptItems = useMemo(() => groupTranscriptLines(lines), [lines])

  const matchCount = searchQuery
    ? lines.filter(l => l.text.toLowerCase().includes(searchQuery.toLowerCase())).length
    : 0

  return (
    <div className="flex-1 flex flex-col overflow-hidden">
      {searchOpen && (
        <div className="border-b border-subtle bg-surface/95 px-4 py-2 shrink-0">
          <div className="max-w-2xl mx-auto w-full flex items-center gap-2">
            <input
              type="text"
              value={searchQuery}
              onChange={e => setSearchQuery(e.target.value)}
              placeholder="Search transcript..."
              autoFocus
              className="flex-1 px-3 py-1.5 rounded bg-overlay border border-subtle text-text text-sm placeholder-muted focus:outline-none focus:border-pine"
            />
            {searchQuery && (
              <span className="text-xs text-muted shrink-0">{matchCount} match{matchCount !== 1 ? 'es' : ''}</span>
            )}
            <button
              onClick={() => { setSearchOpen(false); setSearchQuery('') }}
              aria-label="Close search"
              className="text-muted hover:text-text text-lg leading-none cursor-pointer"
            >
              &times;
            </button>
          </div>
        </div>
      )}

      <div
        ref={transcriptRef}
        onScroll={onScroll}
        aria-live="polite"
        aria-label="Transcript"
        className="flex-1 overflow-y-auto px-4 sm:px-6 py-6"
      >
        <div className="max-w-2xl mx-auto w-full space-y-4">
          {!searchOpen && lines.length > 0 && (
            <div className="sticky top-0 z-10 flex justify-end">
              <button
                onClick={() => setSearchOpen(true)}
                aria-label="Search transcript"
                className="text-muted hover:text-text text-xs bg-surface/90 backdrop-blur-sm px-2.5 py-1 rounded-md border border-subtle cursor-pointer shadow-xs transition-colors"
              >
                Search
              </button>
            </div>
          )}
          {transcriptItems.map(item =>
            item.type === 'room_card' ? (
              <RoomCard
                key={item.key}
                title={item.title}
                body={item.body}
                searchQuery={searchQuery}
                craftedLabels={craftedLabels}
                interactableLabels={interactableLabels}
              />
            ) : (
              <TranscriptLine
                key={item.key}
                line={item.line}
                searchQuery={searchQuery}
                craftedLabels={craftedLabels}
                interactableLabels={interactableLabels}
              />
            ),
          )}
          {busyLabel && lines.length === 0 && <Skeleton lines={4} className="mb-2" />}
          {busyLabel && (
            <div className="flex items-center gap-2.5 py-3 text-muted text-sm font-serif italic tracking-wide">
              <span className="inline-block w-1.5 h-1.5 rounded-full bg-accent/70 animate-pulse" />
              <span>{busyLabel}</span>
            </div>
          )}
          {actClosure && (
            <ActClosureModal actClosure={actClosure} onDismiss={onDismissClosure} />
          )}
          {gameClosure && (
            <ActClosureModal actClosure={gameClosure} onDismiss={onDismissGameClosure} />
          )}
          {gameOver && !actClosure && !gameClosure && (
            <p className="text-love font-semibold text-center pt-4">Game Over</p>
          )}
          <div ref={bottomRef} />
        </div>
      </div>
    </div>
  )
})

export default TranscriptPane
