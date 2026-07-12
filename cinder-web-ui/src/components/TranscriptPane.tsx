import { memo, type MutableRefObject, type UIEvent } from 'react'
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
  return (
    <div
      ref={transcriptRef}
      onScroll={onScroll}
      className="flex-1 overflow-y-auto px-4 py-4 space-y-3"
    >
      {lines.map(line => (
        <TranscriptLine key={line.key} line={line} />
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
  )
})

export default TranscriptPane
