import { memo } from 'react'
import { HighlightedText } from './TranscriptLine'

export interface RoomCardProps {
  title: string
  body?: string
  searchQuery?: string
  craftedLabels?: string[]
  interactableLabels?: string[]
}

export const RoomCard = memo(function RoomCard({
  title,
  body,
  searchQuery,
  craftedLabels,
  interactableLabels,
}: RoomCardProps) {
  const cleanTitle = title.replace(/^==\s*|\s*==$/g, '').trim()

  return (
    <div className="my-5 rounded-xl bg-overlay/50 border border-subtle/80 overflow-hidden shadow-xs">
      <div className="px-4 py-2.5 bg-surface/80 border-b border-subtle/60 flex items-center justify-between gap-3">
        <div className="flex items-center gap-2.5 min-w-0">
          <span className="w-2 h-2 rounded-full bg-iris shrink-0" aria-hidden="true" />
          <h2 className="font-bold text-text text-sm sm:text-base tracking-wide truncate font-prose">
            <HighlightedText
              text={cleanTitle}
              query={searchQuery ?? ''}
              craftedLabels={craftedLabels ?? []}
              interactableLabels={interactableLabels ?? []}
            />
          </h2>
        </div>
        <span className="text-[10px] font-mono uppercase tracking-widest text-muted bg-overlay px-2 py-0.5 rounded border border-subtle/60 shrink-0">
          Room
        </span>
      </div>

      {body && (
        <div className="p-4 sm:p-5 text-sm leading-relaxed text-text/90 font-prose whitespace-pre-wrap">
          <HighlightedText
            text={body}
            query={searchQuery ?? ''}
            craftedLabels={craftedLabels ?? []}
            interactableLabels={interactableLabels ?? []}
          />
        </div>
      )}
    </div>
  )
})

export default RoomCard
