import { useState } from 'react'
import Modal from './Modal'
import Button from './Button'
import Badge from './Badge'
import StatusPanel from './StatusPanel'
import type { UiSnapshot } from '../api'

type View = 'main' | 'rooms' | 'follow' | 'language'

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
  onTakeItem?: (itemId: string) => void
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
  let declared = t.shell_menu.items
    .filter(item => isKnownMenuItem(item.id))
    .map(item => ({ id: item.id, label: item.label }))

  if (declared.length === 0) {
    declared = CANONICAL_FALLBACK
      .map(entry => ({ id: entry.id, label: t[entry.labelKey as keyof typeof t] as string || entry.id }))
      .filter(i => isKnownMenuItem(i.id))
  }

  // Rooms and language are web-platform capabilities backed by the UI
  // snapshot; surface them even when a pack does not list them, so the menu
  // never collapses to just "Exit" on viewports without the sidebar. Follow
  // stays pack-authored: it is an escort-mode feature only some packs use.
  const ids = new Set(declared.map(i => i.id))
  const canonical: FlatItem[] = [
    { id: 'rooms', label: t.room_switcher_title as string || 'Rooms' },
    { id: 'language', label: t.language_menu_label as string || 'Language' },
  ].filter(item => !ids.has(item.id))

  const extras = declared.filter(i => i.id !== 'exit')
  const exit = declared.find(i => i.id === 'exit')
  return exit ? [...canonical, ...extras, exit] : [...canonical, ...extras]
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
  onTakeItem,
}: ShellMenuProps) {
  const t = ui.ui_text
  const items = flattenItems(t)

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
    onTakeItem={onTakeItem}
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
  onTakeItem?: (itemId: string) => void
}

function MainMenu({ items, t, ui, onViewChange, onClose, onExit, busy, onTakeItem }: MainMenuProps) {
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
            onClick={() => handleItemClick(child.id, onViewChange, onExit)}
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
      {/* On viewports without the sidebar the menu is the only status surface,
          so render the packed status sections (map, vitals, level, party, …)
          here; the sidebar already shows them on larger screens. */}
      <div className="lg:hidden mb-1">
        <StatusPanel uiSnapshot={ui} onTakeItem={onTakeItem} />
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
            onClick={() => handleItemClick(item.id, onViewChange, onExit)}
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
  rooms: 'rooms',
  follow: 'follow',
  language: 'language',
}

function handleItemClick(
  id: string,
  onViewChange: (v: View) => void,
  onExit: () => void,
) {
  if (id === 'exit') { onExit(); return }
  const view = VIEW_ROUTE[id]
  if (view) onViewChange(view)
}
