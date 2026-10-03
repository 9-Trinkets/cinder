import { useState } from 'react'
import Modal from './Modal'
import type { UiSnapshot } from '../api'
import type { MenuView } from '../hooks/playUtils'

interface ShellMenuProps {
  ui: UiSnapshot
  view: MenuView
  onViewChange: (v: MenuView) => void
  onClose: () => void
  onSwitchRoom: (roomId: string) => void
  onFollowActor: (actorId: string | null) => void
  onChangeLocale: (locale: string) => void
  onExit: () => void
  busy: boolean
}

interface FlatItem {
  id: string
  label: string
}

const CANONICAL_FALLBACK: { id: string; labelKey: string }[] = [
  { id: 'language', labelKey: 'language_menu_label' },
  { id: 'exit', labelKey: 'exit_label' },
]

const KNOWN_IDS = new Set([
  'rooms', 'follow', 'quests', 'language',
])

function isKnownMenuItem(id: string): boolean {
  return id === 'exit' || KNOWN_IDS.has(id) || !!VIEW_ROUTE[id]
}

function flattenItems(t: UiSnapshot['ui_text']): FlatItem[] {
  if (t.shell_menu.items.length > 0) {
    return t.shell_menu.items
      .filter(item => isKnownMenuItem(item.id))
      .map(item => ({ id: item.id, label: item.label }))
  }

  return CANONICAL_FALLBACK
    .map(entry => ({ id: entry.id, label: t[entry.labelKey as keyof typeof t] as string || entry.id }))
    .filter(i => isKnownMenuItem(i.id))
}

