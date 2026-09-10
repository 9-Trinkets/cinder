import * as api from '../api'
import Minimap from './Minimap'
import Section from './Section'

export default function StatusPanel({
  uiSnapshot,
  onTakeItem,
  onOpenPanel,
}: {
  uiSnapshot: api.UiSnapshot
  onTakeItem?: (itemId: string) => void
  onOpenPanel?: (panel: string) => void
}) {
  const player = uiSnapshot.player
  return (
    <div>
      <Section title="Location" defaultOpen>
        <p className="text-text font-medium">{uiSnapshot.current_room_name}</p>
        <p className="text-text text-xs">
          Day {uiSnapshot.day_number}
          {uiSnapshot.time_label ? <span className="text-muted ml-1">— {uiSnapshot.time_label}</span> : null}
        </p>
      </Section>

      {uiSnapshot.minimap && (
        <Section title={uiSnapshot.ui_text.minimap_sidebar_label || 'Map'} defaultOpen>
          <Minimap map={uiSnapshot.minimap} uiText={uiSnapshot.ui_text} />
        </Section>
      )}

      {uiSnapshot.show_vitals_sidebar && (
        <>
          <Section title="Vitals" defaultOpen>
            <div className="flex items-baseline justify-between text-xs">
              <span className="text-text">HP</span>
              <span className="text-muted">{player.hp}/{player.hp_max}</span>
            </div>
            <div className="h-1.5 w-full rounded-full bg-overlay overflow-hidden">
              <div
                className="h-full rounded-full transition-all duration-300"
                style={{
                  width: `${player.hp_max > 0 ? (player.hp / player.hp_max) * 100 : 0}%`,
                  backgroundColor: 'var(--color-pine)',
                }}
              />
            </div>
            {player.stats.length > 0 && (
              <ul className="space-y-0.5">
                {player.stats.map(stat => (
                  <li key={stat.id} className="flex justify-between text-xs">
                    <span className="text-muted">{stat.id}</span>
                    <span className="text-text">{stat.value}</span>
                  </li>
                ))}
              </ul>
            )}
          </Section>

          {uiSnapshot.levels_revealed && (
            <Section title="Level" defaultOpen>
              <div className="flex items-baseline justify-between text-xs">
                <span className="text-text font-medium">Level {player.level}</span>
                <span className="text-muted">
                  {player.xp_max > 0 ? `${player.xp} / ${player.xp_max} XP` : 'MAX'}
                </span>
              </div>
              <div className="h-1.5 w-full rounded-full bg-overlay overflow-hidden">
                <div
                  role="progressbar"
                  aria-valuenow={player.xp}
                  aria-valuemin={0}
                  aria-valuemax={player.xp_max || 1}
                  className="h-full rounded-full transition-all duration-300"
                  style={{
                    width: `${player.xp_max > 0 ? (player.xp / player.xp_max) * 100 : 100}%`,
                    backgroundColor: 'var(--color-gold)',
                  }}
                />
              </div>
            </Section>
          )}
        </>
      )}

      {uiSnapshot.party.length > 0 && (
        <Section title="Party" defaultOpen>
          <ul className="space-y-2">
            {uiSnapshot.party.map(member => (
              <li key={member.id}>
                <button
                  type="button"
                  onClick={() => onOpenPanel?.(member.order_panel)}
                  className="group w-full rounded-lg border border-subtle bg-surface/40 px-2.5 py-2 text-left transition duration-200 hover:border-pine/50 hover:bg-overlay cursor-pointer"
                >
                  <span className="flex items-start justify-between gap-2">
                    <span className="min-w-0">
                      <span className="block truncate text-xs font-medium text-text">{member.label}</span>
                      <span className="mt-0.5 block text-[10px] uppercase tracking-[0.14em] text-pine">
                        {member.order === 'guard' ? 'Guarding' : 'Assisting'}
                        {uiSnapshot.levels_revealed ? ` · Lv ${member.level}` : ''}
                      </span>
                    </span>
                    <span className="text-muted transition-transform duration-200 group-hover:translate-x-0.5">›</span>
                  </span>
                  <span className="mt-1.5 flex items-center gap-2">
                    <span className="h-1 flex-1 overflow-hidden rounded-full bg-overlay">
                      <span
                        className="block h-full rounded-full bg-pine transition-[width] duration-300"
                        style={{ width: `${member.hp_max > 0 ? (member.hp / member.hp_max) * 100 : 0}%` }}
                      />
                    </span>
                    <span className="text-[10px] tabular-nums text-muted">{member.hp}/{member.hp_max}</span>
                  </span>
                </button>
              </li>
            ))}
          </ul>
        </Section>
      )}

      {uiSnapshot.equipped_items.length > 0 && (
        <Section title="Equipped">
          <ul className="space-y-0.5">
            {uiSnapshot.equipped_items.map((item, i) => (
              <li key={i} className="text-text text-xs">
                • <span className="text-muted">{item.slot}:</span> {item.label}
              </li>
            ))}
          </ul>
        </Section>
      )}

      {uiSnapshot.inventory.length > 0 && (
        <Section title="Inventory">
          <ul className="space-y-0.5">
            {uiSnapshot.inventory.map((item, i) => (
              <li key={i} className="text-text text-xs">
                • {item.label}{item.count > 1 ? <span className="text-muted ml-1">×{item.count}</span> : null}
              </li>
            ))}
          </ul>
        </Section>
      )}

      {uiSnapshot.current_room_items.length > 0 && (
        <Section title={uiSnapshot.ui_text.room_items_sidebar_label || 'On the ground'}>
          <ul className="space-y-0.5">
            {uiSnapshot.current_room_items.map((item, i) => (
              <li key={i} className="text-text text-xs">
                {(onTakeItem && item.id)
                  ? (
                    <button
                      type="button"
                      onClick={() => onTakeItem(item.id!)}
                      className="hover:opacity-80 cursor-pointer text-left"
                    >
                      Take {item.label}
                      {item.count > 1 ? <span className="text-muted ml-1">×{item.count}</span> : null}
                    </button>
                  )
                  : (
                    <span>
                      • {item.label}
                      {item.count > 1 ? <span className="text-muted ml-1">×{item.count}</span> : null}
                    </span>
                  )}
              </li>
            ))}
          </ul>
        </Section>
      )}

      {uiSnapshot.room_consumables.length > 0 && (
        <Section title="Available">
          {uiSnapshot.room_consumables.map((group, gi) => (
            <div key={gi}>
              <p className="text-text text-xs font-medium">{group.feature_label}</p>
              <ul className="space-y-0.5">
                {group.items.map(item => (
                  <li key={item.id} className="text-text text-xs">
                    • <span
                      className={item.is_crafted ? 'font-medium' : undefined}
                      style={item.is_crafted ? { color: 'var(--color-crafted-highlight)' } : undefined}
                    >{item.label}</span>
                    {item.stock > 1 ? <span className="text-muted ml-1">×{item.stock}</span> : null}
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </Section>
      )}

      {uiSnapshot.progress_total > 0 && (
        <Section title="Progress">
          <div className="h-1.5 w-full bg-overlay rounded-full overflow-hidden">
            <div
              role="progressbar"
              aria-valuenow={uiSnapshot.progress_completed}
              aria-valuemin={0}
              aria-valuemax={uiSnapshot.progress_total}
              className="h-full bg-pine rounded-full transition-all duration-500"
              style={{ width: `${(uiSnapshot.progress_completed / uiSnapshot.progress_total) * 100}%` }}
            />
          </div>
        </Section>
      )}

      {uiSnapshot.secrets_total > 0 && (
        <Section title="Secrets Found">
          <p className="text-text font-medium">{uiSnapshot.secrets_found} / {uiSnapshot.secrets_total}</p>
        </Section>
      )}

      <Section title="What now?" defaultOpen>
        <p className="text-text text-xs leading-relaxed">
          {uiSnapshot.objective_message || 'No current objective.'}
        </p>
      </Section>
    </div>
  )
}
