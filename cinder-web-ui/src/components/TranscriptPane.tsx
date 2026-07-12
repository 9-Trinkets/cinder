import { memo, useState, type MutableRefObject, type UIEvent } from 'react'
import * as api from '../api'
import TranscriptLine, { type Line } from './TranscriptLine'
import SessionClosureModal from './SessionClosureModal'
import Skeleton from './Skeleton'

const TranscriptPane = memo(function TranscriptPane({
  lines,
  busyLabel,
  sessionClosure,
  gameOver,
  transcriptRef,
  bottomRef,
  onScroll,
  onDismissClosure,
}: {
  lines: Line[]
  busyLabel: string | null
  sessionClosure: api.SessionClosureData | null
  gameOver: boolean
  transcriptRef: MutableRefObject<HTMLDivElement | null>
  bottomRef: MutableRefObject<HTMLDivElement | null>
  onScroll: (event: UIEvent<HTMLDivElement>) => void
  onDismissClosure: () => void
}) {
  const [searchOpen, setSearchOpen] = useState(false)
  const [searchQuery, setSearchQuery] = useState('')

  const matchCount = searchQuery
    ? lines.filter(l => l.text.toLowerCase().includes(searchQuery.toLowerCase())).length
    : 0

  return (
    <div className="flex-1 flex flex-col overflow-hidden">
      {searchOpen && (
        <div className="flex items-center gap-2 px-4 py-2 border-b border-subtle shrink-0">
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
      )}

      <div
        ref={transcriptRef}
        onScroll={onScroll}
        aria-live="polite"
        aria-label="Transcript"
        className="flex-1 overflow-y-auto px-4 py-4 space-y-3"
      >
        {!searchOpen && lines.length > 0 && (
          <button
            onClick={() => setSearchOpen(true)}
            aria-label="Search transcript"
            className="sticky top-0 float-right text-muted hover:text-text text-xs bg-surface/80 backdrop-blur-sm px-2 py-1 rounded border border-subtle z-10 cursor-pointer"
          >
            Search
          </button>
        )}
        {lines.map(line => (
          <TranscriptLine key={line.key} line={line} searchQuery={searchQuery} />
        ))}
        {busyLabel && lines.length === 0 && <Skeleton lines={4} className="mb-2" />}
        {busyLabel && <p className="text-muted text-sm italic">{busyLabel}</p>}
        {sessionClosure && (
          <SessionClosureModal sessionClosure={sessionClosure} onDismiss={onDismissClosure} />
        )}
        {gameOver && !sessionClosure && (
          <p className="text-love font-semibold text-center pt-4">Game Over</p>
        )}
        <div ref={bottomRef} />
      </div>
    </div>
  )
})

export default TranscriptPane
