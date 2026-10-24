import type { ReactNode } from 'react'
import * as api from '../api'
import Minimap from './Minimap'
import Section from './Section'
import RelationshipChart from './RelationshipChart'
import { titleize } from '../utils/text'
import { formatClockLabel, useI18n } from '../i18n'

const skillKindClasses: Record<api.SkillKind, string> = {
  attack: 'border-rose/30 bg-rose/10 text-rose',
  defend: 'border-pine/30 bg-pine/10 text-pine',
  heal: 'border-foam/30 bg-foam/10 text-foam',
  spell: 'border-iris/30 bg-iris/10 text-iris',
  passive: 'border-gold/30 bg-gold/10 text-gold',
  support: 'border-crt-glow/30 bg-crt-glow/10 text-crt-glow',
}

export default function StatusPanel({
  uiSnapshot,
  onTakeItem,
  onTakeFromMember,
  onGiveToMember,
  onOpenPanel,
  hideLocation = false,
}: {
  uiSnapshot: api.UiSnapshot
  onTakeItem?: (itemId: string) => void
  onTakeFromMember?: (memberId: string, itemId: string) => void
  onGiveToMember?: (memberId: string, itemLabel: string) => void
  onOpenPanel?: (panel: string) => void
  hideLocation?: boolean
}) {
  const player = uiSnapshot.player
  const levelsRevealed = uiSnapshot.sidebar_sections.includes('level')
  const { t, locale } = useI18n()

  const orderLabel = (order: string) => {
    const known: Record<string, string> = {
      guard: t('hud.orderGuard'),
      follow: t('hud.orderFollow'),
      patrol: t('hud.orderPatrol'),
    }
    if (known[order]) return known[order]
    if (order.startsWith('deploy ') || order.startsWith('deploy:')) {
      const target = order.replace(/^deploy[:\s]+/, '').trim()
      return target ? t('hud.deployingTo', { target: titleize(target) }) : t('hud.deployed')
    }
    if (order === 'deploy') return t('hud.deployed')
    return t('hud.order', { order })
  }

  const renderers: Record<string, () => ReactNode> = {
    location: () => (
      <Section title={t('hud.location')} defaultOpen>
        <p className="text-text font-medium">{uiSnapshot.current_room_name}</p>
        {uiSnapshot.show_clock !== false && (
          <p className="text-text text-xs">
            {t('hud.day', { day: uiSnapshot.day_number })}
            {uiSnapshot.time_label ? <span className="text-muted ml-1">— {formatClockLabel(uiSnapshot.time_label, locale)}</span> : null}
          </p>
        )}
      </Section>
    ),

    map: () =>
      uiSnapshot.minimap ? (
        <Section title={uiSnapshot.ui_text.minimap_sidebar_label || t('hud.map')} defaultOpen>
          <Minimap map={uiSnapshot.minimap} uiText={uiSnapshot.ui_text} />
        </Section>
      ) : null,

    vitals: () => (
      <Section title={t('hud.vitals')} defaultOpen>
        <div className="flex items-baseline justify-between text-xs">
          <span className="text-text">{t('hud.hp')}</span>
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
        {player.mp !== undefined && player.mp_max !== undefined && (
          <>
            <div className="flex items-baseline justify-between text-xs mt-1.5">
              <span className="text-text">{t('hud.mp')}</span>
              <span className="text-muted">{player.mp}/{player.mp_max}</span>
            </div>
            <div className="h-1.5 w-full rounded-full bg-overlay overflow-hidden">
              <div
                className="h-full rounded-full transition-all duration-300"
                style={{
                  width: `${player.mp_max > 0 ? (player.mp / player.mp_max) * 100 : 0}%`,
                  backgroundColor: 'var(--color-iris)',
                }}
              />
            </div>
          </>
        )}
        {(player.attack !== undefined || player.defense !== undefined) && (
          <div className="grid grid-cols-2 gap-2 text-xs pt-1.5 pb-1 border-t border-overlay/40">
            {player.attack !== undefined && (
              <div className="flex justify-between">
                <span className="text-muted">{t('hud.attack')}</span>
                <span className="text-text font-medium">{player.attack}</span>
              </div>
            )}
            {player.defense !== undefined && (
              <div className="flex justify-between">
                <span className="text-muted">{t('hud.defense')}</span>
                <span className="text-text font-medium">{player.defense}</span>
              </div>
            )}
          </div>
        )}
        {player.stats.length > 0 && (
          <ul className="space-y-0.5 pt-1 border-t border-overlay/30">
            {player.stats.map(stat => (
              <li key={stat.id} className="flex justify-between text-xs">
                <span className="text-muted">{stat.id}</span>
                <span className="text-text">{stat.value}</span>
              </li>
            ))}
          </ul>
        )}
      </Section>
    ),

    level: () => (
      <Section title={t('hud.level')} defaultOpen>
        <div className="flex items-baseline justify-between text-xs">
          <span className="text-text font-medium">{t('hud.levelN', { n: player.level })}</span>
          <span className="text-muted">
            {player.xp_max > 0 ? t('hud.xpOfMax', { current: player.xp, max: player.xp_max }) : t('hud.max')}
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
    ),

    skills: () =>
      uiSnapshot.skills && uiSnapshot.skills.length > 0 ? (
        <Section title={t('hud.skills')} defaultOpen>
          <ul className="space-y-2">
            {uiSnapshot.skills.map(skill => {
              const isMax = skill.level >= 10
              const spSpan = skill.sp_level_max - skill.sp_level_min
              const spProgress = isMax
                ? 100
                : spSpan > 0
                  ? Math.min(100, Math.max(0, ((skill.sp - skill.sp_level_min) / spSpan) * 100))
                  : 0
              return (
                <li key={skill.id} className="rounded-lg border border-subtle bg-surface/40 p-2 text-xs">
                  <div className="flex items-baseline justify-between">
                    <span className="font-medium text-text">
                      {skill.label}
                      <span className="ml-1.5 text-[10px] font-mono text-pine">{t('hud.lv', { n: skill.level })}</span>
                    </span>
                    <span className="text-[10px] tabular-nums text-muted font-mono">
                      {isMax ? t('hud.max') : t('hud.spOfMax', { sp: skill.sp, max: skill.sp_level_max })}
                    </span>
                  </div>
                  <div className="mt-1 h-1.5 w-full rounded-full bg-overlay overflow-hidden">
                    <div
                      role="progressbar"
                      aria-valuenow={skill.sp}
                      aria-valuemin={skill.sp_level_min}
                      aria-valuemax={skill.sp_level_max}
                      className="h-full rounded-full transition-all duration-300"
                      style={{
                        width: `${spProgress}%`,
                        backgroundColor: 'var(--color-iris)',
                      }}
                    />
                  </div>
                  {skill.scalar_summary && (
                    <div className="mt-1 text-[10px] text-muted">
                      {skill.scalar_summary}
                    </div>
                  )}
                </li>
              )
            })}
          </ul>
        </Section>
      ) : null,

    party: () =>
      uiSnapshot.party.length > 0 ? (
        <Section title={t('hud.party')} defaultOpen>
          <ul className="space-y-2">
            {uiSnapshot.party.map(member => (
              <li key={member.id} className="rounded-lg border border-subtle bg-surface/40 p-2.5 transition duration-200">
                <button
                  type="button"
                  onClick={() => onOpenPanel?.(member.order_panel)}
                  className="group w-full text-left transition duration-200 hover:text-pine cursor-pointer"
                >
                  <span className="flex items-start justify-between gap-2">
                    <span className="min-w-0">
                      <span className="block truncate text-xs font-medium text-text">{member.label}</span>
                      <span className="mt-0.5 block text-[10px] uppercase tracking-[0.14em] text-pine">
                        {member.order ? orderLabel(member.order) : t('hud.noOrder')}
                        {levelsRevealed ? ` · ${t('hud.lv', { n: member.level })}` : ''}
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

                {member.skills && member.skills.length > 0 && (
                  <ul
                    aria-label={t('hud.memberSkills', { name: member.label })}
                    className="mt-2 flex flex-wrap gap-1"
                  >
                    {member.skills.map(skill => (
                      <li
                        key={skill.id}
                        title={skill.kind ? `${titleize(skill.kind)} ${t('hud.skill')}` : t('hud.skill')}
                        className={`max-w-full truncate rounded-full border px-1.5 py-0.5 text-[9px] font-medium leading-none ${
                          skill.kind
                            ? skillKindClasses[skill.kind]
                            : 'border-subtle bg-overlay/60 text-muted'
                        }`}
                      >
                        {skill.label}
                        {levelsRevealed && skill.level !== undefined
                          ? ` ${t('hud.lv', { n: skill.level })}`
                          : ''}
                      </li>
                    ))}
                  </ul>
                )}

                {member.equipped_items && member.equipped_items.length > 0 && (
                  <div className="mt-2 pt-2 border-t border-subtle/50 text-[11px]">
                    <div className="text-[10px] uppercase tracking-wider text-muted font-mono mb-1">{t('hud.equipped')}</div>
                    <ul className="space-y-0.5 pl-1">
                      {member.equipped_items.map((item, i) => (
                        <li key={i} className="text-text">
                          <span className="text-muted">{item.slot}:</span> {titleize(item.label)}
                        </li>
                      ))}
                    </ul>
                  </div>
                )}

                {member.inventory && member.inventory.length > 0 && (
                  <div className="mt-2 pt-1.5 border-t border-subtle/50 text-[11px]">
                    <div className="text-[10px] uppercase tracking-wider text-muted font-mono mb-1">{t('hud.carrying')}</div>
                    <ul className="space-y-0.5 pl-1">
                      {member.inventory.map((item, i) => (
                        <li key={i} className="text-text">
                          • {titleize(item.label)}{item.count > 1 ? <span className="text-muted ml-1">×{item.count}</span> : null}
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
              </li>
            ))}
          </ul>
        </Section>
      ) : null,

    equipped: () =>
      uiSnapshot.equipped_items.length > 0 ? (
        <Section title={t('hud.equipped')}>
          <ul className="space-y-0.5">
            {uiSnapshot.equipped_items.map((item, i) => (
              <li key={i} className="text-text text-xs">
                • <span className="text-muted">{item.slot}:</span> {item.label}
              </li>
            ))}
          </ul>
        </Section>
      ) : null,

    inventory: () =>
      uiSnapshot.inventory.length > 0 ? (
        <Section title={uiSnapshot.ui_text.look_group_items || t('hud.inventory')}>
          <ul className="space-y-0.5">
            {uiSnapshot.inventory.map((item, i) => (
              <li key={i} className="text-text text-xs">
                • {titleize(item.label)}
                {item.count > 1 ? <span className="text-muted ml-1">×{item.count}</span> : null}
              </li>
            ))}
          </ul>
        </Section>
      ) : null,

    room_items: () =>
      uiSnapshot.current_room_items.length > 0 ? (
        <Section title={uiSnapshot.ui_text.room_items_sidebar_label || t('hud.onTheGround')}>
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
                      {t('hud.takeItem', { item: titleize(item.label) })}
                      {item.count > 1 ? <span className="text-muted ml-1">×{item.count}</span> : null}
                    </button>
                  )
                  : (
                    <span>
                      • {titleize(item.label)}
                      {item.count > 1 ? <span className="text-muted ml-1">×{item.count}</span> : null}
                    </span>
                  )}
              </li>
            ))}
          </ul>
        </Section>
      ) : null,

    room_consumables: () =>
      uiSnapshot.room_consumables.length > 0 ? (
        <Section title={t('hud.available')}>
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
      ) : null,

    secrets: () =>
      uiSnapshot.secrets_total > 0 ? (
        <Section title={t('hud.secretsFound')}>
          <p className="text-text font-medium">{uiSnapshot.secrets_found} / {uiSnapshot.secrets_total}</p>
        </Section>
      ) : null,

    quests: () => (
      <Section title={uiSnapshot.ui_text.quests_sidebar_label || t('hud.quests')} defaultOpen>
        {uiSnapshot.objectives && uiSnapshot.objectives.length > 0 ? (
          <div className="space-y-2.5">
            {uiSnapshot.objectives.map((obj, idx) => {
              const isMain = obj.quest_kind?.toLowerCase() === 'main'
              const isSide = obj.quest_kind?.toLowerCase() === 'side'
              const hasQuestCard = Boolean(obj.quest_title || obj.quest_kind)

              if (hasQuestCard) {
                return (
                  <div key={obj.stage_id || idx} className="rounded-lg bg-overlay/60 p-2.5 border border-subtle">
                    <div className="flex items-center justify-between gap-2 mb-1">
                      <span className="font-semibold text-text text-xs">
                        {obj.quest_title || (isMain ? t('hud.mainQuest') : t('hud.sideQuest'))}
                      </span>
                      {obj.quest_kind && (
                        <span className={`text-[10px] px-1.5 py-0.5 rounded font-semibold uppercase tracking-wider ${
                          isMain
                            ? 'bg-gold/15 text-gold border border-gold/30'
                            : isSide
                            ? 'bg-iris/15 text-iris border border-iris/30'
                            : 'bg-pine/15 text-pine border border-pine/30'
                        }`}>
                          {obj.quest_kind}
                        </span>
                      )}
                    </div>
                    <p className="text-text font-medium text-xs mb-0.5">{obj.summary}</p>
                    {obj.message && obj.message !== obj.summary && (
                      <p className="text-muted text-[11px] leading-relaxed">{obj.message}</p>
                    )}
                  </div>
                )
              }

              return (
                <div key={obj.stage_id || idx} className="rounded-lg bg-overlay/40 p-2 border border-subtle/60">
                  <p className="text-text text-xs leading-relaxed">{obj.summary}</p>
                  {obj.message && obj.message !== obj.summary && (
                    <p className="text-muted text-[11px] leading-relaxed mt-0.5">{obj.message}</p>
                  )}
                </div>
              )
            })}
          </div>
        ) : (
          <p className="text-text text-xs leading-relaxed">
            {uiSnapshot.objective_message || t('hud.noActiveQuests')}
          </p>
        )}
      </Section>
    ),

    relationships: () =>
      uiSnapshot.relationship_pairs.length > 0 ? (
        <RelationshipChart pairs={uiSnapshot.relationship_pairs} />
      ) : null,
  }

  return (
    <div>
      {uiSnapshot.sidebar_sections.map(id => {
        if (id === 'location' && hideLocation) return null
        const render = renderers[id]
        return render ? <div key={id}>{render()}</div> : null
      })}
    </div>
  )
}
