import type * as api from '../api'
import StatusPanel from './StatusPanel'
import RelationshipChart from './RelationshipChart'

interface FolioPanelProps {
  uiSnapshot: api.UiSnapshot
  onTakeItem: (itemId: string) => void
  onOpenPanel: (panel: string) => void
}

export default function FolioPanel({
  uiSnapshot,
  onTakeItem,
  onOpenPanel,
}: FolioPanelProps) {
  return (
    <div className="space-y-4">
      <div className="pb-3 border-b border-subtle/50">
        <span className="text-[10px] font-mono uppercase tracking-widest text-muted block mb-1">
          Current Location
        </span>
        <h3 className="text-xl font-bold font-prose text-text tracking-tight">
          {uiSnapshot.current_room_name}
        </h3>
        <div className="flex items-center gap-2.5 text-xs text-muted mt-1 font-mono">
          <span>Day {uiSnapshot.day_number}{uiSnapshot.time_label ? ` — ${uiSnapshot.time_label}` : ''}</span>
          {uiSnapshot.followed_actor_name && (
            <>
              <span className="text-muted/40">&bull;</span>
              <span className="text-foam">Following {uiSnapshot.followed_actor_name}</span>
            </>
          )}
        </div>
      </div>

      <StatusPanel
        uiSnapshot={uiSnapshot}
        onTakeItem={onTakeItem}
        onOpenPanel={onOpenPanel}
        hideLocation
      />

      {uiSnapshot.show_relationship_sidebar && uiSnapshot.relationship_pairs.length > 0 && (
        <RelationshipChart pairs={uiSnapshot.relationship_pairs} />
      )}
    </div>
  )
}