export default function ShellMenu({
  ui,
  view,
  onViewChange,
  onClose,
  onSwitchRoom,
  onFollowActor,
  onChangeLocale,
  onExit,
  busy,
}: ShellMenuProps) {
  const t = ui.ui_text
  const items = flattenItems(t).filter(item => item.id !== 'quests' || ui.quests_revealed !== false)

  if (view === 'rooms') {
    return (
      <Modal title={t.room_switcher_title || 'Fast Travel'} onClose={onClose}>
        <MenuBackButton onClick={() => onViewChange('main')} />
        <div className="divide-y divide-subtle/40 border-t border-b border-subtle/40">
          {ui.rooms.map((r) => (
            <button
              key={r.id}
              type="button"
              className="w-full py-3 px-1 flex items-center justify-between group hover:text-foam text-left cursor-pointer transition-colors disabled:opacity-50"
              onClick={() => onSwitchRoom(r.id)}
              disabled={busy}
            >
              <div>
                <span className="text-sm font-medium text-text group-hover:text-foam transition-colors block">{r.title}</span>
                {r.menu_text && <span className="text-xs text-muted">{r.menu_text}</span>}
              </div>
              <span className="text-xs text-foam opacity-0 group-hover:opacity-100 transition-opacity">Travel &rsaquo;</span>
            </button>
          ))}
        </div>
      </Modal>
    )
  }

  if (view === 'follow') {
    return (
      <Modal title={t.follow_actor_title || 'Companions'} onClose={onClose}>
        <MenuBackButton onClick={() => onViewChange('main')} />
        <div className="divide-y divide-subtle/40 border-t border-b border-subtle/40">
          {ui.follow_options.map((a) => {
            const isFollowing = (a.id === 'none' && !ui.followed_actor_name) ||
              (a.title === ui.followed_actor_name)
            return (
              <button
                key={a.id}
                type="button"
                className="w-full py-3 px-1 flex items-center justify-between group hover:text-foam text-left cursor-pointer transition-colors disabled:opacity-50"
                onClick={() => onFollowActor(a.id === 'none' ? null : a.id)}
                disabled={busy}
              >
                <div>
                  <span className="text-sm font-medium text-text group-hover:text-foam transition-colors block">{a.title}</span>
                  {a.menu_text && <span className="text-xs text-muted">{a.menu_text}</span>}
                </div>
                {isFollowing && (
                  <span className="text-xs font-mono uppercase tracking-wider text-foam bg-pine/15 px-2 py-0.5 rounded border border-pine/30">
                    Active
                  </span>
                )}
              </button>
            )
          })}
        </div>
      </Modal>
    )
  }

  if (view === 'language') {
    return (
      <Modal title={t.language_modal_title || 'Language'} onClose={onClose}>
        <MenuBackButton onClick={() => onViewChange('main')} />
        <div className="divide-y divide-subtle/40 border-t border-b border-subtle/40">
          {ui.locale_options.map((l) => (
            <button
              key={l.code}
              type="button"
              className={`w-full py-3 px-1 flex items-center justify-between group text-left cursor-pointer transition-colors ${
                l.code === ui.current_locale ? 'text-foam font-semibold' : 'text-text hover:text-foam'
              }`}
              onClick={() => onChangeLocale(l.code)}
              disabled={busy || l.code === ui.current_locale}
            >
              <span className="text-sm">{l.label}</span>
              {l.code === ui.current_locale && (
                <span className="text-xs font-mono uppercase tracking-wider text-foam bg-pine/15 px-2 py-0.5 rounded border border-pine/30">
                  Active
                </span>
              )}
            </button>
          ))}
        </div>
      </Modal>
    )
  }

  if (view === 'quests') {
    const groups = [
      { status: 'active', label: (t.quests_active_label as string) || 'Active' },
      { status: 'completed', label: (t.quests_completed_label as string) || 'Completed' },
      { status: 'failed', label: (t.quests_failed_label as string) || 'Failed' },
    ] as const
    return (
      <Modal title={(t.quests_menu_title as string) || 'Quest Log'} onClose={onClose}>
        <MenuBackButton onClick={() => onViewChange('main')} />
        {ui.quests.length > 0 ? (
          <div className="space-y-5">
            {groups.map(group => {
              const quests = ui.quests.filter(quest => quest.status === group.status)
              if (quests.length === 0) return null
              return (
                <section key={group.status} aria-labelledby={`quests-${group.status}`}>
                  <div className="mb-2 flex items-center gap-2">
                    <h3
                      id={`quests-${group.status}`}
                      className={`text-[10px] font-mono font-semibold uppercase tracking-[0.2em] ${
                        group.status === 'active'
                          ? 'text-foam'
                          : group.status === 'completed'
                          ? 'text-pine'
                          : 'text-love'
                      }`}
                    >
                      {group.label}
                    </h3>
                    <span className="h-px flex-1 bg-subtle/60" />
                    <span className="text-[10px] tabular-nums text-muted">{quests.length}</span>
                  </div>
                  <div className="space-y-2">
                    {quests.map(quest => (
                      <article
                        key={quest.quest_id}
                        className={`rounded-lg border p-3 ${
                          quest.status === 'active'
                            ? 'border-foam/25 bg-foam/5'
                            : quest.status === 'completed'
                            ? 'border-pine/20 bg-pine/5'
                            : 'border-love/25 bg-love/5'
                        }`}
                      >
                        <div className="flex items-start justify-between gap-3">
                          <h4 className="text-sm font-semibold leading-tight text-text">{quest.title}</h4>
                          <span className="shrink-0 text-[9px] font-mono uppercase tracking-[0.14em] text-muted">
                            {quest.kind}
                          </span>
                        </div>
                        {quest.summary && (
                          <p className="mt-1.5 text-xs leading-relaxed text-text/90">{quest.summary}</p>
                        )}
                        {quest.message && quest.message !== quest.summary && (
                          <p className="mt-1 text-[11px] leading-relaxed text-muted">{quest.message}</p>
                        )}
                      </article>
                    ))}
                  </div>
                </section>
              )
            })}
          </div>
        ) : (
          <p className="border-y border-subtle/40 py-5 text-center text-sm text-muted">
            {(t.quests_empty as string) || 'No quests have been recorded yet.'}
          </p>
        )}
      </Modal>
    )
  }

  return (
    <MainMenu
      items={items}
      t={t}
      ui={ui}
      onViewChange={onViewChange}
      onClose={onClose}
      onExit={onExit}
      busy={busy}
    />
  )
}

interface MainMenuProps {
  items: FlatItem[]
  t: UiSnapshot['ui_text']
  ui: UiSnapshot
  onViewChange: (v: MenuView) => void
  onClose: () => void
  onExit: () => void
  busy: boolean
}

