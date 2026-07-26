import * as api from '../api'

const BAR_BLOCKS = 5

function safetySuffix(safety: number): string {
  if (safety > 2) return '+'
  if (safety < -2) return '-'
  return ''
}

export default function RelationshipChart({ pairs }: { pairs: api.RelationshipPair[] }) {
  if (pairs.length === 0) return null

  return (
    <div className="space-y-3">
      <p className="text-xs text-muted uppercase tracking-wider">Relationships</p>
      {pairs.map((pair, i) => {
        const aFilled = Math.round((pair.attraction / 10) * BAR_BLOCKS)
        return (
          <div key={i} className="space-y-0.5">
            <div className="flex items-center justify-between text-xs">
              <span className="text-text truncate">
                {pair.actor_a} & {pair.actor_b}
              </span>
              <span className="text-love ml-1 shrink-0">
                {'♥'.repeat(Math.min(pair.connection, 5)) || '·'}
              </span>
            </div>
            <div className="flex items-center gap-1">
              <div className="flex gap-0.5">
                {Array.from({ length: BAR_BLOCKS }).map((_, j) => (
                  <div
                    key={j}
                    className="w-1.5 h-3 rounded-sm"
                    style={{
                      backgroundColor: j < aFilled ? 'var(--color-love)' : 'var(--color-overlay)',
                    }}
                  />
                ))}
              </div>
              <span className="text-muted text-[10px] tabular-nums">
                {pair.attraction}{safetySuffix(pair.safety)}
              </span>
            </div>
          </div>
        )
      })}
    </div>
  )
}
