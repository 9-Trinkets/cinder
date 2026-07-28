import * as api from '../api'

const HEARTS = 5

export default function RelationshipChart({ pairs }: { pairs: api.RelationshipPair[] }) {
  if (pairs.length === 0) return null

  return (
    <div className="space-y-3">
      <p className="text-xs text-muted uppercase tracking-wider">Relationships</p>
      {pairs.map((pair, i) => {
        const filled = Math.round((pair.attraction / 10) * HEARTS)
        return (
          <div key={i} className="space-y-0.5">
            <p className="text-xs text-text truncate">
              {pair.actor_a} & {pair.actor_b}
            </p>
            <div className="flex items-center gap-1.5">
              <div className="flex gap-px text-sm leading-none select-none">
                {Array.from({ length: HEARTS }).map((_, j) => (
                  <span
                    key={j}
                    style={{ color: j < filled ? 'var(--color-love)' : 'var(--color-overlay)' }}
                  >
                    {j < filled ? '♥' : '♡'}
                  </span>
                ))}
              </div>
              <span className="text-muted text-[10px] tabular-nums">{pair.attraction}</span>
            </div>
          </div>
        )
      })}
    </div>
  )
}