function MainMenu({
  items,
  t,
  ui,
  onViewChange,
  onClose,
  onExit,
  busy,
}: MainMenuProps) {
  const [submenu, setSubmenu] = useState<{ id: string; label: string }[] | null>(null)
  const [submenuTitle, setSubmenuTitle] = useState('')

  if (submenu !== null) {
    return (
      <Modal title={submenuTitle} onClose={onClose}>
        <MenuBackButton onClick={() => { setSubmenu(null); setSubmenuTitle('') }} />
        <div className="divide-y divide-subtle/40 border-t border-b border-subtle/40">
          {submenu.map((child) => (
            <button
              key={child.id}
              type="button"
              className="w-full py-3 px-1 flex items-center justify-between group hover:text-foam text-left cursor-pointer transition-colors"
              onClick={() => handleItemClick(child.id, onViewChange, onExit)}
            >
              <span className="text-sm font-medium text-text group-hover:text-foam transition-colors">{child.label}</span>
              <span className="text-muted group-hover:text-foam group-hover:translate-x-1 transition-transform">&rsaquo;</span>
            </button>
          ))}
        </div>
      </Modal>
    )
  }

  return (
    <Modal title={t.shell_menu_title || 'System Menu'} onClose={onClose}>
      <div className="divide-y divide-subtle/40 border-t border-b border-subtle/40 my-1">
        {items.map((item) => {
          const packItem = t.shell_menu.items.find(i => i.id === item.id)
          const hasChildren = packItem?.children && packItem.children.length > 0

          if (item.id === 'exit') {
            return (
              <button
                key={item.id}
                type="button"
                onClick={onExit}
                className="w-full py-3.5 px-1 flex items-center justify-between group text-left cursor-pointer transition-colors"
              >
                <div>
                  <span className="text-sm font-medium text-text group-hover:text-love transition-colors block">
                    {item.label}
                  </span>
                  <span className="text-xs text-muted">
                    Bookmark progress and return to library
                  </span>
                </div>
                <span className="text-muted group-hover:text-love group-hover:translate-x-1 transition-transform">
                  &rsaquo;
                </span>
              </button>
            )
          }

          let subtitle = ''
          if (item.id === 'rooms') subtitle = 'Fast travel to discovered chambers'
          else if (item.id === 'follow') subtitle = ui.followed_actor_name ? `Accompanying ${ui.followed_actor_name}` : 'Travel unaccompanied'
          else if (item.id === 'quests') {
            const activeCount = ui.quests.filter(quest => quest.status === 'active').length
            subtitle = activeCount === 1 ? '1 active quest' : `${activeCount} active quests`
          }
          else if (item.id === 'language') subtitle = ui.locale_options.find(l => l.code === ui.current_locale)?.label || ui.current_locale

          if (hasChildren) {
            const children = packItem!.children!
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => {
                  setSubmenu(children)
                  setSubmenuTitle(item.label)
                }}
                className="w-full py-3.5 px-1 flex items-center justify-between group hover:text-foam text-left cursor-pointer transition-colors"
              >
                <div>
                  <span className="text-sm font-medium text-text group-hover:text-foam transition-colors block">
                    {item.label}
                  </span>
                  {subtitle && <span className="text-xs text-muted">{subtitle}</span>}
                </div>
                <span className="text-muted group-hover:text-foam group-hover:translate-x-1 transition-transform">
                  &rsaquo;
                </span>
              </button>
            )
          }

          return (
            <button
              key={item.id}
              type="button"
              onClick={() => handleItemClick(item.id, onViewChange, onExit)}
              disabled={busy}
              className="w-full py-3.5 px-1 flex items-center justify-between group hover:text-foam text-left cursor-pointer transition-colors disabled:opacity-50"
            >
              <div>
                <span className="text-sm font-medium text-text group-hover:text-foam transition-colors block">
                  {item.label}
                </span>
                {subtitle && <span className="text-xs text-muted">{subtitle}</span>}
              </div>
              <span className="text-muted group-hover:text-foam group-hover:translate-x-1 transition-transform">
                &rsaquo;
              </span>
            </button>
          )
        })}
      </div>
    </Modal>
  )
}

function MenuBackButton({ onClick }: { onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="text-xs font-mono uppercase tracking-widest text-muted hover:text-text cursor-pointer flex items-center gap-1.5 mb-3 transition-colors"
    >
      &lsaquo; Back
    </button>
  )
}

const VIEW_ROUTE: Record<string, MenuView> = {
  rooms: 'rooms',
  follow: 'follow',
  quests: 'quests',
  language: 'language',
}

function handleItemClick(
  id: string,
  onViewChange: (v: MenuView) => void,
  onExit: () => void,
) {
  if (id === 'exit') { onExit(); return }
  const view = VIEW_ROUTE[id]
  if (view) onViewChange(view)
}
