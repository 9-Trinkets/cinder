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
      {uiSnapshot.party.length > 0 && (
        <div>
          <p className="text-xs text-muted uppercase tracking-wider">Party</p>
          <ul className="mt-1 space-y-0.5">
            {uiSnapshot.party.map((name, i) => (
              <li key={i} className="text-pine font-medium text-xs">• {name}</li>
            ))}
          </ul>
        </div>
      )}
      {uiSnapshot.current_room_tags.length > 0 && (
        <div>
          <p className="text-xs text-muted uppercase tracking-wider">Markers here</p>
          <ul className="mt-1 space-y-0.5">
            {uiSnapshot.current_room_tags.map((label, i) => (
              <li key={i} className="text-text text-xs">• {label}</li>
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
