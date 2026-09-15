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
  'rooms', 'follow', 'language',
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
  const items = flattenItems(t)

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
