import { useState } from 'react'
import Modal from './Modal'
import Button from './Button'
import Badge from './Badge'
import type { UiSnapshot } from '../api'

type View = 'main' | 'about' | 'rooms' | 'follow' | 'language'

interface ShellMenuProps {
  ui: UiSnapshot
  view: View
  onViewChange: (v: View) => void
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

const CANONICAL_ORDER: string[] = [
  'resume',
  'rooms', 'follow', 'language', 'about', 'exit',
]

const CANONICAL_FALLBACK: { id: string; labelKey: string }[] = [
  { id: 'resume', labelKey: 'resume_label' },
  { id: 'language', labelKey: 'language_menu_label' },
  { id: 'about', labelKey: 'about_label' },
  { id: 'exit', labelKey: 'exit_label' },
]

const KNOWN_IDS = new Set([
  'about',
  'rooms', 'follow', 'language',
])

function isKnownMenuItem(id: string): boolean {
  return id === 'resume' || id === 'exit' || KNOWN_IDS.has(id) || !!VIEW_ROUTE[id]
}

function flattenItems(t: UiSnapshot['ui_text']): FlatItem[] {
  if (t.shell_menu.items.length > 0) {
    return t.shell_menu.items
      .filter(item => isKnownMenuItem(item.id))
      .map(item => ({ id: item.id, label: item.label }))
  }

  const out: FlatItem[] = []
  for (const entry of CANONICAL_FALLBACK) {
    out.push({ id: entry.id, label: t[entry.labelKey as keyof typeof t] as string || entry.id })
  }
  return out.filter(i => isKnownMenuItem(i.id))
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

  if (view === 'about') {
    return (
      <Modal title={t.about_label} onClose={onClose}>
        <MenuBackButton onClick={() => onViewChange('main')} />
        <p className="whitespace-pre-wrap leading-relaxed text-sm">{ui.about_body}</p>
      </Modal>
    )
  }

  if (view === 'rooms') {
    return (
      <Modal title={t.room_switcher_title} onClose={onClose}>
        <MenuBackButton onClick={() => onViewChange('main')} />
        {ui.rooms.map((r) => (
          <Button
            key={r.id}
            variant="secondary"
            className="block w-full text-left"
            onClick={() => onSwitchRoom(r.id)}
            disabled={busy}
          >
            <span className="font-medium">{r.title}</span>
            {r.menu_text && <span className="text-muted text-xs ml-2">{r.menu_text}</span>}
          </Button>
        ))}
      </Modal>
    )
  }

  if (view === 'follow') {
    return (
      <Modal title={t.follow_actor_title} onClose={onClose}>
        <MenuBackButton onClick={() => onViewChange('main')} />
        {ui.follow_options.map((a) => (
          <Button
            key={a.id}
            variant="secondary"
            className="block w-full text-left"
            onClick={() => onFollowActor(a.id === 'none' ? null : a.id)}
            disabled={busy}
          >
            <span className="font-medium">{a.title}</span>
            {a.menu_text && <span className="text-muted text-xs ml-2">{a.menu_text}</span>}
          </Button>
        ))}
      </Modal>
    )
  }

  if (view === 'language') {
    return (
      <Modal title={t.language_modal_title} onClose={onClose}>
        <MenuBackButton onClick={() => onViewChange('main')} />
        {ui.locale_options.map((l) => (
          <Button
            key={l.code}
            variant="secondary"
            className={`block w-full text-left ${l.code === ui.current_locale ? '!bg-pine/20 !border-pine' : ''}`}
            onClick={() => onChangeLocale(l.code)}
            disabled={busy || l.code === ui.current_locale}
          >
            <span className="font-medium">{l.label}</span>
            {l.code === ui.current_locale && <span className="text-pine text-xs ml-2">(current)</span>}
          </Button>
        ))}
      </Modal>
    )
  }

  return <MainMenu
    items={items}
    t={t}
    ui={ui}
    onViewChange={onViewChange}
    onClose={onClose}
    onExit={onExit}
    busy={busy}
  />
}

interface MainMenuProps {
  items: FlatItem[]
  t: UiSnapshot['ui_text']
  ui: UiSnapshot
  onViewChange: (v: View) => void
  onClose: () => void
  onExit: () => void
  busy: boolean
}

function MainMenu({ items, t, ui, onViewChange, onClose, onExit, busy }: MainMenuProps) {
  const [submenu, setSubmenu] = useState<{ id: string; label: string }[] | null>(null)
  const [submenuTitle, setSubmenuTitle] = useState('')

  if (submenu !== null) {
    return (
      <Modal title={submenuTitle} onClose={onClose}>
        <MenuBackButton onClick={() => { setSubmenu(null); setSubmenuTitle('') }} />
        {submenu.map((child) => (
          <Button
            key={child.id}
            variant="secondary"
            className="block w-full text-left"
            onClick={() => handleItemClick(child.id, onViewChange, onClose, onExit)}
          >
            {child.label}
          </Button>
        ))}
      </Modal>
    )
  }

  return (
    <Modal title={t.shell_menu_title} onClose={onClose}>
      <div className="rounded-lg border border-subtle bg-base/30 px-3 py-3 text-xs text-muted">
        <div className="flex flex-wrap gap-2">
          <Badge>{ui.current_room_name}</Badge>
          <Badge>
            Day {ui.day_number}{ui.time_label ? ` — ${ui.time_label}` : ''}
          </Badge>
          {ui.followed_actor_name && (
            <Badge color="success">
              Following {ui.followed_actor_name}
            </Badge>
          )}
        </div>
      </div>
      {items.map((item) => {
        const packItem = t.shell_menu.items.find(i => i.id === item.id)
        const hasChildren = packItem?.children && packItem.children.length > 0

        if (item.id === 'exit') {
          return (
            <div key={item.id}>
              <hr className="border-subtle my-2" />
              <Button
                variant="secondary"
                className="block w-full text-left"
                onClick={onExit}
              >
                {item.label}
              </Button>
            </div>
          )
        }

        if (item.id === 'resume') {
          return (
            <Button
              key={item.id}
              variant="secondary"
              className="block w-full text-left"
              onClick={onClose}
            >
              {item.label}
            </Button>
          )
        }

        if (hasChildren) {
          const children = packItem!.children!
          return (
            <Button
              key={item.id}
              variant="secondary"
              className="block w-full text-left"
              onClick={() => {
                setSubmenu(children)
                setSubmenuTitle(item.label)
              }}
            >
              {item.label} &rarr;
            </Button>
          )
        }

        return (
          <Button
            key={item.id}
            variant="secondary"
            className="block w-full text-left"
            onClick={() => handleItemClick(item.id, onViewChange, onClose, onExit)}
            disabled={busy}
          >
            {item.label}
          </Button>
        )
      })}
    </Modal>
  )
}

function MenuBackButton({ onClick }: { onClick: () => void }) {
  return (
    <Button
      variant="secondary"
      className="block w-full text-left mb-2"
      onClick={onClick}
    >
      &larr; Back
    </Button>
  )
}

const VIEW_ROUTE: Record<string, View> = {
  about: 'about',
  rooms: 'rooms',
  follow: 'follow',
  language: 'language',
}

function handleItemClick(
  id: string,
  onViewChange: (v: View) => void,
  onClose: () => void,
  onExit: () => void,
) {
  if (id === 'exit') { onExit(); return }
  if (id === 'resume') { onClose(); return }
  const view = VIEW_ROUTE[id]
  if (view) onViewChange(view)
}
