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
      {uiSnapshot.followed_actor_name && (
        <div>
          <p className="text-xs text-muted uppercase tracking-wider">Following</p>
          <p className="text-pine font-medium">{uiSnapshot.followed_actor_name}</p>
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
