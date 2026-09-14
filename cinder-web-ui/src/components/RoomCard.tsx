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
    <div className="my-5 rounded-xl bg-overlay/40 border border-subtle overflow-hidden shadow-xs">
      <div className="px-4 py-3 bg-overlay/80 border-b border-subtle flex items-center justify-between gap-3">
        <div className="flex items-center gap-2.5 min-w-0">
          <span className="w-2.5 h-2.5 rounded-full bg-foam shadow-[0_0_8px_rgba(156,207,216,0.4)] shrink-0" aria-hidden="true" />
          <h2 className="font-bold text-text text-base sm:text-lg tracking-wide truncate font-prose">
            <HighlightedText
              text={cleanTitle}
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
