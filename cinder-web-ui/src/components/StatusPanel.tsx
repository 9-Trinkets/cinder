import * as api from '../api'

export default function StatusPanel({ uiSnapshot }: { uiSnapshot: api.UiSnapshot }) {
  return (
    <div className="space-y-4">
      <div>
        <p className="text-xs text-muted uppercase tracking-wider">Location</p>
        <p className="text-text font-medium">{uiSnapshot.current_room_name}</p>
      </div>
      <div>
        <p className="text-xs text-muted uppercase tracking-wider">Time</p>
        <p className="text-text">
          Day {uiSnapshot.day_number}
          {uiSnapshot.time_label ? <span className="text-muted ml-1">— {uiSnapshot.time_label}</span> : null}
        </p>
      </div>
      <div>
        <p className="text-xs text-muted uppercase tracking-wider">Vitals</p>
        <div className="mt-1">
          <div className="flex items-baseline justify-between text-xs">
            <span className="text-text">HP</span>
            <span className="text-muted">{uiSnapshot.player.hp}/{uiSnapshot.player.hp_max}</span>
          </div>
          <div className="mt-1 h-1.5 w-full rounded-full bg-overlay overflow-hidden">
            <div
              className="h-full rounded-full transition-all duration-300"
              style={{
                width: `${uiSnapshot.player.hp_max > 0 ? (uiSnapshot.player.hp / uiSnapshot.player.hp_max) * 100 : 0}%`,
                backgroundColor: 'var(--color-pine)',
              }}
            />
          </div>
          {uiSnapshot.player.stats.length > 0 && (
            <ul className="mt-1.5 space-y-0.5">
              {uiSnapshot.player.stats.map(stat => (
                <li key={stat.id} className="flex justify-between text-xs">
                  <span className="text-muted">{stat.id}</span>
                  <span className="text-text">{stat.value}</span>
                </li>
              ))}
            </ul>
          )}
          {uiSnapshot.player.level > 0 && (
            <div className="mt-2 border-t border-overlay pt-2">
              <div className="flex items-baseline justify-between text-xs">
                <span className="text-text font-medium">Level {uiSnapshot.player.level}</span>
                <span className="text-muted">
                  {uiSnapshot.player.xp_max > 0
                    ? `${uiSnapshot.player.xp} / ${uiSnapshot.player.xp_max} XP`
                    : 'MAX'}
                </span>
              </div>
              <div className="mt-1 h-1.5 w-full rounded-full bg-overlay overflow-hidden">
                <div
                  role="progressbar"
                  aria-valuenow={uiSnapshot.player.xp}
                  aria-valuemin={0}
                  aria-valuemax={uiSnapshot.player.xp_max || 1}
                  className="h-full rounded-full transition-all duration-300"
                  style={{
                    width: `${uiSnapshot.player.xp_max > 0 ? (uiSnapshot.player.xp / uiSnapshot.player.xp_max) * 100 : 100}%`,
                    backgroundColor: 'var(--color-gold)',
                  }}
                />
              </div>
            </div>
          )}
        </div>
      </div>
      {uiSnapshot.party.length > 0 && (
        <div>
          <p className="text-xs text-muted uppercase tracking-wider">Party</p>
          <ul className="mt-1 space-y-0.5">
            {uiSnapshot.party.map((member, i) => (
              <li key={i} className="text-pine font-medium text-xs">
                • {member.label}{member.count > 1 ? <span className="text-muted ml-1">×{member.count}</span> : null}
              </li>
            ))}
          </ul>
        </div>
      )}
      {uiSnapshot.current_room_items.length > 0 && (
        <div>
          <p className="text-xs text-muted uppercase tracking-wider">On the ground</p>
          <ul className="mt-1 space-y-0.5">
            {uiSnapshot.current_room_items.map((item, i) => (
              <li key={i} className="text-text text-xs">
                • {item.label}{item.count > 1 ? <span className="text-muted ml-1">×{item.count}</span> : null}
              </li>
            ))}
          </ul>
        </div>
      )}
      {uiSnapshot.equipped_items.length > 0 && (
        <div>
          <p className="text-xs text-muted uppercase tracking-wider">Equipped</p>
          <ul className="mt-1 space-y-0.5">
            {uiSnapshot.equipped_items.map((item, i) => (
              <li key={i} className="text-text text-xs">
                • <span className="text-muted">{item.slot}:</span> {item.label}
              </li>
            ))}
          </ul>
        </div>
      )}
      {uiSnapshot.inventory.length > 0 && (
        <div>
          <p className="text-xs text-muted uppercase tracking-wider">Inventory</p>
          <ul className="mt-1 space-y-0.5">
            {uiSnapshot.inventory.map((item, i) => (
              <li key={i} className="text-text text-xs">
                • {item.label}{item.count > 1 ? <span className="text-muted ml-1">×{item.count}</span> : null}
              </li>
            ))}
          </ul>
        </div>
      )}
      {uiSnapshot.room_consumables.length > 0 && (
        <div>
          <p className="text-xs text-muted uppercase tracking-wider">Available</p>
          {uiSnapshot.room_consumables.map((group, gi) => (
            <div key={gi} className="mt-1">
              <p className="text-text text-xs font-medium">{group.feature_label}</p>
              <ul className="space-y-0.5">
                {group.items.map((item) => (
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
        </div>
      )}
      <div>
        <p className="text-xs text-muted uppercase tracking-wider">What now?</p>
        <p className="text-text text-xs leading-relaxed">
          {uiSnapshot.objective_message || 'No current objective.'}
        </p>
      </div>
      {uiSnapshot.progress_total > 0 && (
        <div>
          <p className="text-xs text-muted uppercase tracking-wider">Progress</p>
          <div className="mt-1 h-1.5 w-full bg-overlay rounded-full overflow-hidden">
            <div
              role="progressbar"
              aria-valuenow={uiSnapshot.progress_completed}
              aria-valuemin={0}
              aria-valuemax={uiSnapshot.progress_total}
              className="h-full bg-pine rounded-full transition-all duration-500"
              style={{ width: `${(uiSnapshot.progress_completed / uiSnapshot.progress_total) * 100}%` }}
            />
          </div>
        </div>
      )}
      {uiSnapshot.secrets_total > 0 && (
        <div>
          <p className="text-xs text-muted uppercase tracking-wider">Secrets Found</p>
          <p className="text-text font-medium">{uiSnapshot.secrets_found} / {uiSnapshot.secrets_total}</p>
        </div>
      )}
    </div>
  )
}
