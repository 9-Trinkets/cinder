import * as api from '../api'
import { useState } from 'react'

function Section({
  title,
  defaultOpen = false,
  children,
}: {
  title: string
  defaultOpen?: boolean
  children: React.ReactNode
}) {
  const [open, setOpen] = useState(defaultOpen)
  return (
    <div className="border-b border-subtle pb-1">
      <button
        type="button"
        onClick={() => setOpen(open => !open)}
        className="flex w-full items-center justify-between py-2 text-left text-xs uppercase tracking-wider text-muted hover:text-text transition-colors cursor-pointer"
        aria-expanded={open}
      >
        <span>{title}</span>
        <span className={`transition-transform duration-200 ${open ? 'rotate-90' : ''}`}>›</span>
      </button>
      {open && <div className="pb-2 space-y-1.5">{children}</div>}
    </div>
  )
}

export default function StatusPanel({ uiSnapshot }: { uiSnapshot: api.UiSnapshot }) {
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

      {uiSnapshot.party.length > 0 && (
        <Section title="Party" defaultOpen>
          <ul className="space-y-0.5">
            {uiSnapshot.party.map((member, i) => (
              <li key={i} className="text-pine font-medium text-xs">
                • {member.label}
                {member.count > 1 ? <span className="text-muted ml-1">×{member.count}</span> : null}
                {uiSnapshot.levels_revealed ? <span className="text-muted ml-1">— Lv {member.level}</span> : null}
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
        <Section title="On the ground">
          <ul className="space-y-0.5">
            {uiSnapshot.current_room_items.map((item, i) => (
              <li key={i} className="text-text text-xs">
                • {item.label}{item.count > 1 ? <span className="text-muted ml-1">×{item.count}</span> : null}
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
