import * as api from '../api'
import Section from './Section'

const HEARTS = 5

export default function RelationshipChart({ pairs }: { pairs: api.RelationshipPair[] }) {
  if (pairs.length === 0) return null

  return (
    <Section title="Relationships" defaultOpen>
      {pairs.map((pair, i) => {
        const filled = Math.round((pair.attraction / 10) * HEARTS)
        return (
          <div key={i} className="space-y-0.5">
            <p className="text-xs text-text truncate">
              {pair.actor_a} & {pair.actor_b}
            </p>
            <div className="flex gap-1 text-sm leading-none select-none">
              {Array.from({ length: HEARTS }).map((_, j) => (
                <span
                  key={j}
                  style={{ color: j < filled ? 'var(--color-love)' : 'var(--color-overlay)' }}
                >
                  {j < filled ? '♥' : '♡'}
                </span>
              ))}
            </div>
          </div>
        )
      })}
    </Section>
  )
}
