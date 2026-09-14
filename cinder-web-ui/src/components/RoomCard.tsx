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
    <article className="pt-8 pb-4 my-2">
      <header className="mb-4">
        <div className="flex items-center gap-2 mb-1.5">
          <span className="text-[10px] font-mono uppercase tracking-widest text-muted">
            Location
          </span>
        </div>
        <h2 className="text-xl sm:text-2xl font-bold font-prose text-text tracking-tight">
          <HighlightedText
            text={cleanTitle}
            query={searchQuery ?? ''}
            craftedLabels={[]}
            interactableLabels={[]}
          />
        </h2>
        <div className="h-px bg-gradient-to-r from-subtle/90 via-subtle/40 to-transparent mt-3" />
      </header>

      {body && (
        <div className="text-base leading-[1.8] text-text font-prose whitespace-pre-wrap">
          <HighlightedText
            text={body}
            query={searchQuery ?? ''}
            craftedLabels={craftedLabels ?? []}
            interactableLabels={interactableLabels ?? []}
          />
        </div>
      )}
    </article>
  )
})

export default RoomCard
