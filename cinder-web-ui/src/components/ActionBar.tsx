import { memo } from 'react'
import * as api from '../api'
import { titleize } from '../utils/text'

export function getActionMeta(actionId: string, label: string): { borderClass: string; textClass: string; bgClass: string } {
  const id = actionId.toLowerCase()
  const lbl = label.toLowerCase()
  if (id.includes('look') || lbl.includes('look')) {
    return { borderClass: 'border-iris/40 hover:border-iris', textClass: 'text-iris', bgClass: 'bg-iris/10 hover:bg-iris/20' }
  }
  if (id.includes('move') || lbl.includes('move')) {
    return { borderClass: 'border-pine/40 hover:border-pine', textClass: 'text-foam', bgClass: 'bg-pine/10 hover:bg-pine/20' }
  }
  if (id.includes('attack') || lbl.includes('attack') || id.includes('strike')) {
    return { borderClass: 'border-love/40 hover:border-love', textClass: 'text-love font-medium', bgClass: 'bg-love/10 hover:bg-love/20' }
  }
  if (id.includes('speak') || lbl.includes('talk') || id.includes('talk')) {
    return { borderClass: 'border-rose/40 hover:border-rose', textClass: 'text-rose', bgClass: 'bg-rose/10 hover:bg-rose/20' }
  }
  if (id.includes('trace') || lbl.includes('trace') || id.includes('sigil')) {
    return { borderClass: 'border-gold/40 hover:border-gold', textClass: 'text-gold font-medium', bgClass: 'bg-gold/10 hover:bg-gold/20' }
  }
  if (id.includes('take') || lbl.includes('take') || id.includes('item') || id.includes('equip')) {
    return { borderClass: 'border-gold/40 hover:border-gold', textClass: 'text-gold', bgClass: 'bg-gold/10 hover:bg-gold/20' }
  }
  if (id.includes('follow') || lbl.includes('follow')) {
    return { borderClass: 'border-pine/40 hover:border-pine', textClass: 'text-foam', bgClass: 'bg-pine/10 hover:bg-pine/20' }
  }
  return { borderClass: 'border-subtle hover:border-text/40', textClass: 'text-text', bgClass: 'bg-overlay hover:bg-highlight-low' }
}

export interface ActionBarProps {
  actions: api.ActionBarAction[]
  roomItems?: api.InventoryItem[]
  hasOverflow: boolean
  busy: boolean
  gameOver: boolean
  onAction: (action: api.ActionBarAction) => void
  onTakeItem: (item: api.InventoryItem) => void
  onToggleOverflow: () => void
}

export const ActionBar = memo(function ActionBar({
  actions,
  roomItems = [],
  hasOverflow,
  busy,
  gameOver,
  onAction,
  onTakeItem,
  onToggleOverflow,
}: ActionBarProps) {
  const takeableRoomItems = roomItems.filter(item => {
    const id = (item.id ?? item.label).toLowerCase()
    return !id.includes('sigil') && !item.label.toLowerCase().includes('sigil')
  })

  return (
    <div className="border-t border-subtle/50 bg-surface/90">
      <div className="max-w-2xl mx-auto flex flex-wrap items-center gap-1.5 px-4 py-2">
        {actions.map((action, idx) => {
          const meta = getActionMeta(action.id, action.label)
          const shortcutNum = idx < 9 ? idx + 1 : undefined
          const label = titleize(action.label)
          return (
            <button
              key={action.id}
              onClick={() => onAction(action)}
              disabled={busy || gameOver}
              title={`Shortcut: ${shortcutNum ? `${shortcutNum} (or Alt+${shortcutNum})` : 'Action'}`}
              className={`px-2.5 py-1 rounded-md border text-xs sm:text-sm font-medium flex items-center gap-1 transition-all duration-150 active:scale-[0.97] disabled:opacity-50 cursor-pointer ${meta.borderClass} ${meta.bgClass} ${meta.textClass}`}
            >
              <span>{label}</span>
              {shortcutNum && (
                <span className="hidden sm:inline-block font-mono text-[10px] opacity-40 ml-0.5 select-none">
                  {shortcutNum}
                </span>
              )}
            </button>
          )
        })}

        {takeableRoomItems.map((item, idx) => {
          const baseCount = actions.length
          const shortcutNum = baseCount + idx < 9 ? baseCount + idx + 1 : undefined
          const label = titleize(item.label)
          return (
            <button
              key={`room-item-${item.id ?? item.label}-${idx}`}
              onClick={() => onTakeItem(item)}
              disabled={busy || gameOver}
              title={`Take ${label}${shortcutNum ? ` (Shortcut: ${shortcutNum})` : ''}`}
              className="px-2.5 py-1 rounded-md border border-gold/40 hover:border-gold bg-gold/10 hover:bg-gold/20 text-gold text-xs sm:text-sm font-medium flex items-center gap-1 transition-all duration-150 active:scale-[0.97] disabled:opacity-50 cursor-pointer"
            >
              <span>Take {label}{item.count > 1 ? ` (${item.count})` : ''}</span>
              {shortcutNum && (
                <span className="hidden sm:inline-block font-mono text-[10px] opacity-40 ml-0.5 select-none">
                  {shortcutNum}
                </span>
              )}
            </button>
          )
        })}

        {hasOverflow && (
          <button
            onClick={onToggleOverflow}
            disabled={busy || gameOver}
            aria-label="More actions"
            title="More actions (Shortcut: 0 or Alt+0)"
            className="px-2.5 py-1 rounded-md bg-overlay hover:bg-highlight-low border border-subtle text-muted hover:text-text text-xs sm:text-sm font-medium transition-all duration-150 active:scale-[0.97] disabled:opacity-50 cursor-pointer flex items-center gap-1"
          >
            <span className="font-mono text-xs tracking-widest px-0.5">&hellip;</span>
            <span className="hidden sm:inline-block font-mono text-[10px] opacity-40 ml-0.5 select-none">
              0
            </span>
          </button>
        )}
      </div>
    </div>
  )
})

export default ActionBar
